//! Global search: one SQLite FTS5 index, in memory, over search documents. Every source of searchable content (the
//! switched-on packs now; characters and plugins later) adds `Doc`s; the index doesn't know where they came from.

use crate::commands::catalog::{also, description_text, Also, KINDS};
use crate::commands::packs::{disabled, enabled, loaded};
use crate::packs::Pack;
use crate::storage::{blocking, StorageError};
use rusqlite::{params, Connection};
use std::sync::Mutex;
use tauri::AppHandle;

/// A search result: what it is (`kind`), where it lives (`pack`, `source`), and for a rule section the chapter that
/// opens it. The frontend turns `kind` into the place it opens.
pub type Hit = Also;

/// One searchable thing: its result, and the text searched besides its name.
pub struct Doc {
    pub hit: Hit,
    pub body: String,
}

/// The index, and the switched-off packs it was built for: it's rebuilt when those change.
static INDEX: Mutex<Option<(Vec<String>, Connection)>> = Mutex::new(None);

fn sql_error(e: rusqlite::Error) -> StorageError {
    StorageError::Io(format!("Search index: {e}"))
}

/// The packs' Catalog entries and the rule sections a chapter lists (one that none lists can't be opened).
pub fn pack_docs(packs: &[&Pack]) -> Vec<Doc> {
    let kinds = KINDS.iter().copied().chain(["rule-sections"]);
    kinds
        .flat_map(|kind| packs.iter().flat_map(move |p| p.entries.get(kind).into_iter().flatten().map(move |e| (*p, kind, e))))
        .map(|(pack, kind, entry)| Doc { hit: also(pack, kind, entry), body: description_text(entry) })
        .filter(|d| d.hit.kind != "rule-sections" || d.hit.chapter_index.is_some())
        .collect()
}

pub fn build(docs: &[Doc]) -> Result<Connection, StorageError> {
    let db = Connection::open_in_memory().map_err(sql_error)?;
    // name and body first: bm25's weights below are per column, in this order.
    db.execute_batch(
        "CREATE VIRTUAL TABLE docs USING fts5(name, body, kind UNINDEXED, pack UNINDEXED, source UNINDEXED,
            idx UNINDEXED, chapter_index UNINDEXED, chapter_name UNINDEXED, tokenize = 'unicode61 remove_diacritics 2');",
    )
    .map_err(sql_error)?;
    let tx = db.unchecked_transaction().map_err(sql_error)?;
    {
        let mut insert = tx.prepare("INSERT INTO docs VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)").map_err(sql_error)?;
        for Doc { hit: h, body } in docs {
            insert
                .execute(params![h.name, body, h.kind, h.pack, h.source, h.index, h.chapter_index, h.chapter_name])
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
pub fn search(db: &Connection, query: &str, limit: Option<u32>) -> Result<Vec<Hit>, StorageError> {
    let words = words(query);
    if words.is_empty() {
        return Ok(Vec::new());
    }
    let mut select = db
        .prepare_cached(
            "SELECT kind, pack, source, idx, name, chapter_index, chapter_name FROM docs WHERE docs MATCH ?1
             ORDER BY lower(name) LIKE ?2 ESCAPE '\\' DESC, bm25(docs, 10.0, 1.0), name LIMIT ?3",
        )
        .map_err(sql_error)?;
    let starts = escape_like(query.trim().to_lowercase()) + "%";
    let rows = select
        .query_map(params![fts_query(&words), starts, limit.map_or(-1, i64::from)], |r| {
            Ok(Hit {
                kind: r.get(0)?,
                pack: r.get(1)?,
                source: r.get(2)?,
                index: r.get(3)?,
                name: r.get(4)?,
                chapter_index: r.get(5)?,
                chapter_name: r.get(6)?,
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
    let off = disabled(app)?;
    let mut index = INDEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if index.as_ref().is_none_or(|(built_for, _)| *built_for != off) {
        let db = build(&pack_docs(&enabled(&loaded(app)?.0, &off)))?;
        *index = Some((off, db));
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
#[tauri::command]
pub async fn search_all(app: AppHandle, query: String, limit: Option<u32>) -> Result<Vec<Hit>, StorageError> {
    blocking(move || with_index(&app, |db| search(db, &query, limit))).await
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
        search(db, query, limit).unwrap().into_iter().map(|h| h.name).collect()
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
        let hit = search(&db, "blinded", None).unwrap().into_iter().find(|h| h.kind == "rule-sections").unwrap();
        assert!(hit.chapter_index.is_some());
        for query in ["", "  ", "\"", "*", "a AND", "NEAR(", "50%_\\", "-"] {
            assert!(search(&db, query, Some(8)).is_ok(), "{query:?}");
        }
        assert!(search(&db, "", None).unwrap().is_empty());
    }
}
