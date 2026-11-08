//! Encounters: one file per encounter in its campaign's folder, `encounters/<id>.json`. Only active campaigns are
//! changed (an archived one is read-only). Its map picture sits beside it as `encounters/<id>.map.jpg`. The XP budgets for the difficulty come from the switched-on packs.

use crate::commands::campaigns::{active_dir, dirs, now_ms, picture_at, store_picture, Picture};
use crate::commands::packs::{disabled, enabled, loaded};
use crate::packs::Pack;
use crate::storage::{blocking, check_id, lock_update, to_json, write_atomic, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const FOLDER: &str = "encounters";
const MAX_NAME_LEN: usize = 100;
const MAX_QUANTITY: u32 = 999;
const MAX_NOTE_LEN: usize = 20_000;
const MAX_PINS: usize = 100;
/// The map is stored shrunk so its longest side is at most this many pixels.
const MAP_MAX_SIDE: u32 = 2048;

/// One roster row: a pack's monster and how many. The name and source are kept so the row still reads when its pack
/// is switched off or gone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterEntry {
    pub pack: String,
    pub index: String,
    pub name: String,
    pub source: String,
    pub quantity: u32,
}

/// The DM's notes: a text (shown as "Notes"; stored as its first name, `description`) and the pins on the map.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Notes {
    pub description: String,
    pub pins: Vec<Pin>,
}

/// A pin on the map: where (0 to 1 across and down the picture), its title and its text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pin {
    pub x: f64,
    pub y: f64,
    /// Pins saved before there were titles read with none.
    #[serde(default)]
    pub title: String,
    pub text: String,
}

/// Draft while it's being prepared; Ready locks it in Prepare and lets it be run.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Draft,
    Ready,
}

/// Who fights: the campaign's party as it is at the time (Party), or the encounter's own pick (Custom).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PartyMode {
    Party,
    #[default]
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Encounter {
    /// Empty when saving a new encounter; set by `save`.
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub monsters: Vec<RosterEntry>,
    /// The ids of the characters in the party.
    pub party: Vec<String>,
    /// Files saved before there was a status read as Draft.
    #[serde(default)]
    pub status: Status,
    /// Files saved before there was a mode read as Custom: they keep their own `party`. In Party mode `party` is
    /// still kept (the Custom pick to go back to).
    #[serde(default)]
    pub party_mode: PartyMode,
    /// Files saved before there were notes read with none.
    #[serde(default)]
    pub notes: Notes,
    /// The map picture, found beside the file when it's read; never saved in it or taken from the window.
    #[serde(default, skip_deserializing)]
    pub map: Option<Picture>,
    /// Milliseconds since the Unix epoch; set by `save`.
    #[serde(default)]
    pub saved_at: u64,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct EncounterList {
    /// Most recently saved first.
    pub encounters: Vec<Encounter>,
    /// Files that couldn't be read as an encounter. They are left untouched.
    pub unreadable: Vec<String>,
}

/// XP per character of one level, for the three 2024 difficulties.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    pub level: u32,
    pub low: u64,
    pub moderate: u64,
    pub high: u64,
}

fn check(encounter: &mut Encounter) -> Result<(), StorageError> {
    encounter.name = encounter.name.trim().into();
    let mut problems = Vec::new();
    if encounter.name.is_empty() {
        problems.push("A name is required.".to_string());
    } else if encounter.name.chars().count() > MAX_NAME_LEN {
        problems.push(format!("The name must be at most {MAX_NAME_LEN} characters."));
    }
    if encounter.monsters.iter().any(|m| !(1..=MAX_QUANTITY).contains(&m.quantity)) {
        problems.push(format!("A quantity must be 1 to {MAX_QUANTITY}."));
    }
    if encounter.monsters.iter().any(|m| m.pack.is_empty() || m.index.is_empty()) {
        problems.push("Each monster needs its pack and index.".into());
    }
    let notes = &encounter.notes;
    let mut texts = std::iter::once(&notes.description).chain(notes.pins.iter().map(|p| &p.text));
    if texts.any(|t| t.chars().count() > MAX_NOTE_LEN) {
        problems.push(format!("A note must be at most {MAX_NOTE_LEN} characters."));
    }
    if notes.pins.iter().any(|p| p.title.chars().count() > MAX_NAME_LEN) {
        problems.push(format!("A pin's title must be at most {MAX_NAME_LEN} characters."));
    }
    if notes.pins.len() > MAX_PINS {
        problems.push(format!("A map can have at most {MAX_PINS} pins."));
    }
    if notes.pins.iter().any(|p| !(0.0..=1.0).contains(&p.x) || !(0.0..=1.0).contains(&p.y)) {
        problems.push("A pin must be on the map.".into());
    }
    if problems.is_empty() { Ok(()) } else { Err(StorageError::Invalid(problems.join(" "))) }
}

