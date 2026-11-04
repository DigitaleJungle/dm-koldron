//! Library → Catalog: the entries of one category from the switched-on packs, and one entry with its text as HTML.
//! Which columns a category shows is up to the frontend; this only hands out the fields it asks for.

use crate::commands::packs::{disabled, enabled, loaded};
use crate::commands::rules::render;
use crate::packs::Pack;
use crate::storage::StorageError;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
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
    Ok(packs
        .iter()
        .flat_map(|pack| {
            list(pack, kind).iter().map(|entry| CatalogRow {
                kind: kind.into(),
                pack: pack.manifest.id.clone(),
                source: pack.manifest.name.clone(),
                index: text(entry, "index").into(),
                name: text(entry, "name").into(),
                fields: fields.iter().filter_map(|f| entry.get(f).map(|v| (f.clone(), v.clone()))).collect(),
            })
        })
        .collect())
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
        entry: found.clone(),
    })
}

fn description_html(entry: &Map<String, Value>) -> String {
    let description = match entry.get("description").or_else(|| entry.get("desc")) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    };
    render(&paragraphs(&description), text(entry, "name"))
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
    fn paragraphs_keep_tables_and_lists_together() {
        assert_eq!(paragraphs("A\n  B\n\n\nC"), "A\n\nB\n\nC");
        assert_eq!(paragraphs("Intro\n| a | b |\n|---|---|\n| 1 | 2 |\nAfter"), "Intro\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nAfter");
        assert_eq!(paragraphs("- x\n- y\nZ"), "- x\n- y\n\nZ");
    }
}
