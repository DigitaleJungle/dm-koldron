//! Encounters: one file per encounter in its campaign's folder, `encounters/<id>.json`. Only active campaigns are
//! changed (an archived one is read-only). Its map picture sits beside it as `encounters/<id>.map.jpg`. The XP budgets for the difficulty come from the switched-on packs.

use crate::commands::campaigns::{active_dir, dirs, now_ms, picture_at, store_picture, Picture};
use crate::commands::packs::{disabled, enabled, loaded};
use crate::commands::sessions;
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
pub const MAP_MAX_SIDE: u32 = 2048;

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
    check_pins(&notes.pins, &mut problems);
    if problems.is_empty() { Ok(()) } else { Err(StorageError::Invalid(problems.join(" "))) }
}

/// A map's pins: titles not too long, not too many, all on the picture. (Their texts are checked with the notes.)
pub fn check_pins(pins: &[Pin], problems: &mut Vec<String>) {
    if pins.iter().any(|p| p.title.chars().count() > MAX_NAME_LEN) {
        problems.push(format!("A pin's title must be at most {MAX_NAME_LEN} characters."));
    }
    if pins.len() > MAX_PINS {
        problems.push(format!("A map can have at most {MAX_PINS} pins."));
    }
    if pins.iter().any(|p| !(0.0..=1.0).contains(&p.x) || !(0.0..=1.0).contains(&p.y)) {
        problems.push("A pin must be on the map.".into());
    }
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

/// Deletes the encounter with its map and run, and unlinks it from every session's scenes.
pub fn delete(campaign_dir: &Path, id: &str) -> Result<(), StorageError> {
    check_id(id, "encounter")?;
    let folder = campaign_dir.join(FOLDER);
    let _guard = lock_update();
    read(&folder, id)?; // only what exists, and never a file that can't be read
    // Unlinked first: if a session can't be written, the encounter is still there to delete again.
    // shortcut: a session file that can't be read keeps its link; the scenes only show encounters that exist.
    // The caller has run world::migrate, so writing a session back drops no locations or NPCs of its own.
    for mut session in sessions::read_all(campaign_dir)? {
        let before: usize = session.scenes.iter().map(|sc| sc.encounter_ids.len()).sum();
        session.scenes.iter_mut().for_each(|sc| sc.encounter_ids.retain(|e| e != id));
        if session.scenes.iter().map(|sc| sc.encounter_ids.len()).sum::<usize>() != before {
            sessions::write(campaign_dir, &session)?;
        }
    }
    fs::remove_file(folder.join(format!("{id}.json")))?;
    for extra in [map_path(&folder, id), run_path(&folder, id)] {
        match fs::remove_file(extra) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        }
    }
    Ok(())
}

const MAX_COMBATANTS: usize = 1000;
const MAX_ID_LEN: usize = 64;
const MAX_CONDITIONS: usize = 50;
const MAX_LOG: usize = 1000;
const MAX_LOG_TEXT: usize = 500;
const DEATH_SAVES: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CombatantKind {
    Monster,
    Character,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    pub index: String,
    pub name: String,
}

/// One line of the encounter log: the round and whose turn it was, and what happened.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub round: u32,
    pub actor: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Combatant {
    pub id: String,
    pub kind: CombatantKind,
    #[serde(default)]
    pub pack: String,
    #[serde(default)]
    pub index: String,
    #[serde(default)]
    pub character: String,
    #[serde(default)]
    pub group: String,
    pub name: String,
    #[serde(default)]
    pub initiative: Option<i32>,
    #[serde(default)]
    pub initiative_bonus: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub temp_hp: i32,
    #[serde(default)]
    pub ac: Option<i32>,
    #[serde(default)]
    pub concentrating: bool,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// A downed character's death saving throws, 0 to 3 each.
    #[serde(default)]
    pub death_successes: u8,
    #[serde(default)]
    pub death_failures: u8,
}