fn read(folder: &Path, id: &str) -> Result<Encounter, StorageError> {
    let text = fs::read_to_string(folder.join(format!("{id}.json"))).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => StorageError::NotFound("That encounter doesn't exist.".into()),
        _ => e.into(),
    })?;
    let mut encounter: Encounter = serde_json::from_str(&text).map_err(|e| StorageError::Corrupt(e.to_string()))?;
    encounter.id = id.into(); // the file name is the id
    encounter.map = picture_at(&map_path(folder, id));
    Ok(encounter)
}

fn map_path(folder: &Path, id: &str) -> PathBuf {
    folder.join(format!("{id}.map.jpg"))
}

pub fn list(campaign_dir: &Path) -> Result<EncounterList, StorageError> {
    let folder = campaign_dir.join(FOLDER);
    let entries = match fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(EncounterList { encounters: vec![], unreadable: vec![] }),
        Err(e) => return Err(e.into()),
    };
    let (mut encounters, mut unreadable) = (Vec::new(), Vec::new());
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = name.strip_suffix(".json").filter(|id| check_id(id, "encounter").is_ok()) else { continue };
        match read(&folder, id) {
            Ok(e) => encounters.push(e),
            Err(StorageError::Corrupt(_)) => unreadable.push(name),
            Err(e) => return Err(e),
        }
    }
    encounters.sort_by(|a, b| b.saved_at.cmp(&a.saved_at).then(a.name.cmp(&b.name)));
    unreadable.sort();
    Ok(EncounterList { encounters, unreadable })
}

pub fn get(campaign_dir: &Path, id: &str) -> Result<Encounter, StorageError> {
    check_id(id, "encounter")?;
    read(&campaign_dir.join(FOLDER), id)
}

/// Creates the encounter (empty `id`) or replaces the saved one.
pub fn save(campaign_dir: &Path, mut encounter: Encounter, now: u64) -> Result<Encounter, StorageError> {
    check(&mut encounter)?;
    let folder = campaign_dir.join(FOLDER);
    let _guard = lock_update();
    if encounter.id.is_empty() {
        let base = format!("{now:x}");
        let (mut id, mut n) = (base.clone(), 1);
        while folder.join(format!("{id}.json")).exists() {
            n += 1;
            id = format!("{base}-{n}");
        }
        encounter.id = id;
    } else {
        check_id(&encounter.id, "encounter")?;
        read(&folder, &encounter.id)?; // replacing only what exists, and never a file that can't be read
    }
    encounter.saved_at = now;
    write_atomic(&folder.join(format!("{}.json", encounter.id)), &to_json(&Encounter { map: None, ..encounter.clone() })?)?;
    encounter.map = picture_at(&map_path(&folder, &encounter.id));
    Ok(encounter)
}

/// Sets a saved encounter's map from a picture file (shrunk, stored as JPEG), or removes it (`None`).
pub fn set_map(campaign_dir: &Path, id: &str, source: Option<&Path>) -> Result<Option<Picture>, StorageError> {
    check_id(id, "encounter")?;
    let folder = campaign_dir.join(FOLDER);
    read(&folder, id)?;
    let path = map_path(&folder, id);
    match source {
        Some(source) => store_picture(source, MAP_MAX_SIDE, &path)?,
        None => match fs::remove_file(&path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        },
    }
    Ok(picture_at(&path))
}

