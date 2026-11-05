//! Global search: one SQLite FTS5 index, in memory, over search documents. Every source of searchable content (the
//! switched-on packs and the active campaigns' characters now; plugins later) adds `Doc`s; the index doesn't know
//! where they came from.

use crate::commands::campaigns::{self, active_dir};
use crate::commands::catalog::{also, description_text, KINDS};
use crate::commands::characters::{self, class_line};
use crate::commands::packs::{disabled, enabled, loaded};
use crate::packs::Pack;
use crate::storage::{blocking, StorageError};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::AppHandle;

/// A search result: what it is (`kind`) and where it lives. The frontend turns `kind` into the place it opens.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub kind: String,
    /// The pack's id; empty for what isn't from a pack (a character).
    pub pack: String,
    /// Where it comes from, to show: the pack's or the campaign's name.
    pub source: String,
    pub index: String,
    pub name: String,
    /// A rule section's chapter, to open it in the Rules tab.
    pub chapter_index: Option<String>,
    pub chapter_name: Option<String>,
    /// The campaign it belongs to; none for what every campaign shares (pack content).
    pub campaign: Option<String>,
}

/// One searchable thing: its result, and the text searched besides its name.
pub struct Doc {
    pub hit: Hit,
    pub body: String,
}

/// The index, with what it was built for: the switched-off packs and the `CHANGES` count. It's rebuilt when those change.
type BuiltFor = (Vec<String>, u64);
static INDEX: Mutex<Option<(BuiltFor, Connection)>> = Mutex::new(None);
/// Counts changes to searchable campaign data (characters, campaign names and folders).
static CHANGES: AtomicU64 = AtomicU64::new(0);

/// Call after a change to searchable campaign data: the next search rebuilds the index.
pub fn changed() {
    CHANGES.fetch_add(1, Ordering::SeqCst);
}

fn sql_error(e: rusqlite::Error) -> StorageError {
    StorageError::Io(format!("Search index: {e}"))
}

/// The packs' Catalog entries and the rule sections a chapter lists (one that none lists can't be opened).
pub fn pack_docs(packs: &[&Pack]) -> Vec<Doc> {
    let kinds = KINDS.iter().copied().chain(["rule-sections"]);
    kinds
        .flat_map(|kind| packs.iter().flat_map(move |p| p.entries.get(kind).into_iter().flatten().map(move |e| (*p, kind, e))))
        .map(|(pack, kind, entry)| {
            let a = also(pack, kind, entry);
            let hit = Hit { kind: a.kind, pack: a.pack, source: a.source, index: a.index, name: a.name, chapter_index: a.chapter_index, chapter_name: a.chapter_name, campaign: None };
            Doc { hit, body: description_text(entry) }
        })
        .filter(|d| d.hit.kind != "rule-sections" || d.hit.chapter_index.is_some())
        .collect()
}

/// The active campaigns' characters, found by name, player or class.
// shortcut: a campaign or characters folder that can't be read is left out of search (its own screens show the error); report it here if that confuses users.
fn character_docs(dirs: &campaigns::Dirs) -> Vec<Doc> {
    let Ok(all) = campaigns::list(dirs) else { return Vec::new() };
    all.campaigns
        .into_iter()
        .filter_map(|campaign| Some((characters::list(&active_dir(dirs, &campaign.id).ok()?).ok()?, campaign)))
        .flat_map(|(list, campaign)| {
            list.characters.into_iter().map(move |c| Doc {
                body: format!("{} {}", c.player, class_line(&c)),
                hit: Hit {
                    kind: "characters".into(),
                    pack: String::new(),
                    source: campaign.name.clone(),
                    index: c.id,
                    name: c.name,
                    chapter_index: None,
                    chapter_name: None,
                    campaign: Some(campaign.id.clone()),
                },
            })
        })
        .collect()
}