/// A fight in progress, saved beside its encounter as `encounters/<id>.run.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    /// 0 = setup, before Start.
    pub round: u32,
    /// Index into `combatants`.
    pub turn: u32,
    pub combatants: Vec<Combatant>,
    /// Newest first.
    #[serde(default)]
    pub log: Vec<LogEntry>,
    /// Milliseconds since the Unix epoch; set by `save_run`.
    #[serde(default)]
    pub saved_at: u64,
}

fn check_run(run: &mut Run) -> Result<(), StorageError> {
    let mut problems = Vec::new();
    if run.combatants.len() > MAX_COMBATANTS {
        problems.push(format!("A fight can have at most {MAX_COMBATANTS} combatants."));
    }
    let mut ids = std::collections::HashSet::new();
    for c in &mut run.combatants {
        c.name = c.name.trim().into();
        if !(1..=MAX_ID_LEN).contains(&c.id.chars().count()) || !ids.insert(c.id.clone()) {
            problems.push("Each combatant needs its own id of 1 to 64 characters.".into());
        }
        if !(1..=MAX_NAME_LEN).contains(&c.name.chars().count()) {
            problems.push(format!("A name must be 1 to {MAX_NAME_LEN} characters."));
        }
        if c.hp < 0 || c.max_hp < 0 || c.temp_hp < 0 {
            problems.push("HP can't be negative.".into());
        }
        if c.kind == CombatantKind::Monster && (c.pack.is_empty() || c.index.is_empty()) {
            problems.push("A monster needs its pack and index.".into());
        }
        if c.kind == CombatantKind::Character && c.character.is_empty() {
            problems.push("A character combatant needs its character.".into());
        }
        if c.conditions.len() > MAX_CONDITIONS {
            problems.push(format!("A combatant can have at most {MAX_CONDITIONS} conditions."));
        }
        if c.conditions.iter().any(|k| !(1..=MAX_ID_LEN).contains(&k.index.chars().count()) || !(1..=MAX_NAME_LEN).contains(&k.name.chars().count())) {
            problems.push("A condition needs an index and a name.".into());
        }
    }
    if run.combatants.iter().any(|c| c.death_successes > DEATH_SAVES || c.death_failures > DEATH_SAVES) {
        problems.push(format!("Death saves go up to {DEATH_SAVES}."));
    }
    if run.log.len() > MAX_LOG {
        problems.push(format!("The log can have at most {MAX_LOG} entries."));
    }
    if run.log.iter().any(|l| l.actor.chars().count() > MAX_NAME_LEN || !(1..=MAX_LOG_TEXT).contains(&l.text.chars().count())) {
        problems.push(format!("A log entry must be 1 to {MAX_LOG_TEXT} characters."));
    }
    if run.turn as usize >= run.combatants.len().max(1) {
        problems.push("The turn must point at a combatant.".into());
    }
    problems.dedup();
    if problems.is_empty() { Ok(()) } else { Err(StorageError::Invalid(problems.join(" "))) }
}

fn run_path(folder: &Path, id: &str) -> PathBuf {
    folder.join(format!("{id}.run.json"))
}

