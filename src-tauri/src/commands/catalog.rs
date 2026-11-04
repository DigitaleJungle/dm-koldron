//! Library → Catalog: the entries of one category from the switched-on packs, and one entry with its text as HTML.
//! Which columns a category shows is up to the frontend; this only hands out the fields it asks for.

use crate::commands::packs::{disabled, enabled, loaded};
use crate::commands::rules::render;
use crate::bundles::{groups, norm};
use crate::packs::Pack;
use crate::storage::StorageError;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};
use tauri::AppHandle;

/// The entry types the Catalog has a tab for.
const KINDS: [&str; 9] = ["spells", "monsters", "equipment", "magic-items", "feats", "classes", "species", "backgrounds", "conditions"];

#[derive(Debug, Serialize, PartialEq)]
pub struct CatalogRow {
    /// The entry type (`spells`…), for the All tab.
    pub kind: String,
    pub pack: String,
    /// The pack's name, for the Source column.
    pub source: String,
    pub index: String,
    pub name: String,
    /// Only the requested fields the entry has.
    pub fields: Map<String, Value>,
    /// The same entry in other packs (same name, at least 80% the same text), shown as this one row.
    pub others: Vec<Also>,
}

/// Another place with the same rule: a Catalog entry, or a rule section (then with its chapter, to open it in Rules).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Also {
    pub kind: String,
    pub pack: String,
    pub source: String,
    pub index: String,
    pub name: String,
    pub chapter_index: Option<String>,
    pub chapter_name: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CatalogEntry {
    pub pack: String,
    pub source: String,
    pub entry: Map<String, Value>,
    /// The entry's description (`description` or `desc`), as HTML.
    pub html: String,
    /// Other texts of the entry as HTML: `higher_level`, and `<list>.<n>` for each monster trait or action's `desc`.
    pub parts: BTreeMap<String, String>,
    /// Entries of the same pack the page shows: a class's levels and features, a species' traits.
    pub related: Vec<Related>,
    /// The same rule elsewhere: another Catalog entry or a rule section, in any switched-on pack.
    pub also: Vec<Also>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Related {
    pub kind: String,
    pub entry: Map<String, Value>,
    /// Its description as HTML; empty for levels.
    pub html: String,
}

/// The lists of named texts in a monster entry.
const ACTION_LISTS: [&str; 5] = ["special_abilities", "actions", "bonus_actions", "reactions", "legendary_actions"];

fn check_kind(kind: &str) -> Result<(), StorageError> {
    if KINDS.contains(&kind) { Ok(()) } else { Err(StorageError::Invalid(format!("No Catalog category \"{kind}\"."))) }
}

fn text<'a>(entry: &'a Map<String, Value>, field: &str) -> &'a str {
    entry.get(field).and_then(Value::as_str).unwrap_or("")
}

fn list<'a>(pack: &'a Pack, kind: &str) -> &'a [Map<String, Value>] {
    pack.entries.get(kind).map_or(&[], Vec::as_slice)
}

pub fn rows(packs: &[&Pack], kind: &str, fields: &[String]) -> Result<Vec<CatalogRow>, StorageError> {
    check_kind(kind)?;
    let all: Vec<(&Pack, &Map<String, Value>)> = packs.iter().flat_map(|p| list(p, kind).iter().map(move |e| (*p, e))).collect();
    // Bundled entries become one row: the first (in pack order) keeps the others.
    let mut keeper: Vec<usize> = (0..all.len()).collect();
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, (_, e)) in all.iter().enumerate() {
        by_name.entry(norm(text(e, "name"))).or_default().push(i);
    }
    for same_name in by_name.values().filter(|ids| ids.len() > 1) {
        let texts: Vec<String> = same_name.iter().map(|&i| description_text(all[i].1)).collect();
        let group = groups(&texts.iter().map(String::as_str).collect::<Vec<_>>());
        for (k, &i) in same_name.iter().enumerate() {
            keeper[i] = same_name[group[k]];
        }
    }
    let mut others: HashMap<usize, Vec<Also>> = HashMap::new();
    for (i, &k) in keeper.iter().enumerate().filter(|(i, k)| *i != **k) {
        others.entry(k).or_default().push(also(all[i].0, kind, all[i].1));
    }
    Ok(all
        .iter()
        .enumerate()
        .filter(|(i, _)| keeper[*i] == *i)
        .map(|(i, (pack, entry))| CatalogRow {
            kind: kind.into(),
            pack: pack.manifest.id.clone(),
            source: pack.manifest.name.clone(),
            index: text(entry, "index").into(),
            name: text(entry, "name").into(),
            fields: fields.iter().filter_map(|f| entry.get(f).map(|v| (f.clone(), v.clone()))).collect(),
            others: others.remove(&i).unwrap_or_default(),
        })
        .collect())
}

