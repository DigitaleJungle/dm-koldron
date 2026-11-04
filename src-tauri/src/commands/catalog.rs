//! Library → Catalog: the entries of one category from the switched-on packs, and one entry with its text as HTML.
//! Which columns a category shows is up to the frontend; this only hands out the fields it asks for.

use crate::commands::packs::{disabled, enabled, loaded};
use crate::commands::rules::render;
use crate::packs::Pack;
use crate::storage::StorageError;
use serde::Serialize;
use serde_json::{Map, Value};
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
}

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
    let description = match found.get("description").or_else(|| found.get("desc")) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    };
    Ok(CatalogEntry {
        pack: pack.into(),
        source: found_pack.manifest.name.clone(),
        html: render(&paragraphs(&description), text(found, "name")),
        entry: found.clone(),
    })
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
    fn paragraphs_keep_tables_and_lists_together() {
        assert_eq!(paragraphs("A\n  B\n\n\nC"), "A\n\nB\n\nC");
        assert_eq!(paragraphs("Intro\n| a | b |\n|---|---|\n| 1 | 2 |\nAfter"), "Intro\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nAfter");
        assert_eq!(paragraphs("- x\n- y\nZ"), "- x\n- y\n\nZ");
    }
}