/// The budget table of the first switched-on pack that has one (a ruleset's pack); empty when none has.
pub fn budgets(packs: &[&Pack]) -> Vec<Budget> {
    let Some(rows) = packs.iter().find_map(|p| p.entries.get("encounter-budgets")) else { return Vec::new() };
    // The loader checked every row's level and three whole numbers.
    let mut out: Vec<Budget> = rows.iter().filter_map(|r| serde_json::from_value(Value::Object(r.clone())).ok()).collect();
    out.sort_by_key(|b| b.level);
    out
}

#[tauri::command]
pub async fn list_encounters(app: AppHandle, campaign: String) -> Result<EncounterList, StorageError> {
    blocking(move || list(&active_dir(&dirs(&app)?, &campaign)?)).await
}

#[tauri::command]
pub async fn get_encounter(app: AppHandle, campaign: String, id: String) -> Result<Encounter, StorageError> {
    blocking(move || get(&active_dir(&dirs(&app)?, &campaign)?, &id)).await
}

#[tauri::command]
pub async fn save_encounter(app: AppHandle, campaign: String, encounter: Encounter) -> Result<Encounter, StorageError> {
    blocking(move || save(&active_dir(&dirs(&app)?, &campaign)?, encounter, now_ms())).await
}

/// `path`: the picture the DM chose, or none to remove the map.
#[tauri::command]
pub async fn set_encounter_map(app: AppHandle, campaign: String, id: String, path: Option<String>) -> Result<Option<Picture>, StorageError> {
    blocking(move || set_map(&active_dir(&dirs(&app)?, &campaign)?, &id, path.as_deref().map(Path::new))).await
}

