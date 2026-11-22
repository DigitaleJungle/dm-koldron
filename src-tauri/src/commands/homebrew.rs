//! Homebrew books (`docs/SPEC.md` → Homebrew): content packs the app writes in the data folder's `homebrew/`. A save
//! writes the whole book again and installs it like any pack: checked completely, then swapped in atomically.

use crate::commands::packs::{disabled, enabled, homebrew_dir, homebrew_staging, loaded, lock_pack_change, reload};
use crate::commands::addons::{dialog, picked_path};
use crate::commands::images;
use crate::commands::settings;
use crate::packs::{self, NewPack, Pack, SCHEMA, TYPES};
use crate::storage::{blocking, data_dir, write_atomic_bytes, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use std::path::Path;
use tauri::{ipc, AppHandle};

const MAX_NAME: usize = 100;

/// A homebrew book as the Homebrew tab edits it: the manifest's fields and the entries per pack type.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Book {
    /// Empty for a new book: made from the name on its first save, then never changed.
    pub id: String,
    pub name: String,
    pub version: String,
    pub license: String,
    pub attribution: String,
    pub ruleset: String,
    pub entries: BTreeMap<String, Vec<Map<String, Value>>>,
}

impl From<&Pack> for Book {
    fn from(pack: &Pack) -> Self {
        let m = &pack.manifest;
        Book {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version.clone(),
            license: m.license.clone(),
            attribution: m.attribution.clone(),
            ruleset: m.ruleset.clone(),
            entries: pack.entries.clone(),
        }
    }
}

/// Lowercase letters, digits and dashes made from `name`, as a pack id must be.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-');
    if out.is_empty() { "book".into() } else { out.into() }
}

/// The book's id (made for a new one, never in `taken`) and its files as `packs::install` takes them.
fn prepare(mut book: Book, packs: &[Pack], taken: &dyn Fn(&str) -> bool) -> Result<NewPack, StorageError> {
    book.name = book.name.trim().into();
    if book.name.is_empty() {
        return Err(StorageError::Invalid("The name is required.".into()));
    }
    if book.name.chars().count() > MAX_NAME {
        return Err(StorageError::Invalid(format!("The name can be at most {MAX_NAME} characters.")));
    }
    if book.id.is_empty() {
        let base = slug(&book.name);
        book.id = (1..).map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") }).find(|id| !taken(id)).expect("a free id");
    } else if !packs.iter().any(|p| p.homebrew && p.manifest.id == book.id) {
        return Err(StorageError::NotFound(format!("There's no homebrew book \"{}\".", book.id)));
    }
    let lower = book.name.to_lowercase();
    if packs.iter().any(|p| p.manifest.id != book.id && p.manifest.name.to_lowercase() == lower) {
        return Err(StorageError::Invalid(format!("Another book is already called \"{}\".", book.name)));
    }
    if let Some(kind) = book.entries.keys().find(|k| !TYPES.contains(&k.as_str())) {
        return Err(StorageError::Invalid(format!("Unknown entry type \"{kind}\".")));
    }
    // A pack needs at least one file: a book without entries yet has an empty chapter list.
    book.entries.retain(|_, list| !list.is_empty());
    if book.entries.is_empty() {
        book.entries.insert("rules".into(), Vec::new());
    }
    let mut files = BTreeMap::new();
    let mut listed = Vec::new();
    for (kind, list) in book.entries {
        let path = format!("data/{kind}.json");
        listed.push(json!({ "type": kind, "path": path }));
        files.insert(path, Value::Array(list.into_iter().map(Value::Object).collect()));
    }
    let manifest = json!({
        "schema": SCHEMA, "id": book.id, "name": book.name, "version": book.version, "license": book.license,
        "attribution": book.attribution, "ruleset": book.ruleset, "files": listed,
    });
    Ok(NewPack { manifest, files, images: None, license: None })
}

/// A choice in a Homebrew form: an entry to refer to, or a value the data already uses (then `index` is the value).
#[derive(Debug, Serialize, PartialEq)]
pub struct Choice {
    pub index: String,
    pub name: String,
}

/// The entries of `kind` in `packs` (once per index), or with `field` the different values of that field ("a.b" for a
/// field inside one; each item of a list), sorted by name.
fn choices(packs: &[&Pack], kind: &str, field: Option<&str>) -> Vec<Choice> {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for entry in packs.iter().flat_map(|p| p.entries.get(kind).into_iter().flatten()) {
        let Some(path) = field else {
            if let (Some(Value::String(index)), Some(Value::String(name))) = (entry.get("index"), entry.get("name")) {
                found.entry(index.clone()).or_insert_with(|| name.clone());
            }
            continue;
        };
        let mut parts = path.split('.');
        let mut value = parts.next().and_then(|first| entry.get(first));
        for part in parts {
            value = value.and_then(|v| v.get(part));
        }
        let values = match value {
            Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).collect(),
            Some(Value::String(s)) => vec![s.as_str()],
            _ => Vec::new(),
        };
        for v in values.into_iter().filter(|v| !v.trim().is_empty()) {
            found.insert(v.into(), v.into());
        }
    }
    let mut list: Vec<Choice> = found.into_iter().map(|(index, name)| Choice { index, name }).collect();
    list.sort_by_key(|c| c.name.to_lowercase());
    list
}

