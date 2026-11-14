//! Content packs (`packs/packs.md`): finding them and validating each completely.
//! Game-rules interpretation (difficulty, stat blocks…) lives elsewhere; this only checks the declared schema.

use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use crate::storage::{check_id, subfolders, StorageError};
use std::path::{Component, Path};

pub const SCHEMA: u32 = 1;
const MANIFEST: &str = "pack.json";
/// Optional: the images the pack's entries name that can be downloaded (Settings → Rules → Enable images).
const IMAGES: &str = "images.json";

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

/// `images.json`: where the images are, and each downloadable image's path (an entry's `image`) with its size in bytes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Images {
    pub host: String,
    pub images: BTreeMap<String, u64>,
}

#[derive(Debug)]
pub struct Pack {
    pub manifest: Manifest,
    /// The entries per type, from all of the pack's files of that type.
    pub entries: BTreeMap<String, Vec<Map<String, Value>>>,
    pub images: Option<Images>,
    /// Installed in the data folder (by an addon), not bundled with the app.
    pub installed: bool,
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
    let (mut packs, mut invalid) = (Vec::<Pack>::new(), Vec::new());
    for folder in subfolders(dir)? {
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

/// The bundled packs and the ones installed in the data folder, sorted by id. An installed pack is used instead of a
/// bundled one with the same id (an updated SRD); an invalid installed one doesn't hide the bundled one.
pub fn load_all(bundled: &Path, installed: &Path) -> io::Result<(Vec<Pack>, Vec<Invalid>)> {
    let (mut packs, mut invalid) = load_dir(installed)?;
    packs.iter_mut().for_each(|p| p.installed = true);
    let (shipped, shipped_invalid) = load_dir(bundled)?;
    let replaced: HashSet<String> = packs.iter().map(|p| p.manifest.id.clone()).collect();
    packs.extend(shipped.into_iter().filter(|p| !replaced.contains(&p.manifest.id)));
    packs.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    invalid.extend(shipped_invalid);
    Ok((packs, invalid))
}

/// A pack handed over to be installed: its `pack.json`, each data file by the path the manifest gives it, and
/// optionally its `images.json` and license text.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewPack {
    pub manifest: Value,
    pub files: BTreeMap<String, Value>,
    pub images: Option<Value>,
    pub license: Option<String>,
}

const LICENSE: &str = "LICENSE.md";

/// Writes `new` to `staging`, validates it there completely, then swaps it in as `installed/<id>` (replacing the pack
/// installed with that id). On any failure `installed` is left as it was. `staging` must be on the same disk.
pub fn install(installed: &Path, staging: &Path, new: &NewPack) -> Result<Pack, StorageError> {
    let _ = fs::remove_dir_all(staging); // what a crash left behind
    let result = write_and_load(staging, new).and_then(|pack| {
        let target = installed.join(&pack.manifest.id);
        let old = staging.with_extension("old");
        let _ = fs::remove_dir_all(&old);
        fs::create_dir_all(installed)?;
        let had_one = target.exists();
        if had_one {
            fs::rename(&target, &old)?;
        }
        if let Err(e) = fs::rename(staging, &target) {
            if had_one {
                let _ = fs::rename(&old, &target); // put the previous pack back
            }
            return Err(e.into());
        }
        let _ = fs::remove_dir_all(&old);
        Ok(pack)
    });
    let _ = fs::remove_dir_all(staging);
    result.map(|mut pack| {
        pack.installed = true;
        pack
    })
}

fn write_and_load(staging: &Path, new: &NewPack) -> Result<Pack, StorageError> {
    let write = |path: &Path, bytes: &[u8]| -> Result<(), StorageError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(fs::write(path, bytes)?)
    };
    let json = |value: &Value| serde_json::to_vec(value).expect("a JSON value serializes");
    for (path, data) in &new.files {
        if !safe_path(path) || [MANIFEST, IMAGES, LICENSE].contains(&path.as_str()) {
            return Err(StorageError::Invalid(format!("{path}: not a data file path a pack can use.")));
        }
        write(&staging.join(path), &json(data))?;
    }
    write(&staging.join(MANIFEST), &json(&new.manifest))?;
    if let Some(images) = &new.images {
        write(&staging.join(IMAGES), &json(images))?;
    }
    if let Some(license) = &new.license {
        write(&staging.join(LICENSE), license.as_bytes())?;
    }
    match load_pack(staging) {
        Ok(Some(pack)) => Ok(pack),
        Ok(None) => Err(StorageError::Invalid(format!("{MANIFEST} is missing."))),
        Err((_, errors)) => Err(StorageError::Invalid(errors.join("
"))),
    }
}

/// Removes the pack installed as `installed/<id>`. A bundled pack can't be removed.
pub fn remove(installed: &Path, id: &str) -> Result<(), StorageError> {
    check_id(id, "pack")?;
    let folder = installed.join(id);
    if !folder.is_dir() {
        return Err(StorageError::NotFound(format!("\"{id}\" isn't an installed pack; a bundled pack can't be removed.")));
    }
    Ok(fs::remove_dir_all(folder)?)
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
    let images = match read_images(folder) {
        Ok(images) => images,
        Err(e) => {
            errors.push(e);
            None
        }
    };
    if errors.is_empty() {
        Ok(Some(Pack { manifest, entries, images, installed: false }))
    } else {
        Err((Some(manifest.name), errors))
    }
}

fn read_images(folder: &Path) -> Result<Option<Images>, String> {
    let text = match fs::read_to_string(folder.join(IMAGES)) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{IMAGES}: {e}")),
    };
    let images: Images = serde_json::from_str(&text).map_err(|e| format!("{IMAGES}: {e}"))?;
    if !images.host.starts_with("https://") || images.host.len() == 8 || images.host[8..].contains(['/', '?', '#', '@']) {
        return Err(format!("{IMAGES}: \"host\" must be https:// and a host name only."));
    }
    if let Some(bad) = images.images.keys().find(|p| !p.starts_with('/')) {
        return Err(format!("{IMAGES}: \"{bad}\" must start with /."));
    }
    // The window may only show kept images named *.png (the asset scope in tauri.conf.json).
    if let Some(bad) = images.images.keys().find(|p| !p.ends_with(".png")) {
        return Err(format!("{IMAGES}: \"{bad}\" must be a .png image."));
    }
    Ok(Some(images))
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
pub fn safe_path(path: &str) -> bool {
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
        let mut listed = HashSet::new();
        for r in refs {
            match r.get("index").and_then(Value::as_str) {
                Some(index) if !listed.insert(index) => {
                    errors.push(format!("rules entry {}: section \"{index}\" is listed twice.", i + 1))
                }
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
        let images = srd.images.as_ref().expect("the SRD lists its images");
        assert_eq!(images.host, "https://www.dnd5eapi.co");
        assert!(images.images.len() > 300);
    }

    #[test]
    fn an_images_list_must_name_an_https_host_and_absolute_paths() {
        let dir = temp_dir("images");
        let data = serde_json::json!([{ "index": "a", "name": "A", "description": "x" }]);
        let files = serde_json::json!([{ "type": "conditions", "path": "c.json" }]);
        for (folder, images, ok) in [
            ("good", serde_json::json!({ "host": "https://img.example", "images": { "/a.png": 1 } }), true),
            ("http", serde_json::json!({ "host": "http://img.example", "images": {} }), false),
            ("no-host", serde_json::json!({ "host": "https://", "images": {} }), false),
            ("path-in-host", serde_json::json!({ "host": "https://img.example/x", "images": {} }), false),
            ("relative", serde_json::json!({ "host": "https://img.example", "images": { "a.png": 1 } }), false),
            ("not-png", serde_json::json!({ "host": "https://img.example", "images": { "/a.jpg": 1 } }), false),
        ] {
            write_pack(&dir, folder, manifest(folder, files.clone()), &[("c.json", data.clone()), (IMAGES, images)]);
            assert_eq!(load_pack(&dir.join(folder)).is_ok(), ok, "{folder}");
        }
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
        let rules = serde_json::json!([{ "index": "ch", "name": "Ch", "subsections": [{ "index": "elsewhere" }, { "index": "elsewhere" }] }]);
        let budgets = serde_json::json!([{ "level": 1, "low": 1, "moderate": 2, "high": 3 }, { "level": 1, "low": -1, "moderate": 2, "high": 3 }]);
        let files = serde_json::json!([
            { "type": "feats", "path": "f.json" }, { "type": "monsters", "path": "m.json" },
            { "type": "rules", "path": "r.json" }, { "type": "encounter-budgets", "path": "b.json" }]);
        write_pack(&dir, "p", manifest("p", files), &[("f.json", feats), ("m.json", monsters), ("r.json", rules), ("b.json", budgets)]);
        let (_, errors) = load_pack(&dir.join("p")).unwrap_err();
        let all = errors.join("\n");
        for want in ["index \"a\" is used twice", "feats entry 2: \"name\" is missing", "feats entry 3: \"index\" is missing",
            "\"challenge_rating\" must be a number", "no rule section \"elsewhere\"", "section \"elsewhere\" is listed twice", "level 1 is listed twice", "\"low\" must be a whole number"] {
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

    fn new_pack(id: &str, version: &str, conditions: Value) -> NewPack {
        let mut manifest = manifest(id, serde_json::json!([{ "type": "conditions", "path": "data/c.json" }]));
        manifest["version"] = version.into();
        NewPack { manifest, files: [("data/c.json".to_string(), conditions)].into(), images: None, license: Some("MIT".into()) }
    }

    #[test]
    fn installing_validates_first_then_replaces_the_installed_pack() {
        let dir = temp_dir("install");
        let (installed, staging) = (dir.join("packs"), dir.join("staging/new"));
        let good = serde_json::json!([{ "index": "a", "name": "A", "description": "x" }]);
        let pack = install(&installed, &staging, &new_pack("home", "1.0.0", good.clone())).unwrap();
        assert!(pack.installed && installed.join("home/data/c.json").is_file() && installed.join("home/LICENSE.md").is_file());

        // An invalid update is refused with its errors and the installed version stays.
        let Err(StorageError::Invalid(errors)) = install(&installed, &staging, &new_pack("home", "2.0.0", serde_json::json!([{ "index": "a" }]))) else {
            panic!("an invalid pack must be refused");
        };
        assert!(errors.contains("\"name\" is missing"), "{errors}");
        assert_eq!(load_pack(&installed.join("home")).unwrap().unwrap().manifest.version, "1.0.0");

        install(&installed, &staging, &new_pack("home", "2.0.0", good)).unwrap();
        assert_eq!(load_pack(&installed.join("home")).unwrap().unwrap().manifest.version, "2.0.0");
        assert!(!staging.exists() && !staging.with_extension("old").exists());

        let mut escaping = new_pack("x", "1", serde_json::json!([]));
        escaping.files.insert("../evil.json".into(), serde_json::json!([]));
        assert!(matches!(install(&installed, &staging, &escaping), Err(StorageError::Invalid(_))));
        assert!(!dir.join("evil.json").exists());
    }

    #[test]
    fn an_installed_pack_replaces_the_bundled_one_and_removing_it_brings_that_back() {
        let dir = temp_dir("store");
        let (bundled, installed) = (dir.join("bundled"), dir.join("installed"));
        let files = serde_json::json!([{ "type": "conditions", "path": "c.json" }]);
        let data = serde_json::json!([{ "index": "a", "name": "A", "description": "x" }]);
        for (root, id, version) in [(&bundled, "srd", "1"), (&bundled, "extra", "1"), (&installed, "srd", "2")] {
            let mut m = manifest(id, files.clone());
            m["version"] = version.into();
            write_pack(root, id, m, &[("c.json", data.clone())]);
        }
        let (packs, _) = load_all(&bundled, &installed).unwrap();
        let got: Vec<_> = packs.iter().map(|p| (p.manifest.id.as_str(), p.manifest.version.as_str(), p.installed)).collect();
        assert_eq!(got, [("extra", "1", false), ("srd", "2", true)]);

        // An installed pack that no longer validates (say, after a schema change) leaves the bundled one in use.
        fs::create_dir_all(installed.join("extra")).unwrap();
        fs::write(installed.join("extra").join(MANIFEST), "{ nope").unwrap();
        let (packs, invalid) = load_all(&bundled, &installed).unwrap();
        assert_eq!(packs.iter().find(|p| p.manifest.id == "extra").map(|p| p.installed), Some(false));
        assert_eq!(invalid.iter().map(|i| i.folder.as_str()).collect::<Vec<_>>(), ["extra"]);
        fs::remove_dir_all(installed.join("extra")).unwrap();

        assert!(matches!(remove(&installed, "extra"), Err(StorageError::NotFound(_))));
        assert!(matches!(remove(&installed, "../bundled"), Err(StorageError::Invalid(_))));
        remove(&installed, "srd").unwrap();
        let (packs, _) = load_all(&bundled, &installed).unwrap();
        assert_eq!(packs.iter().find(|p| p.manifest.id == "srd").unwrap().manifest.version, "1");
    }
}