pub fn build(docs: &[Doc]) -> Result<Connection, StorageError> {
    let db = Connection::open_in_memory().map_err(sql_error)?;
    // name and body first: bm25's weights below are per column, in this order.
    db.execute_batch(
        "CREATE VIRTUAL TABLE docs USING fts5(name, body, kind UNINDEXED, pack UNINDEXED, source UNINDEXED,
            idx UNINDEXED, chapter_index UNINDEXED, chapter_name UNINDEXED, campaign UNINDEXED, tokenize = 'unicode61 remove_diacritics 2');",
    )
    .map_err(sql_error)?;
    let tx = db.unchecked_transaction().map_err(sql_error)?;
    {
        let mut insert = tx.prepare("INSERT INTO docs VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)").map_err(sql_error)?;
        for Doc { hit: h, body } in docs {
            insert
                .execute(params![h.name, body, h.kind, h.pack, h.source, h.index, h.chapter_index, h.chapter_name, h.campaign])
                .map_err(sql_error)?;
        }
    }
    tx.commit().map_err(sql_error)?;
    Ok(db)
}

/// The words of a search, lowercased; anything that isn't a letter or digit separates words.
fn words(query: &str) -> Vec<String> {
    query.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase).collect()
}

/// Every word as a word prefix; with several words, also all of them run together ("fire ball" → fireball…).
/// Words hold only letters and digits, so quoting them is enough to keep FTS5 syntax out.
fn fts_query(words: &[String]) -> String {
    let all = words.iter().map(|w| format!("\"{w}\"*")).collect::<Vec<_>>().join(" AND ");
    if words.len() > 1 { format!("({all}) OR \"{}\"*", words.concat()) } else { all }
}

/// The best matches first: names that start with the search, then by relevance with the name counting most.
/// With a `campaign`, only that campaign's own content and what every campaign shares.
pub fn search(db: &Connection, query: &str, limit: Option<u32>, campaign: Option<&str>) -> Result<Vec<Hit>, StorageError> {
    let words = words(query);
    if words.is_empty() {
        return Ok(Vec::new());
    }
    let mut select = db
        .prepare_cached(
            "SELECT kind, pack, source, idx, name, chapter_index, chapter_name, campaign FROM docs
             WHERE docs MATCH ?1 AND (?4 IS NULL OR campaign IS NULL OR campaign = ?4)
             ORDER BY lower(name) LIKE ?2 ESCAPE '\\' DESC, bm25(docs, 10.0, 1.0), name LIMIT ?3",
        )
        .map_err(sql_error)?;
    let starts = escape_like(query.trim().to_lowercase()) + "%";
    let rows = select
        .query_map(params![fts_query(&words), starts, limit.map_or(-1, i64::from), campaign], |r| {
            Ok(Hit {
                kind: r.get(0)?,
                pack: r.get(1)?,
                source: r.get(2)?,
                index: r.get(3)?,
                name: r.get(4)?,
                chapter_index: r.get(5)?,
                chapter_name: r.get(6)?,
                campaign: r.get(7)?,
            })
        })
        .map_err(sql_error)?;
    rows.collect::<Result<_, _>>().map_err(sql_error)
}

fn escape_like(text: String) -> String {
    text.chars().fold(String::new(), |mut out, c| {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
        out
    })
}

/// Runs `work` on the index for the switched-on packs, building it first when it's missing or they changed.
/// Blocking: call it inside `blocking`.
fn with_index<T>(app: &AppHandle, work: impl FnOnce(&Connection) -> Result<T, StorageError>) -> Result<T, StorageError> {
    let wanted = (disabled(app)?, CHANGES.load(Ordering::SeqCst));
    let mut index = INDEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if index.as_ref().is_none_or(|(built_for, _)| *built_for != wanted) {
        let mut docs = pack_docs(&enabled(&loaded(app)?.0, &wanted.0));
        docs.extend(character_docs(&campaigns::dirs(app)?));
        *index = Some((wanted, build(&docs)?));
    }
    work(&index.as_ref().expect("built above").1)
}

/// Builds the index ahead of the first search, so typing doesn't wait for it.
pub fn warm(app: AppHandle) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = with_index(&app, |_| Ok(())); // a failure shows on the first search instead
    });
}