/// The Homebrew forms' choices, from the switched-on packs (`choices`).
#[tauri::command]
pub async fn homebrew_choices(app: AppHandle, kind: String, field: Option<String>) -> Result<Vec<Choice>, StorageError> {
    blocking(move || {
        let loaded = loaded(&app)?;
        Ok(choices(&enabled(&loaded.0, &disabled(&app)?), &kind, field.as_deref()))
    })
    .await
}

/// A book's folder as a pack .zip: its files at the top, as Add pack .zip takes it.
fn zip_book(folder: &Path) -> Result<Vec<u8>, StorageError> {
    let failed = |e: zip::result::ZipError| StorageError::Io(e.to_string());
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut folders = vec![folder.to_path_buf()];
    while let Some(dir) = folders.pop() {
        for item in std::fs::read_dir(&dir)? {
            let path = item?.path();
            if path.is_dir() {
                folders.push(path);
                continue;
            }
            let name = path.strip_prefix(folder).map_err(|e| StorageError::Io(e.to_string()))?;
            zip.start_file(name.to_string_lossy().replace('\\', "/"), options).map_err(failed)?;
            zip.write_all(&std::fs::read(&path)?)?;
        }
    }
    Ok(zip.finish().map_err(failed)?.into_inner())
}

/// Export: the user picks where the book goes as a pack .zip (`file_type`: the dialog's name for it). False when
/// cancelled.
#[tauri::command]
pub async fn export_homebrew(app: AppHandle, id: String, file_type: String) -> Result<bool, StorageError> {
    blocking(move || {
        let current = loaded(&app)?;
        let book = current.0.iter().find(|p| p.homebrew && p.manifest.id == id);
        let book = book.ok_or_else(|| StorageError::NotFound(format!("There's no homebrew book \"{id}\".")))?;
        let bytes = zip_book(&homebrew_dir(&app)?.join(&book.manifest.id))?;
        let Some(path) = picked_path(dialog(&app).set_file_name(format!("{id}.zip")).add_filter(&file_type, &["zip"]).blocking_save_file())? else {
            return Ok(false);
        };
        write_atomic_bytes(&path, &bytes)?;
        Ok(true)
    })
    .await
}

/// A picked picture is at most this large before it's shrunk.
const MAX_PICTURE: usize = 20 * 1024 * 1024;

/// The file name a picture is kept under in its book's images folder: from its contents, so the same picture is kept once.
fn picture_name(png: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    png.hash(&mut hasher);
    format!("homebrew-{:016x}.png", hasher.finish())
}

/// The pictures a book's entries name (`image`, any type).
fn pictures_in(entries: &BTreeMap<String, Vec<Map<String, Value>>>) -> std::collections::HashSet<String> {
    entries.values().flatten().filter_map(|e| e.get("image").and_then(Value::as_str)).map(str::to_string).collect()
}