#[tauri::command]
pub async fn encounter_budgets(app: AppHandle) -> Result<Vec<Budget>, StorageError> {
    blocking(move || Ok(budgets(&enabled(&loaded(&app)?.0, &disabled(&app)?)))).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("koldron-encounters-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn goblins(quantity: u32) -> RosterEntry {
        RosterEntry { pack: "srd".into(), index: "goblin-warrior".into(), name: "Goblin Warrior".into(), source: "SRD".into(), quantity }
    }

    fn encounter(name: &str, monsters: Vec<RosterEntry>) -> Encounter {
        Encounter { id: String::new(), name: name.into(), monsters, party: vec!["a".into()], status: Status::Draft, party_mode: PartyMode::Custom, notes: Notes::default(), map: None, saved_at: 0 }
    }

    #[test]
    fn keeps_notes_checks_pins_and_stores_the_map_beside_the_file() {
        let dir = temp_dir("notes");
        let pin = |x: f64| Pin { x, y: 0.5, title: "Trapdoor".into(), text: "Leads to the crypt".into() };
        let notes = Notes { description: "Flee at half HP".into(), pins: vec![pin(0.25)] };
        let saved = save(&dir, Encounter { notes: notes.clone(), ..encounter("Bridge", vec![]) }, 1).unwrap();
        assert_eq!(get(&dir, &saved.id).unwrap().notes, notes);
        let off = Notes { pins: vec![pin(1.5)], ..Notes::default() };
        assert!(matches!(save(&dir, Encounter { notes: off, ..saved.clone() }, 2), Err(StorageError::Invalid(_))));
        let long = Notes { pins: vec![Pin { title: "x".repeat(MAX_NAME_LEN + 1), ..pin(0.5) }], ..Notes::default() };
        assert!(matches!(save(&dir, Encounter { notes: long, ..saved.clone() }, 2), Err(StorageError::Invalid(_))));
        let old: Pin = serde_json::from_str(r#"{"x":0.1,"y":0.2,"text":"Old"}"#).unwrap();
        assert_eq!(old.title, "");

        let png = dir.join("map.png");
        image::RgbImage::new(4000, 1000).save(&png).unwrap();
        let map = set_map(&dir, &saved.id, Some(&png)).unwrap().unwrap();
        let stored = image::open(&map.path).unwrap();
        assert_eq!((stored.width(), stored.height()), (MAP_MAX_SIDE, 512));
        let resaved = save(&dir, saved.clone(), 3).unwrap();
        assert_eq!(resaved.map.map(|m| m.path), Some(map.path.clone()));
        let file = dir.join(FOLDER).join(format!("{}.json", saved.id));
        assert!(!fs::read_to_string(&file).unwrap().contains("map.jpg"));
        assert!(matches!(set_map(&dir, &saved.id, Some(&file)), Err(StorageError::Invalid(_))));
        assert_eq!(set_map(&dir, &saved.id, None).unwrap(), None);
        assert!(matches!(set_map(&dir, "nope", None), Err(StorageError::NotFound(_))));
    }

    #[test]
    fn keeps_the_status_and_reads_older_files_as_draft() {
        let dir = temp_dir("status");
        let ready = save(&dir, Encounter { status: Status::Ready, ..encounter("Bridge", vec![goblins(1)]) }, 1).unwrap();
        assert_eq!(get(&dir, &ready.id).unwrap().status, Status::Ready);
        fs::write(dir.join(FOLDER).join("old.json"), r#"{"name":"Old","monsters":[],"party":[]}"#).unwrap();
        assert_eq!(get(&dir, "old").unwrap().status, Status::Draft);
    }

    #[test]
    fn keeps_the_party_mode_and_reads_older_files_as_custom() {
        let dir = temp_dir("party-mode");
        let party = save(&dir, Encounter { party_mode: PartyMode::Party, ..encounter("Bridge", vec![goblins(1)]) }, 1).unwrap();
        assert_eq!(get(&dir, &party.id).unwrap().party_mode, PartyMode::Party);
        fs::write(dir.join(FOLDER).join("old.json"), r#"{"name":"Old","monsters":[],"party":["x"]}"#).unwrap();
        let old = get(&dir, "old").unwrap();
        assert_eq!((old.party_mode, old.party), (PartyMode::Custom, vec!["x".to_string()]));
    }

    #[test]
    fn saves_new_and_existing_encounters_and_lists_the_latest_first() {
        let dir = temp_dir("save");
        let first = save(&dir, encounter(" Bridge ", vec![goblins(4)]), 10).unwrap();
        assert_eq!((first.id.as_str(), first.name.as_str(), first.saved_at), ("a", "Bridge", 10));
        let second = save(&dir, encounter("Cellar", vec![]), 11).unwrap();
        assert_eq!(list(&dir).unwrap().encounters.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["Cellar", "Bridge"]);
        save(&dir, Encounter { name: "Bridge ambush".into(), ..first }, 20).unwrap();
        let all = list(&dir).unwrap().encounters;
        assert_eq!((all[0].name.as_str(), all[0].monsters[0].quantity, all.len()), ("Bridge ambush", 4, 2));
        assert_eq!(get(&dir, &second.id).unwrap(), second);
    }

    #[test]
    fn rejects_broken_rows_unknown_ids_and_keeps_damaged_files() {
        let dir = temp_dir("rules");
        let Err(StorageError::Invalid(all)) = save(&dir, encounter(" ", vec![goblins(0)]), 1) else { panic!() };
        assert!(all.contains("name is required") && all.contains("quantity must be 1 to 999"), "{all}");
        let ghost = Encounter { id: "nope".into(), ..encounter("A", vec![]) };
        assert!(matches!(save(&dir, ghost, 1), Err(StorageError::NotFound(_))));
        assert!(matches!(get(&dir, "../x"), Err(StorageError::Invalid(_))));
        fs::create_dir_all(dir.join(FOLDER)).unwrap();
        fs::write(dir.join(FOLDER).join("b.json"), "{ nope").unwrap();
        assert_eq!(list(&dir).unwrap().unreadable, ["b.json"]);
        let damaged = Encounter { id: "b".into(), ..encounter("A", vec![]) };
        assert!(matches!(save(&dir, damaged, 1), Err(StorageError::Corrupt(_))));
        assert_eq!(fs::read_to_string(dir.join(FOLDER).join("b.json")).unwrap(), "{ nope");
    }

    #[test]
    fn the_srd_pack_brings_the_2024_budgets() {
        let (packs, _) = crate::packs::load_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs")).unwrap();
        let table = budgets(&packs.iter().collect::<Vec<_>>());
        assert_eq!(table.len(), 20);
        assert_eq!(table[0], Budget { level: 1, low: 50, moderate: 75, high: 100 });
        assert!(budgets(&[]).is_empty());
    }
}
