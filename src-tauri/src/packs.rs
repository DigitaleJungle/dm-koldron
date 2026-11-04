//! Content packs (`packs/packs.md`): finding them, validating each completely, and bundling duplicate rules.
//! Game-rules interpretation (difficulty, stat blocks…) lives elsewhere; this only checks the declared schema.

use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Component, Path};

pub const SCHEMA: u32 = 1;
const MANIFEST: &str = "pack.json";

/// Entry types a pack may declare. All but `encounter-budgets` are lists of entries with an `index` and a `name`.
pub const TYPES: [&str; 28] = [
    "ability-scores", "alignments", "backgrounds", "classes", "conditions", "damage-types", "encounter-budgets",
    "equipment", "equipment-categories", "feats", "features", "languages", "levels", "magic-items", "magic-schools",
    "monsters", "poisons", "proficiencies", "rule-sections", "rules", "skills", "species", "spells", "subclasses",
    "subspecies", "traits", "weapon-mastery-properties", "weapon-properties",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub license: String,
    // Required by the schema; nothing reads it yet (Settings → About shows the SRD's from strings.ts).
    #[allow(dead_code)]
    pub attribution: String,
    pub ruleset: String,
    pub files: Vec<DataFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataFile {
    #[serde(rename = "type")]
    pub kind: String,
    pub path: String,
}

#[derive(Debug)]
pub struct Pack {
    pub manifest: Manifest,
    /// The entries per type, from all of the pack's files of that type.
    pub entries: BTreeMap<String, Vec<Map<String, Value>>>,
}

impl Pack {
    pub fn entry_count(&self) -> usize {
        self.entries.values().map(Vec::len).sum()
    }
}

/// A folder with a `pack.json` that isn't a valid pack. `name` is the manifest's, when it could be read.
#[derive(Debug)]
pub struct Invalid {
    pub folder: String,
    pub name: Option<String>,
    pub errors: Vec<String>,
}

/// Loads every pack folder in `dir`, sorted by folder name. Folders without a `pack.json` are not packs and are
/// skipped; an invalid pack is reported, never partly loaded, and doesn't stop the others.
pub fn load_dir(dir: &Path) -> io::Result<(Vec<Pack>, Vec<Invalid>)> {
    let mut folders: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(Result::ok).filter(|e| e.path().is_dir()).collect(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e),
    };
    folders.sort_by_key(|e| e.file_name());
    let (mut packs, mut invalid) = (Vec::<Pack>::new(), Vec::new());
    for folder in folders {
        let name = folder.file_name().to_string_lossy().into_owned();
        match load_pack(&folder.path()) {
            Ok(None) => {}
            Ok(Some(pack)) if packs.iter().any(|p| p.manifest.id == pack.manifest.id) => invalid.push(Invalid {
                errors: vec![format!("Another pack already has the id \"{}\".", pack.manifest.id)],
                name: Some(pack.manifest.name),
                folder: name,
            }),
            Ok(Some(pack)) => packs.push(pack),
            Err((manifest_name, errors)) => invalid.push(Invalid { folder: name, name: manifest_name, errors }),
        }
    }
    Ok((packs, invalid))
}

type LoadError = (Option<String>, Vec<String>);

/// `Ok(None)` when the folder has no `pack.json`. On errors, the manifest's name if it was readable.
pub fn load_pack(folder: &Path) -> Result<Option<Pack>, LoadError> {
    let text = match fs::read_to_string(folder.join(MANIFEST)) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err((None, vec![format!("{MANIFEST}: {e}")])),
    };
    let manifest: Manifest = serde_json::from_str(&text).map_err(|e| (None, vec![format!("{MANIFEST}: {e}")]))?;
    let mut errors = check_manifest(&manifest);
    let mut entries: BTreeMap<String, Vec<Map<String, Value>>> = BTreeMap::new();
    for file in &manifest.files {
        if !safe_path(&file.path) || !TYPES.contains(&file.kind.as_str()) {
            continue; // already reported by check_manifest
        }
        match read_entries(&folder.join(&file.path)) {
            Ok(list) => entries.entry(file.kind.clone()).or_default().extend(list),
            Err(e) => errors.push(format!("{}: {e}", file.path)),
        }
    }
    if errors.is_empty() {
        errors = check_entries(&entries);
    }
    if errors.is_empty() {
        Ok(Some(Pack { manifest, entries }))
    } else {
        Err((Some(manifest.name), errors))
    }
}