fn also(pack: &Pack, kind: &str, entry: &Map<String, Value>) -> Also {
    let index = text(entry, "index");
    // A rule section opens in the Rules tab, in the chapter that lists it.
    let chapter = if kind == "rule-sections" {
        list(pack, "rules").iter().find(|c| {
            c.get("subsections").and_then(Value::as_array).is_some_and(|refs| refs.iter().any(|r| ref_index(Some(r)) == index))
        })
    } else {
        None
    };
    Also {
        kind: kind.into(),
        pack: pack.manifest.id.clone(),
        source: pack.manifest.name.clone(),
        index: index.into(),
        name: text(entry, "name").into(),
        chapter_index: chapter.map(|c| text(c, "index").into()),
        chapter_name: chapter.map(|c| text(c, "name").into()),
    }
}

/// The other places with the same rule as this entry: same-named Catalog entries and rule sections, any pack.
fn also_for(packs: &[&Pack], kind: &str, pack: &Pack, entry: &Map<String, Value>) -> Vec<Also> {
    let name = norm(text(entry, "name"));
    let candidates: Vec<(&Pack, &str, &Map<String, Value>)> = KINDS
        .iter()
        .copied()
        .chain(["rule-sections"])
        .flat_map(|k| packs.iter().flat_map(move |p| list(p, k).iter().map(move |e| (*p, k, e))))
        .filter(|(_, _, e)| norm(text(e, "name")) == name)
        .collect();
    let is_me = |(p, k, e): &(&Pack, &str, &Map<String, Value>)| {
        p.manifest.id == pack.manifest.id && *k == kind && text(e, "index") == text(entry, "index")
    };
    let Some(me) = candidates.iter().position(is_me) else { return Vec::new() };
    let texts: Vec<String> = candidates.iter().map(|(_, _, e)| description_text(e)).collect();
    let group = groups(&texts.iter().map(String::as_str).collect::<Vec<_>>());
    candidates
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != me && group[*i] == group[me])
        .map(|(_, (p, k, e))| also(p, k, e))
        // A rule section no chapter lists can't be opened in the Rules tab, so it isn't offered.
        .filter(|a| a.kind != "rule-sections" || a.chapter_index.is_some())
        .collect()
}

pub fn entry(packs: &[&Pack], kind: &str, pack: &str, index: &str) -> Result<CatalogEntry, StorageError> {
    check_kind(kind)?;
    let not_found = || StorageError::NotFound(format!("No {kind} entry \"{index}\" in the switched-on packs."));
    let found_pack = packs.iter().find(|p| p.manifest.id == pack).ok_or_else(not_found)?;
    let found = list(found_pack, kind).iter().find(|e| text(e, "index") == index).ok_or_else(not_found)?;
    Ok(CatalogEntry {
        pack: pack.into(),
        source: found_pack.manifest.name.clone(),
        html: description_html(found),
        parts: parts(found),
        related: related(found_pack, kind, found),
        also: also_for(packs, kind, found_pack, found),
        entry: found.clone(),
    })
}

