//! Encounters: one file per encounter in its campaign's folder, `encounters/<id>.json`. Only active campaigns are
//! changed (an archived one is read-only). The XP budgets for the difficulty come from the switched-on packs.

use crate::commands::campaigns::{active_dir, dirs, now_ms};
use crate::commands::packs::{disabled, enabled, loaded};
use crate::packs::Pack;
use crate::storage::{blocking, lock_update, to_json, write_atomic, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io;
use std::path::Path;
use tauri::AppHandle;

const FOLDER: &str = "encounters";
const MAX_NAME_LEN: usize = 100;
const MAX_QUANTITY: u32 = 999;

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

// Ids become file names, so only allow characters that can't leave the folder.
fn check_id(id: &str) -> Result<(), StorageError> {
    if !id.is_empty() && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        Ok(())
    } else {
        Err(StorageError::Invalid(format!("\"{id}\" is not an encounter id.")))
    }
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
    if problems.is_empty() { Ok(()) } else { Err(StorageError::Invalid(problems.join(" "))) }
}

fn read(folder: &Path, id: &str) -> Result<Encounter, StorageError> {
    let text = fs::read_to_string(folder.join(format!("{id}.json"))).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => StorageError::NotFound("That encounter doesn't exist.".into()),
        _ => e.into(),
    })?;
    let mut encounter: Encounter = serde_json::from_str(&text).map_err(|e| StorageError::Corrupt(e.to_string()))?;
    encounter.id = id.into(); // the file name is the id
    Ok(encounter)
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
        let Some(id) = name.strip_suffix(".json").filter(|id| check_id(id).is_ok()) else { continue };
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
    check_id(id)?;
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
        check_id(&encounter.id)?;
        read(&folder, &encounter.id)?; // replacing only what exists, and never a file that can't be read
    }
    encounter.saved_at = now;
    write_atomic(&folder.join(format!("{}.json", encounter.id)), &to_json(&encounter)?)?;
    Ok(encounter)
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
        Encounter { id: String::new(), name: name.into(), monsters, party: vec!["a".into()], saved_at: 0 }
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