fn check_manifest(m: &Manifest) -> Vec<String> {
    let mut errors = Vec::new();
    if m.schema != SCHEMA {
        errors.push(format!("Schema {} isn't supported; this app reads schema {SCHEMA}.", m.schema));
    }
    let id_ok = m.id.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && m.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !id_ok {
        errors.push(format!("id \"{}\" must be lowercase letters, digits and dashes.", m.id));
    }
    for (field, value) in [("name", &m.name), ("version", &m.version), ("license", &m.license), ("ruleset", &m.ruleset)] {
        if value.trim().is_empty() {
            errors.push(format!("\"{field}\" is empty."));
        }
    }
    if m.files.is_empty() {
        errors.push("\"files\" is empty.".into());
    }
    for file in &m.files {
        if !TYPES.contains(&file.kind.as_str()) {
            errors.push(format!("{}: unknown type \"{}\".", file.path, file.kind));
        }
        if !safe_path(&file.path) {
            errors.push(format!("{}: the path must stay inside the pack folder.", file.path));
        }
    }
    errors
}

/// A relative path that can't leave the pack folder.
fn safe_path(path: &str) -> bool {
    !path.is_empty() && Path::new(path).components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

fn read_entries(path: &Path) -> Result<Vec<Map<String, Value>>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let Value::Array(items) = serde_json::from_str(&text).map_err(|e| e.to_string())? else {
        return Err("must be a JSON array.".into());
    };
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| match item {
            Value::Object(map) => Ok(map),
            _ => Err(format!("entry {} isn't an object.", i + 1)),
        })
        .collect()
}

fn text_field<'a>(entry: &'a Map<String, Value>, field: &str) -> Option<&'a str> {
    entry.get(field).and_then(Value::as_str).filter(|s| !s.trim().is_empty())
}

fn check_entries(entries: &BTreeMap<String, Vec<Map<String, Value>>>) -> Vec<String> {
    let mut errors = Vec::new();
    for (kind, list) in entries {
        if kind == "encounter-budgets" {
            check_budgets(list, &mut errors);
            continue;
        }
        let mut seen = HashSet::new();
        for (i, entry) in list.iter().enumerate() {
            let at = |problem: &str| format!("{kind} entry {}: {problem}", i + 1);
            match text_field(entry, "index") {
                Some(index) if !seen.insert(index) => errors.push(at(&format!("index \"{index}\" is used twice."))),
                Some(_) => {}
                None => errors.push(at("\"index\" is missing.")),
            }
            if text_field(entry, "name").is_none() {
                errors.push(at("\"name\" is missing."));
            }
            if kind == "monsters" {
                for field in ["challenge_rating", "xp"] {
                    if !entry.get(field).is_some_and(Value::is_number) {
                        errors.push(at(&format!("\"{field}\" must be a number.")));
                    }
                }
            }
        }
    }
    // A chapter lists sections of its own pack only.
    let sections: HashSet<&str> =
        entries.get("rule-sections").into_iter().flatten().filter_map(|s| text_field(s, "index")).collect();
    for (i, chapter) in entries.get("rules").into_iter().flatten().enumerate() {
        let refs = chapter.get("subsections").and_then(Value::as_array);
        let Some(refs) = refs else {
            errors.push(format!("rules entry {}: \"subsections\" must be a list.", i + 1));
            continue;
        };
        for r in refs {
            match r.get("index").and_then(Value::as_str) {
                Some(index) if sections.contains(index) => {}
                Some(index) => errors.push(format!("rules entry {}: no rule section \"{index}\" in this pack.", i + 1)),
                None => errors.push(format!("rules entry {}: a subsection has no \"index\".", i + 1)),
            }
        }
    }
    errors
}