fn description_text(entry: &Map<String, Value>) -> String {
    match entry.get("description").or_else(|| entry.get("desc")) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

fn description_html(entry: &Map<String, Value>) -> String {
    render(&paragraphs(&description_text(entry)), text(entry, "name"))
}

fn parts(entry: &Map<String, Value>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Some(higher) = entry.get("higher_level").and_then(Value::as_str) {
        out.insert("higher_level".into(), render(&paragraphs(higher), ""));
    }
    for list in ACTION_LISTS {
        for (i, item) in entry.get(list).and_then(Value::as_array).into_iter().flatten().enumerate() {
            if let Some(desc) = item.get("desc").and_then(Value::as_str) {
                out.insert(format!("{list}.{i}"), render(&paragraphs(desc), ""));
            }
        }
    }
    out
}

fn ref_index(value: Option<&Value>) -> &str {
    value.and_then(|r| r.get("index")).and_then(Value::as_str).unwrap_or("")
}

/// A class: its own levels in order (not a subclass's), then the features those levels list, in that order.
/// A species: its traits, in its order.
fn related(pack: &Pack, kind: &str, entry: &Map<String, Value>) -> Vec<Related> {
    let index = text(entry, "index");
    let find = |kind: &str, wanted: &str| list(pack, kind).iter().find(|e| text(e, "index") == wanted);
    let item = |kind: &str, e: &Map<String, Value>, html: String| Related { kind: kind.into(), entry: e.clone(), html };
    match kind {
        "classes" => {
            let mut levels: Vec<_> =
                list(pack, "levels").iter().filter(|l| ref_index(l.get("class")) == index && l.get("subclass").is_none()).collect();
            levels.sort_by_key(|l| l.get("level").and_then(Value::as_u64).unwrap_or(0));
            // A feature several levels list (Ability Score Improvement) comes once, at its first level.
            let mut seen = std::collections::HashSet::new();
            let features: Vec<_> = levels
                .iter()
                .flat_map(|l| l.get("features").and_then(Value::as_array).into_iter().flatten())
                .map(|r| ref_index(Some(r)))
                .filter(|index| seen.insert(*index))
                .filter_map(|index| find("features", index))
                .map(|f| item("features", f, description_html(f)))
                .collect();
            levels.into_iter().map(|l| item("levels", l, String::new())).chain(features).collect()
        }
        "species" => entry
            .get("traits")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|r| find("traits", ref_index(Some(r))))
            .map(|t| item("traits", t, description_html(t)))
            .collect(),
        _ => Vec::new(),
    }
}

/// 5e-database text breaks paragraphs with a single newline; Markdown needs a blank line. Consecutive table rows and
/// list items stay together.
fn paragraphs(text: &str) -> String {
    let kind = |line: &str| match line {
        l if l.starts_with('|') => 1,
        l if l.starts_with("- ") || l.starts_with("* ") => 2,
        _ => 0,
    };
    let mut out = String::new();
    let mut previous = None;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(previous) = previous {
            out.push_str(if previous != 0 && previous == kind(line) { "\n" } else { "\n\n" });
        }
        out.push_str(line);
        previous = Some(kind(line));
    }
    out
}

#[tauri::command]
pub async fn catalog_rows(app: AppHandle, kind: String, fields: Vec<String>) -> Result<Vec<CatalogRow>, StorageError> {
    tauri::async_runtime::spawn_blocking(move || rows(&enabled(&loaded(&app)?.0, &disabled(&app)?), &kind, &fields))
        .await
        .map_err(|e| StorageError::Io(e.to_string()))?
}