pub fn get_run_at(campaign_dir: &Path, id: &str) -> Result<Option<Run>, StorageError> {
    check_id(id, "encounter")?;
    let folder = campaign_dir.join(FOLDER);
    read(&folder, id)?; // NotFound when the encounter itself is gone
    match fs::read_to_string(run_path(&folder, id)) {
        Ok(text) => serde_json::from_str(&text).map(Some).map_err(|e| StorageError::Corrupt(e.to_string())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn save_run_at(campaign_dir: &Path, id: &str, mut run: Run, now: u64) -> Result<Run, StorageError> {
    check_id(id, "encounter")?;
    check_run(&mut run)?;
    let folder = campaign_dir.join(FOLDER);
    let _guard = lock_update();
    read(&folder, id)?;
    run.saved_at = now;
    write_atomic(&run_path(&folder, id), &to_json(&run)?)?;
    Ok(run)
}

pub fn end_run_at(campaign_dir: &Path, id: &str) -> Result<(), StorageError> {
    check_id(id, "encounter")?;
    let _guard = lock_update();
    match fs::remove_file(run_path(&campaign_dir.join(FOLDER), id)) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
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
pub async fn delete_encounter(app: AppHandle, campaign: String, id: String) -> Result<(), StorageError> {
    // Migrated first (outside delete's lock, as migrate takes it too): older sessions still hold their own
    // locations and NPCs, which writing them back would drop.
    blocking(move || delete(&sessions::migrated(&app, &campaign)?, &id)).await
}

#[tauri::command]
pub async fn set_encounter_map(app: AppHandle, campaign: String, id: String, path: Option<String>) -> Result<Option<Picture>, StorageError> {
    blocking(move || set_map(&active_dir(&dirs(&app)?, &campaign)?, &id, path.as_deref().map(Path::new))).await
}

#[tauri::command]
pub async fn encounter_budgets(app: AppHandle) -> Result<Vec<Budget>, StorageError> {
    blocking(move || Ok(budgets(&enabled(&loaded(&app)?.0, &disabled(&app)?)))).await
}

#[tauri::command]
pub async fn get_run(app: AppHandle, campaign: String, encounter: String) -> Result<Option<Run>, StorageError> {
    blocking(move || get_run_at(&active_dir(&dirs(&app)?, &campaign)?, &encounter)).await
}

#[tauri::command]
pub async fn save_run(app: AppHandle, campaign: String, encounter: String, run: Run) -> Result<Run, StorageError> {
    blocking(move || save_run_at(&active_dir(&dirs(&app)?, &campaign)?, &encounter, run, now_ms())).await
}

#[tauri::command]
pub async fn end_run(app: AppHandle, campaign: String, encounter: String) -> Result<(), StorageError> {
    blocking(move || end_run_at(&active_dir(&dirs(&app)?, &campaign)?, &encounter)).await
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

    fn combatant(id: &str) -> Combatant {
        Combatant { id: id.into(), kind: CombatantKind::Monster, pack: "srd".into(), index: "goblin-warrior".into(), character: String::new(), group: String::new(), name: "Goblin".into(), initiative: Some(12), initiative_bonus: 2, hp: 7, max_hp: 7, temp_hp: 0, ac: Some(15), concentrating: false, conditions: vec![], death_successes: 0, death_failures: 0 }
    }

    fn run_of(combatants: Vec<Combatant>) -> Run {
        Run { round: 1, turn: 0, combatants, log: vec![LogEntry { round: 1, actor: "Goblin".into(), text: "Lysa took 5 damage.".into() }], saved_at: 0 }
    }

    #[test]
    fn saves_and_reads_a_run_ends_it_and_list_ignores_it() {
        let dir = temp_dir("run");
        let e = save(&dir, encounter("Bridge", vec![goblins(2)]), 1).unwrap();
        assert_eq!(get_run_at(&dir, &e.id).unwrap(), None);
        end_run_at(&dir, &e.id).unwrap(); // nothing to remove is fine
        let mut first = combatant("a");
        first.conditions = vec![Condition { index: "prone".into(), name: "Prone".into() }];
        let saved = save_run_at(&dir, &e.id, run_of(vec![first, combatant("b")]), 55).unwrap();
        assert_eq!(saved.saved_at, 55);
        assert_eq!(get_run_at(&dir, &e.id).unwrap(), Some(saved));
        let listed = list(&dir).unwrap();
        assert_eq!((listed.encounters.len(), listed.unreadable.len()), (1, 0));
        end_run_at(&dir, &e.id).unwrap();
        assert_eq!(get_run_at(&dir, &e.id).unwrap(), None);
    }

    #[test]
    fn deletes_with_its_map_and_run_and_unlinks_it_from_scenes() {
        let dir = temp_dir("delete");
        let gone = save(&dir, encounter("Bridge", vec![goblins(2)]), 1).unwrap();
        let kept = save(&dir, encounter("Crypt", vec![goblins(1)]), 2).unwrap();
        save_run_at(&dir, &gone.id, run_of(vec![combatant("a")]), 3).unwrap();
        fs::write(map_path(&dir.join(FOLDER), &gone.id), b"jpg").unwrap();
        let session: sessions::Session = serde_json::from_value(serde_json::json!({
            "id": "s1", "number": 1, "title": "Vault",
            "scenes": [{ "id": "a", "encounterIds": [gone.id, kept.id] }, { "id": "b", "encounterIds": [gone.id] }]
        }))
        .unwrap();
        fs::create_dir_all(sessions::folder(&dir)).unwrap();
        sessions::write(&dir, &session).unwrap();

        delete(&dir, &gone.id).unwrap();
        let left: Vec<String> = list(&dir).unwrap().encounters.into_iter().map(|e| e.id).collect();
        assert_eq!(left, vec![kept.id.clone()]);
        let folder = dir.join(FOLDER);
        assert!(!map_path(&folder, &gone.id).exists() && !run_path(&folder, &gone.id).exists());
        let links: Vec<Vec<String>> = sessions::read_all(&dir).unwrap()[0].scenes.iter().map(|sc| sc.encounter_ids.clone()).collect();
        assert_eq!(links, vec![vec![kept.id], vec![]]);
        assert!(matches!(delete(&dir, &gone.id), Err(StorageError::NotFound(_))));
        assert!(matches!(delete(&dir, "../x"), Err(StorageError::Invalid(_))));
    }

    #[test]
    fn rejects_bad_runs_unknown_encounters_and_keeps_a_damaged_run() {
        let dir = temp_dir("run-rules");
        let e = save(&dir, encounter("Bridge", vec![]), 1).unwrap();
        let bad = |run: Run| matches!(save_run_at(&dir, &e.id, run, 2), Err(StorageError::Invalid(_)));
        assert!(bad(run_of(vec![combatant("a"), combatant("a")])));
        assert!(bad(run_of(vec![Combatant { name: " ".into(), ..combatant("a") }])));
        assert!(bad(Run { turn: 1, ..run_of(vec![combatant("a")]) }));
        assert!(bad(Run { turn: 1, ..run_of(vec![]) }));
        assert!(bad(run_of(vec![Combatant { pack: String::new(), ..combatant("a") }])));
        assert!(bad(run_of(vec![Combatant { hp: -1, ..combatant("a") }])));
        assert!(bad(run_of(vec![Combatant { death_failures: 4, ..combatant("a") }])));
        let empty_note = LogEntry { round: 1, actor: String::new(), text: String::new() };
        assert!(bad(Run { log: vec![empty_note], ..run_of(vec![combatant("a")]) }));
        // Older run files, with condition durations and no log or death saves, still read.
        let older = r#"{"round":1,"turn":0,"combatants":[{"id":"a","kind":"custom","name":"Ally","hp":5,"maxHp":5,"tempHp":0,
            "conditions":[{"index":"prone","name":"Prone","endsRound":3}]}]}"#;
        let read: Run = serde_json::from_str(older).unwrap();
        assert_eq!((read.log.len(), read.combatants[0].death_failures), (0, 0));
        assert!(matches!(get_run_at(&dir, "nope"), Err(StorageError::NotFound(_))));
        assert!(matches!(save_run_at(&dir, "nope", run_of(vec![]), 2), Err(StorageError::NotFound(_))));
        let path = dir.join(FOLDER).join(format!("{}.run.json", e.id));
        fs::write(&path, "{ nope").unwrap();
        assert!(matches!(get_run_at(&dir, &e.id), Err(StorageError::Corrupt(_))));
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ nope");
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