/// Removes the kept pictures in `dir` that no entry names any more (replaced, removed, or picked and never saved).
fn prune_pictures(dir: &Path, named: &std::collections::HashSet<String>) -> Result<(), StorageError> {
    let keep: std::collections::HashSet<_> = named.iter().map(|n| images::file_for(dir, n)).collect();
    let files = match std::fs::read_dir(dir) {
        Ok(files) => files,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    for file in files {
        let path = file?.path();
        if path.is_file() && !keep.contains(&path) {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

/// A picture for an entry, chapter or section of a saved book: kept (as PNG, shrunk to 1024 px) in the book's images
/// folder until a save no longer names it. Returns the name to put in the entry's `image`. The picture comes as the
/// raw request body (the book in a header), not as JSON: a photo as a JSON list of numbers is many times its size.
#[tauri::command]
pub async fn put_homebrew_picture(app: AppHandle, request: ipc::Request<'_>) -> Result<String, StorageError> {
    let Some(book) = request.headers().get("book").and_then(|v| v.to_str().ok()).map(str::to_string) else {
        return Err(StorageError::Invalid("A homebrew picture needs its book.".into()));
    };
    let ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(StorageError::Invalid("A homebrew picture needs its bytes as the raw body.".into()));
    };
    let bytes = bytes.clone();
    blocking(move || keep_picture(&app, &book, &bytes)).await
}

/// A book's picture kept for another book too (a new entry's form moved to that book before its first save). Returns
/// its name there.
#[tauri::command]
pub async fn copy_homebrew_picture(app: AppHandle, from: String, to: String, image: String) -> Result<String, StorageError> {
    blocking(move || {
        let bytes = std::fs::read(images::file_for(&data_dir(&app)?.join(images::DIR).join(&from), &image))?;
        keep_picture(&app, &to, &bytes)
    })
    .await
}

/// Keeps a picture for a homebrew book (as PNG, shrunk to 1024 px) and returns its name. Blocking.
pub(crate) fn keep_picture(app: &AppHandle, book: &str, bytes: &[u8]) -> Result<String, StorageError> {
    if bytes.len() > MAX_PICTURE {
        return Err(StorageError::Invalid("The picture is larger than 20 MB.".into()));
    }
    if !loaded(app)?.0.iter().any(|p| p.homebrew && p.manifest.id == book) {
        return Err(StorageError::NotFound(format!("There's no homebrew book \"{book}\".")));
    }
    let png = images::picked_png(bytes)?;
    let name = picture_name(&png);
    let dir = data_dir(app)?.join(images::DIR).join(book);
    std::fs::create_dir_all(&dir)?;
    write_atomic_bytes(&images::file_for(&dir, &name), &png)?;
    Ok(name)
}

/// The Homebrew tab: every valid homebrew book, switched on or off.
#[tauri::command]
pub async fn list_homebrew(app: AppHandle) -> Result<Vec<Book>, StorageError> {
    blocking(move || Ok(loaded(&app)?.0.iter().filter(|p| p.homebrew).map(Book::from).collect())).await
}

/// Saves a whole book (a new one when its id is empty). Refused with the pack's errors when it doesn't validate; the
/// last good book stays.
#[tauri::command]
pub async fn save_homebrew(app: AppHandle, book: Book) -> Result<Book, StorageError> {
    blocking(move || {
        let _guard = lock_pack_change();
        let current = loaded(&app)?;
        let dir = homebrew_dir(&app)?;
        let taken = |id: &str| {
            current.0.iter().any(|p| p.manifest.id == id) || current.1.iter().any(|i| i.folder == id) || dir.join(id).exists()
        };
        let created = book.id.is_empty();
        let new = prepare(book, &current.0, &taken)?;
        let pack = packs::install(&dir, &homebrew_staging(&app)?, &new)?;
        // Saved: from here on the rest of the app sees the new book, whatever fails after.
        reload();
        // shortcut: a picture that can't be deleted now (a file Windows has locked) stays until a later save removes it.
        let _ = prune_pictures(&data_dir(&app)?.join(images::DIR).join(&pack.manifest.id), &pictures_in(&pack.entries));
        // A new book is on, also when a deleted one with its id had been switched off.
        let off = disabled(&app)?;
        if created && off.contains(&pack.manifest.id) {
            let left: Vec<&String> = off.iter().filter(|id| **id != pack.manifest.id).collect();
            settings::update(&data_dir(&app)?.join(settings::FILE_NAME), json!({ "disabledPacks": left }))?;
        }
        Ok(Book::from(&pack))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(id: &str, name: &str) -> Book {
        Book {
            id: id.into(),
            name: name.into(),
            version: "1.0.0".into(),
            license: "Homebrew".into(),
            attribution: String::new(),
            ruleset: "2024".into(),
            entries: BTreeMap::new(),
        }
    }

    fn pack(id: &str, name: &str, homebrew: bool) -> Pack {
        let manifest = serde_json::from_value(json!({ "schema": 1, "id": id, "name": name, "version": "1", "license": "MIT",
            "attribution": "", "ruleset": "2024", "files": [] }))
        .unwrap();
        Pack { manifest, entries: BTreeMap::new(), images: None, installed: homebrew, homebrew }
    }

    #[test]
    fn choices_are_entries_or_the_values_a_field_has() {
        let mut p = pack("srd", "SRD", false);
        let spells = json!([
            { "index": "b", "name": "Bolt", "components": ["V", "S"], "school": { "name": "Evocation" } },
            { "index": "a", "name": "acid", "components": ["V", "M"], "school": { "name": "Conjuration" } },
        ]);
        p.entries.insert("spells".into(), spells.as_array().unwrap().iter().map(|v| v.as_object().unwrap().clone()).collect());
        let names = |list: Vec<Choice>| list.into_iter().map(|c| c.name).collect::<Vec<_>>();
        assert_eq!(names(choices(&[&p], "spells", None)), ["acid", "Bolt"]);
        assert_eq!(names(choices(&[&p], "spells", Some("components"))), ["M", "S", "V"]);
        assert_eq!(names(choices(&[&p], "spells", Some("school.name"))), ["Conjuration", "Evocation"]);
        assert!(choices(&[&p], "feats", None).is_empty());
    }

    #[test]
    fn pictures_no_entry_names_are_removed() {
        let dir = std::env::temp_dir().join(format!("koldron-homebrew-pictures-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["homebrew-a.png", "homebrew-b.png"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        let mut entries = BTreeMap::new();
        entries.insert("monsters".into(), vec![json!({ "index": "m", "name": "M", "image": "homebrew-a.png" }).as_object().unwrap().clone()]);
        prune_pictures(&dir, &pictures_in(&entries)).unwrap();
        assert!(dir.join("homebrew-a.png").exists() && !dir.join("homebrew-b.png").exists());
        prune_pictures(&dir.join("none"), &pictures_in(&entries)).unwrap(); // no folder yet
        assert_eq!(picture_name(b"same"), picture_name(b"same"));
    }

    #[test]
    fn slugs_are_pack_ids() {
        assert_eq!(slug("  The Sunless Citadel: Lore! "), "the-sunless-citadel-lore");
        assert_eq!(slug("¿?"), "book");
    }

    #[test]
    fn a_new_book_gets_a_free_id_and_an_empty_chapter_list() {
        let packs = [pack("srd", "SRD 5.2", false), pack("my-lore", "Old lore", true)];
        let taken = |id: &str| packs.iter().any(|p| p.manifest.id == id);
        let new = prepare(book("", " My lore "), &packs, &taken).unwrap();
        assert_eq!(new.manifest["id"], "my-lore-2");
        assert_eq!(new.manifest["name"], "My lore");
        assert_eq!(new.manifest["files"], json!([{ "type": "rules", "path": "data/rules.json" }]));
        assert_eq!(new.files["data/rules.json"], json!([]));
    }

    #[test]
    fn a_save_is_refused_for_a_bad_name_an_unknown_book_or_type() {
        let packs = [pack("srd", "SRD 5.2", false), pack("lore", "Lore", true)];
        let taken = |_: &str| false;
        let refused = |b: Book| prepare(b, &packs, &taken).is_err();
        assert!(refused(book("", "  ")));
        assert!(refused(book("", &"x".repeat(101))));
        assert!(refused(book("", "srd 5.2"))); // the same name ignoring case
        assert!(refused(book("srd", "Mine"))); // not a homebrew book
        let mut odd = book("lore", "Lore");
        odd.entries.insert("../x".into(), vec![Map::new()]);
        assert!(refused(odd));
        assert!(!refused(book("lore", "Lore"))); // its own name
    }

    #[test]
    fn a_saved_book_installs_and_loads_as_a_homebrew_pack() {
        let dir = std::env::temp_dir().join(format!("koldron-homebrew-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut b = book("", "Lore");
        let section = json!({ "index": "the-city", "name": "The city", "desc": "**Big.**" });
        let chapter = json!({ "index": "world", "name": "World", "desc": "", "subsections": [{ "index": "the-city" }] });
        b.entries.insert("rule-sections".into(), vec![section.as_object().unwrap().clone()]);
        b.entries.insert("rules".into(), vec![chapter.as_object().unwrap().clone()]);
        let new = prepare(b, &[], &|_| false).unwrap();
        packs::install(&dir.join("homebrew"), &dir.join("staging"), &new).unwrap();
        let (loaded, invalid) = packs::load_all(&dir.join("bundled"), &dir.join("installed"), &dir.join("homebrew")).unwrap();
        assert!(invalid.is_empty(), "{invalid:?}");
        let saved = Book::from(&loaded[0]);
        assert!(loaded[0].homebrew);
        assert_eq!((saved.id.as_str(), saved.entries["rules"].len(), saved.entries["rule-sections"].len()), ("lore", 1, 1));

        // Exported, it's a pack .zip with its files at the top.
        let bytes = zip_book(&dir.join("homebrew").join("lore")).unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut names: Vec<String> = (0..archive.len()).map(|i| archive.by_index(i).unwrap().name().unwrap().to_string()).collect();
        names.sort();
        assert_eq!(names, ["data/rule-sections.json", "data/rules.json", "pack.json"]);

        // A chapter naming a section the book doesn't have is refused, and the saved book stays.
        let mut broken = saved;
        broken.entries.remove("rule-sections");
        let new = prepare(broken, &loaded, &|_| false).unwrap();
        assert!(packs::install(&dir.join("homebrew"), &dir.join("staging"), &new).is_err());
        let (loaded, _) = packs::load_all(&dir.join("bundled"), &dir.join("installed"), &dir.join("homebrew")).unwrap();
        assert_eq!(loaded[0].entries["rule-sections"].len(), 1);
    }
}