/// `limit`: at most that many results (the dropdown); none for all of them (the search screen).
/// `campaign`: search only that campaign's content and the packs.
#[tauri::command]
pub async fn search_all(app: AppHandle, query: String, limit: Option<u32>, campaign: Option<String>) -> Result<Vec<Hit>, StorageError> {
    blocking(move || with_index(&app, |db| search(db, &query, limit, campaign.as_deref()))).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::load_dir;
    use std::path::Path;

    fn srd_index() -> Connection {
        let (packs, _) = load_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs")).unwrap();
        build(&pack_docs(&packs.iter().collect::<Vec<_>>())).unwrap()
    }

    fn names(db: &Connection, query: &str, limit: Option<u32>) -> Vec<String> {
        search(db, query, limit, None).unwrap().into_iter().map(|h| h.name).collect()
    }

    #[test]
    fn every_word_matches_as_a_prefix_and_names_starting_with_the_search_come_first() {
        let db = srd_index();
        assert_eq!(names(&db, "fireb", Some(1)), ["Fireball"]);
        assert!(names(&db, "fire ball", None).contains(&"Fireball".to_string()));
        let fire = names(&db, "fire", Some(8));
        assert_eq!(fire.len(), 8);
        // Names starting with "fire" first (six in the SRD), then other names with the word.
        assert!(fire[..6].iter().all(|n| n.starts_with("Fire")), "{fire:?}");
        assert!(fire.iter().all(|n| n.to_lowercase().contains("fire")), "{fire:?}");
        let hits = names(&db, "magic missile", None);
        assert_eq!(hits[0], "Magic Missile");
        assert!(names(&db, "magic zzzqx", None).is_empty());
    }

    #[test]
    fn rule_sections_carry_their_chapter_and_odd_input_is_harmless() {
        let db = srd_index();
        let hit = search(&db, "blinded", None, None).unwrap().into_iter().find(|h| h.kind == "rule-sections").unwrap();
        assert!(hit.chapter_index.is_some());
        for query in ["", "  ", "\"", "*", "a AND", "NEAR(", "50%_\\", "-"] {
            assert!(search(&db, query, Some(8), None).is_ok(), "{query:?}");
        }
        assert!(search(&db, "", None, None).unwrap().is_empty());
    }

    #[test]
    fn characters_are_found_by_name_player_or_class_and_only_in_their_own_campaign() {
        let root = std::env::temp_dir().join(format!("koldron-search-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dirs = campaigns::Dirs { active: root.join("campaigns"), archived: root.join("archived") };
        let a = campaigns::create(&dirs, "Strahd", 1).unwrap();
        let b = campaigns::create(&dirs, "Tomb", 2).unwrap();
        let mira = characters::Character {
            id: String::new(), name: "Mira".into(), player: "Lotte".into(),
            classes: vec![characters::ClassLevel { name: "Rogue".into(), level: 3 }],
            max_hp: None, current_hp: None, temp_hp: None, ac: None, passive_perception: None,
            passive_insight: None, passive_investigation: None, on_home: false, created_at: 0, updated_at: 0, picture: None,
        };
        characters::save(&active_dir(&dirs, &a.id).unwrap(), mira, 5).unwrap();
        let (packs, _) = crate::packs::load_dir(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs")).unwrap();
        let mut docs = pack_docs(&packs.iter().collect::<Vec<_>>());
        docs.extend(character_docs(&dirs));
        let db = build(&docs).unwrap();
        for query in ["mir", "lotte", "rogue"] {
            let hit = search(&db, query, None, Some(&a.id)).unwrap().into_iter().find(|h| h.kind == "characters");
            let hit = hit.unwrap_or_else(|| panic!("{query}"));
            assert_eq!((hit.name.as_str(), hit.source.as_str(), hit.campaign.as_deref()), ("Mira", "Strahd", Some(a.id.as_str())));
        }
        let in_b = search(&db, "rogue", None, Some(&b.id)).unwrap();
        assert!(!in_b.is_empty() && in_b.iter().all(|h| h.kind != "characters"), "packs stay, the other campaign's characters don't");
        assert!(search(&db, "mira", None, None).unwrap().iter().any(|h| h.kind == "characters"));
    }
}