#[tauri::command]
pub async fn catalog_entry(app: AppHandle, kind: String, pack: String, index: String) -> Result<CatalogEntry, StorageError> {
    tauri::async_runtime::spawn_blocking(move || entry(&enabled(&loaded(&app)?.0, &disabled(&app)?), &kind, &pack, &index))
        .await
        .map_err(|e| StorageError::Io(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::load_dir;
    use std::path::Path;

    fn srd() -> Vec<Pack> {
        load_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs")).unwrap().0
    }

    #[test]
    fn rows_carry_only_the_asked_fields() {
        let packs = srd();
        let spells = rows(&packs.iter().collect::<Vec<_>>(), "spells", &["level".into(), "nope".into()]).unwrap();
        assert_eq!(spells.len(), 339);
        assert_eq!(spells[0].source, "SRD 5.2.1");
        assert_eq!(spells[0].fields.keys().collect::<Vec<_>>(), ["level"]);
        assert!(matches!(rows(&[], "rule-sections", &[]), Err(StorageError::Invalid(_))));
        assert!(rows(&[], "spells", &[]).unwrap().is_empty());
    }

    #[test]
    fn an_entry_with_its_text_in_paragraphs() {
        let packs = srd();
        let on: Vec<_> = packs.iter().collect();
        let blinded = entry(&on, "conditions", "srd", "blinded").unwrap();
        assert_eq!(blinded.entry["name"], "Blinded");
        assert!(blinded.html.matches("<p>").count() >= 3, "{}", blinded.html);
        let slaying = entry(&on, "magic-items", "srd", "ammunition-of-slaying").unwrap();
        assert!(slaying.html.contains("<table>"), "{}", slaying.html);
        assert!(matches!(entry(&on, "spells", "srd", "nope"), Err(StorageError::NotFound(_))));
    }

    #[test]
    fn entries_bring_their_texts_and_related_entries() {
        let packs = srd();
        let on: Vec<_> = packs.iter().collect();
        let fireball = entry(&on, "spells", "srd", "fireball").unwrap();
        assert!(fireball.parts["higher_level"].contains("1d6"));
        let dragon = entry(&on, "monsters", "srd", "adult-red-dragon").unwrap();
        assert!(dragon.parts.contains_key("actions.0") && dragon.parts.contains_key("legendary_actions.0"));
        let barbarian = entry(&on, "classes", "srd", "barbarian").unwrap();
        let levels: Vec<_> = barbarian.related.iter().filter(|r| r.kind == "levels").map(|r| r.entry["level"].as_u64().unwrap()).collect();
        assert_eq!(levels, (1..=20).collect::<Vec<_>>());
        let first = barbarian.related.iter().find(|r| r.kind == "features").unwrap();
        assert_eq!((first.entry["name"].as_str().unwrap(), first.html.is_empty()), ("Rage", false));
        let features: Vec<_> = barbarian.related.iter().filter(|r| r.kind == "features").map(|r| r.entry["index"].as_str().unwrap()).collect();
        let unique: std::collections::HashSet<_> = features.iter().collect();
        assert_eq!(unique.len(), features.len(), "Ability Score Improvement is listed at 4, 8, 12 and 16 but shown once");
        let dragonborn = entry(&on, "species", "srd", "dragonborn").unwrap();
        assert!(!dragonborn.related.is_empty() && dragonborn.related.iter().all(|r| r.kind == "traits" && !r.html.is_empty()));
        assert!(entry(&on, "feats", "srd", "alert").unwrap().related.is_empty());
    }

    #[test]
    fn a_condition_is_also_its_glossary_term_but_a_name_collision_is_not() {
        let packs = srd();
        let on: Vec<_> = packs.iter().collect();
        let blinded = entry(&on, "conditions", "srd", "blinded").unwrap();
        let term = blinded.also.iter().find(|a| a.kind == "rule-sections").expect("the glossary term");
        assert_eq!((term.index.as_str(), term.chapter_index.as_deref()), ("blinded-condition", Some("rules-glossary")));
        assert!(blinded.also.iter().all(|a| a.kind != "rule-sections" || a.chapter_index.is_some()));
        let darkness = entry(&on, "spells", "srd", "darkness").unwrap();
        assert!(darkness.also.is_empty(), "the Darkness spell isn't the Darkness rule: {:?}", darkness.also);
    }

    #[test]
    fn the_srd_alone_has_no_rows_merged() {
        let packs = srd();
        let on: Vec<_> = packs.iter().collect();
        for kind in KINDS {
            let merged: Vec<_> = rows(&on, kind, &[]).unwrap().into_iter().filter(|r| !r.others.is_empty()).map(|r| r.name).collect();
            assert!(merged.is_empty(), "{kind}: {merged:?}");
        }
    }

    #[test]
    fn the_same_entry_in_two_packs_is_one_row() {
        let dir = std::env::temp_dir().join(format!("koldron-catalog-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (id, desc) in [("a", "You see nothing at all in the dark."), ("b", "You see nothing at all in the dark, really.")] {
            std::fs::create_dir_all(dir.join(id)).unwrap();
            let manifest = serde_json::json!({ "schema": 1, "id": id, "name": id.to_uppercase(), "version": "1", "license": "MIT",
                "attribution": "", "ruleset": "2024", "files": [{ "type": "conditions", "path": "c.json" }] });
            std::fs::write(dir.join(id).join("pack.json"), manifest.to_string()).unwrap();
            let data = serde_json::json!([{ "index": "blinded", "name": "Blinded", "description": desc }, { "index": id, "name": id, "description": "x" }]);
            std::fs::write(dir.join(id).join("c.json"), data.to_string()).unwrap();
        }
        let packs = load_dir(&dir).unwrap().0;
        let list = rows(&packs.iter().collect::<Vec<_>>(), "conditions", &[]).unwrap();
        assert_eq!(list.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["Blinded", "a", "b"]);
        assert_eq!(list[0].others.iter().map(|o| o.source.as_str()).collect::<Vec<_>>(), ["B"]);
    }

    #[test]
    fn paragraphs_keep_tables_and_lists_together() {
        assert_eq!(paragraphs("A\n  B\n\n\nC"), "A\n\nB\n\nC");
        assert_eq!(paragraphs("Intro\n| a | b |\n|---|---|\n| 1 | 2 |\nAfter"), "Intro\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nAfter");
        assert_eq!(paragraphs("- x\n- y\nZ"), "- x\n- y\n\nZ");
    }
}