fn check_budgets(list: &[Map<String, Value>], errors: &mut Vec<String>) {
    let mut levels = HashSet::new();
    for (i, row) in list.iter().enumerate() {
        let at = |problem: &str| format!("encounter-budgets entry {}: {problem}", i + 1);
        match row.get("level").and_then(Value::as_u64) {
            Some(level) if level >= 1 && levels.insert(level) => {}
            Some(level) if level >= 1 => errors.push(at(&format!("level {level} is listed twice."))),
            _ => errors.push(at("\"level\" must be a whole number from 1.")),
        }
        for field in ["low", "moderate", "high"] {
            if row.get(field).and_then(Value::as_u64).is_none() {
                errors.push(at(&format!("\"{field}\" must be a whole number.")));
            }
        }
    }
}

/// One entry in a pack.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryRef {
    pub pack: String,
    pub kind: String,
    pub index: String,
}

/// The least text two same-named entries must share to be one rule (see `similarity`).
const SAME_RULE: u32 = 80;

/// Groups of entries that are the same rule: the same name (ignoring [tags] and plurals) and at least 80% of the
/// shorter text repeated in the other. The entries all stay in their packs; this only links them.
// shortcut: unused until the Rules reader and search (TODO 5, 7) show bundles; remove the allow then.
#[cfg_attr(not(test), allow(dead_code))]
pub fn bundles(packs: &[&Pack]) -> Vec<Vec<EntryRef>> {
    let mut by_name: HashMap<String, Vec<(EntryRef, Vec<String>)>> = HashMap::new();
    for pack in packs {
        for (kind, list) in &pack.entries {
            if kind == "rules" || kind == "encounter-budgets" {
                continue; // chapters are containers, budgets aren't rules text
            }
            for entry in list {
                let words = words_of(&entry_text(entry));
                let (Some(index), Some(name)) = (text_field(entry, "index"), text_field(entry, "name")) else { continue };
                if words.is_empty() {
                    continue;
                }
                let r = EntryRef { pack: pack.manifest.id.clone(), kind: kind.clone(), index: index.into() };
                by_name.entry(norm(name)).or_default().push((r, words));
            }
        }
    }
    let mut out = Vec::new();
    for members in by_name.values().filter(|m| m.len() > 1) {
        // Connected groups: a links b and b links c bundles all three.
        let mut group: Vec<usize> = (0..members.len()).collect();
        for a in 0..members.len() {
            for b in a + 1..members.len() {
                if similarity(&members[a].1, &members[b].1) >= SAME_RULE {
                    let (ga, gb) = (group[a], group[b]);
                    group.iter_mut().filter(|g| **g == gb).for_each(|g| *g = ga);
                }
            }
        }
        let mut sets: BTreeMap<usize, Vec<EntryRef>> = BTreeMap::new();
        for (i, (r, _)) in members.iter().enumerate() {
            sets.entry(group[i]).or_default().push(r.clone());
        }
        out.extend(sets.into_values().filter(|s| s.len() > 1).map(|mut s| {
            s.sort();
            s
        }));
    }
    out.sort();
    out
}

fn entry_text(entry: &Map<String, Value>) -> String {
    match entry.get("desc").or_else(|| entry.get("description")) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    }
}

/// Name for spotting duplicates: no [tag], no plural s, lowercase (as `docs/convert_srd_rules.py`'s `norm`).
fn norm(name: &str) -> String {
    let mut plain = String::new();
    let mut in_tag = false;
    for c in name.chars() {
        match c {
            '[' => in_tag = true,
            ']' => in_tag = false,
            '\'' => {}
            _ if !in_tag => plain.extend(c.to_lowercase()),
            _ => {}
        }
    }
    let plain: String = plain.chars().map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() { c } else { ' ' }).collect();
    let singular = |w: &str| {
        let plural = w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") && !w.ends_with("us");
        if plural { w[..w.len() - 1].to_string() } else { w.to_string() }
    };
    plain.split_whitespace().map(singular).collect::<Vec<_>>().join(" ")
}

fn words_of(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect()
}

/// How much of the shorter word list the other repeats in order, 0–100: the longest common word subsequence.
fn similarity(a: &[String], b: &[String]) -> u32 {
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    let mut row = vec![0u32; b.len() + 1];
    for wa in a {
        let mut diagonal = 0;
        for (j, wb) in b.iter().enumerate() {
            let up = row[j + 1];
            row[j + 1] = if wa == wb { diagonal + 1 } else { up.max(row[j]) };
            diagonal = up;
        }
    }
    (100.0 * row[b.len()] as f64 / a.len().min(b.len()) as f64).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("koldron-packs-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_pack(dir: &Path, folder: &str, manifest: Value, files: &[(&str, Value)]) {
        let root = dir.join(folder);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(MANIFEST), manifest.to_string()).unwrap();
        for (path, data) in files {
            fs::write(root.join(path), data.to_string()).unwrap();
        }
    }

    fn manifest(id: &str, files: Value) -> Value {
        serde_json::json!({ "schema": 1, "id": id, "name": id, "version": "1.0.0", "license": "MIT",
            "attribution": "", "ruleset": "2024", "files": files })
    }

    fn real_packs() -> (Vec<Pack>, Vec<Invalid>) {
        load_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs")).unwrap()
    }

    #[test]
    fn the_srd_pack_loads_and_folders_without_a_manifest_are_ignored() {
        let (packs, invalid) = real_packs();
        assert!(invalid.is_empty(), "{invalid:?}");
        assert_eq!(packs.iter().map(|p| p.manifest.id.as_str()).collect::<Vec<_>>(), ["srd"]); // not tabler-icons
        let srd = &packs[0];
        assert_eq!(srd.entries["spells"].len(), 339);
        assert_eq!(srd.entries["rule-sections"].len(), 192);
        assert_eq!(srd.entries["encounter-budgets"].len(), 20);
    }

    #[test]
    fn srd_duplicates_bundle_by_name_and_text() {
        let (packs, _) = real_packs();
        let found = bundles(&packs.iter().collect::<Vec<_>>());
        let has = |a: (&str, &str), b: (&str, &str)| {
            found.iter().any(|g| [a, b].iter().all(|(k, i)| g.iter().any(|r| r.kind == *k && r.index == *i)))
        };
        assert!(has(("rule-sections", "blinded-condition"), ("conditions", "blinded")));
        assert!(has(("rule-sections", "charmed-condition"), ("conditions", "charmed")));
        assert!(!has(("rule-sections", "d20-tests"), ("rule-sections", "d20-test"))); // 33% alike
        assert!(!has(("rule-sections", "curses"), ("rule-sections", "curses-and-magical-contagions")));
    }

    #[test]
    fn an_invalid_pack_is_reported_and_skipped_without_stopping_the_others() {
        let dir = temp_dir("mixed");
        let good = serde_json::json!([{ "index": "a", "name": "A", "description": "x" }]);
        write_pack(&dir, "good", manifest("good", serde_json::json!([{ "type": "conditions", "path": "c.json" }])), &[("c.json", good.clone())]);
        let bad_entries = serde_json::json!([{ "index": "a", "name": "A" }, { "index": "a" }, 3]);
        write_pack(&dir, "bad", manifest("bad", serde_json::json!([{ "type": "feats", "path": "f.json" }])), &[("f.json", bad_entries)]);
        write_pack(&dir, "same-id", manifest("good", serde_json::json!([{ "type": "conditions", "path": "c.json" }])), &[("c.json", good)]);
        fs::create_dir_all(dir.join("broken")).unwrap();
        fs::write(dir.join("broken").join(MANIFEST), "{ nope").unwrap();
        fs::create_dir_all(dir.join("icons")).unwrap();

        let (packs, invalid) = load_dir(&dir).unwrap();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].manifest.id, "good");
        assert_eq!(packs[0].entry_count(), 1);
        let folders: Vec<_> = invalid.iter().map(|i| i.folder.as_str()).collect();
        assert_eq!(folders, ["bad", "broken", "same-id"]);
        assert_eq!(invalid[0].name.as_deref(), Some("bad"));
        assert!(invalid[0].errors[0].contains("entry 3 isn't an object"));
        assert_eq!(invalid[1].name, None);
        assert!(invalid[2].errors[0].contains("already has the id"));
    }

    #[test]
    fn entry_problems_are_all_listed() {
        let dir = temp_dir("entries");
        let feats = serde_json::json!([{ "index": "a", "name": "A" }, { "index": "a" }, { "name": "C" }]);
        let monsters = serde_json::json!([{ "index": "m", "name": "M", "xp": 10 }]);
        let rules = serde_json::json!([{ "index": "ch", "name": "Ch", "subsections": [{ "index": "elsewhere" }] }]);
        let budgets = serde_json::json!([{ "level": 1, "low": 1, "moderate": 2, "high": 3 }, { "level": 1, "low": -1, "moderate": 2, "high": 3 }]);
        let files = serde_json::json!([
            { "type": "feats", "path": "f.json" }, { "type": "monsters", "path": "m.json" },
            { "type": "rules", "path": "r.json" }, { "type": "encounter-budgets", "path": "b.json" }]);
        write_pack(&dir, "p", manifest("p", files), &[("f.json", feats), ("m.json", monsters), ("r.json", rules), ("b.json", budgets)]);
        let (_, errors) = load_pack(&dir.join("p")).unwrap_err();
        let all = errors.join("\n");
        for want in ["index \"a\" is used twice", "feats entry 2: \"name\" is missing", "feats entry 3: \"index\" is missing",
            "\"challenge_rating\" must be a number", "no rule section \"elsewhere\"", "level 1 is listed twice", "\"low\" must be a whole number"] {
            assert!(all.contains(want), "missing \"{want}\" in:\n{all}");
        }
    }

    #[test]
    fn the_manifest_is_checked_before_any_data() {
        let dir = temp_dir("manifest");
        let mut m = manifest("Bad Id", serde_json::json!([{ "type": "wands", "path": "../outside.json" }, { "type": "feats", "path": "gone.json" }]));
        m["schema"] = 2.into();
        m["name"] = " ".into();
        write_pack(&dir, "p", m, &[]);
        let (_, errors) = load_pack(&dir.join("p")).unwrap_err();
        let all = errors.join("\n");
        for want in ["Schema 2", "lowercase letters", "\"name\" is empty", "unknown type \"wands\"", "stay inside", "gone.json"] {
            assert!(all.contains(want), "missing \"{want}\" in:\n{all}");
        }
        let mut extra = manifest("p", serde_json::json!([]));
        extra["homepage"] = "x".into();
        write_pack(&dir, "q", extra, &[]);
        assert!(load_pack(&dir.join("q")).unwrap_err().1[0].contains("homepage"));
    }

    #[test]
    fn norm_and_similarity_match_the_conversion_script() {
        assert_eq!(norm("Blinded [Condition]"), "blinded");
        assert_eq!(norm("Breaking Objects"), "breaking object");
        assert_eq!(norm("Bonus Actions"), "bonus action");
        assert_eq!(norm("Grass"), "grass");
        assert_eq!(norm("Creature's Status"), "creature status");
        let w = |s: &str| words_of(s);
        assert_eq!(similarity(&w("a b c d"), &w("x a b y c d z")), 100);
        assert_eq!(similarity(&w("a b c d"), &w("a b")), 100);
        assert_eq!(similarity(&w("a b c d"), &w("a x c y")), 50);
        assert_eq!(similarity(&w(""), &w("a")), 0);
    }
}
