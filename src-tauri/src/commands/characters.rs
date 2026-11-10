//! Characters: one file per character in its campaign's folder, `characters/<id>.json`, with its picture beside it
//! as `characters/<id>.jpg`. Only active campaigns are changed (an archived one is read-only).

use crate::commands::campaigns::{active_dir, dirs, now_ms, picture_at, store_picture_bytes, Picture};
use crate::commands::search;
use crate::storage::{blocking, check_id, lock_update, to_json, write_atomic, StorageError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
use tauri::AppHandle;

const FOLDER: &str = "characters";
const MAX_TEXT_LEN: usize = 100;
const MAX_LEVEL: u32 = 20;
/// The picture is a square avatar of at most this many pixels a side.
const PICTURE_SIDE: u32 = 512;
const MAX_PICTURE_LEN: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassLevel {
    /// The class's name: from a pack, or typed.
    pub name: String,
    pub level: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    /// Empty when saving a new character; set by `save`.
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub player: String,
    pub classes: Vec<ClassLevel>,
    pub max_hp: Option<u32>,
    pub current_hp: Option<u32>,
    pub temp_hp: Option<u32>,
    pub ac: Option<u32>,
    /// The initiative modifier (can be negative). Older files have none.
    #[serde(default)]
    pub initiative: Option<i32>,
    pub passive_perception: Option<u32>,
    pub passive_insight: Option<u32>,
    pub passive_investigation: Option<u32>,
    /// Shown in Home's party.
    #[serde(default)]
    pub on_home: bool,
    /// Milliseconds since the Unix epoch; set by `save`.
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
    /// Filled in when read, never saved.
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub picture: Option<Picture>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct CharacterList {
    pub characters: Vec<Character>,
    /// Files that couldn't be read as a character. They are left untouched.
    pub unreadable: Vec<String>,
}

/// Trims the texts and checks every rule of the form; all problems at once.
fn check(character: &mut Character) -> Result<(), StorageError> {
    let mut problems = Vec::new();
    character.name = character.name.trim().into();
    character.player = character.player.trim().into();
    if character.name.is_empty() {
        problems.push("A name is required.".to_string());
    }
    for (field, text) in [("name", &character.name), ("player", &character.player)] {
        if text.chars().count() > MAX_TEXT_LEN {
            problems.push(format!("The {field} must be at most {MAX_TEXT_LEN} characters."));
        }
    }
    if character.classes.is_empty() {
        problems.push("At least one class is required.".into());
    }
    for class in &mut character.classes {
        class.name = class.name.trim().into();
        if class.name.is_empty() || class.name.chars().count() > MAX_TEXT_LEN {
            problems.push(format!("Each class needs a name of at most {MAX_TEXT_LEN} characters."));
        }
        if !(1..=MAX_LEVEL).contains(&class.level) {
            problems.push(format!("A class level must be 1 to {MAX_LEVEL}."));
        }
    }
    if character.classes.iter().map(|c| c.level).sum::<u32>() > MAX_LEVEL {
        problems.push(format!("The total level must be at most {MAX_LEVEL}."));
    }
    problems.dedup();
    if problems.is_empty() { Ok(()) } else { Err(StorageError::Invalid(problems.join(" "))) }
}

fn read(folder: &Path, id: &str) -> Result<Character, StorageError> {
    let text = fs::read_to_string(folder.join(format!("{id}.json"))).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => StorageError::NotFound("That character doesn't exist.".into()),
        _ => e.into(),
    })?;
    let mut character: Character = serde_json::from_str(&text).map_err(|e| StorageError::Corrupt(e.to_string()))?;
    character.id = id.into(); // the file name is the id
    character.picture = picture_at(&folder.join(format!("{id}.jpg")));
    Ok(character)
}

/// The characters of a campaign folder, by name. A missing folder means none yet.
pub fn list(campaign_dir: &Path) -> Result<CharacterList, StorageError> {
    let folder = campaign_dir.join(FOLDER);
    let entries = match fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(CharacterList { characters: vec![], unreadable: vec![] }),
        Err(e) => return Err(e.into()),
    };
    let (mut characters, mut unreadable) = (Vec::new(), Vec::new());
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = name.strip_suffix(".json").filter(|id| check_id(id, "character").is_ok()) else { continue };
        match read(&folder, id) {
            Ok(c) => characters.push(c),
            Err(StorageError::Corrupt(_)) => unreadable.push(name),
            Err(e) => return Err(e),
        }
    }
    characters.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.id.cmp(&b.id)));
    unreadable.sort();
    Ok(CharacterList { characters, unreadable })
}

/// Creates the character (empty `id`) or replaces the saved one, keeping when it was created.
pub fn save(campaign_dir: &Path, mut character: Character, now: u64) -> Result<Character, StorageError> {
    check(&mut character)?;
    let folder = campaign_dir.join(FOLDER);
    let _guard = lock_update();
    if character.id.is_empty() {
        let base = format!("{now:x}");
        let (mut id, mut n) = (base.clone(), 1);
        while folder.join(format!("{id}.json")).exists() {
            n += 1;
            id = format!("{base}-{n}");
        }
        character.id = id;
        character.created_at = now;
    } else {
        check_id(&character.id, "character")?;
        character.created_at = read(&folder, &character.id)?.created_at;
    }
    character.updated_at = now;
    write_atomic(&folder.join(format!("{}.json", character.id)), &to_json(&Character { picture: None, ..character.clone() })?)?;
    read(&folder, &character.id)
}

/// Deletes the character and its picture.
pub fn delete(campaign_dir: &Path, id: &str) -> Result<(), StorageError> {
    check_id(id, "character")?;
    let folder = campaign_dir.join(FOLDER);
    read(&folder, id)?;
    // The picture first: a crash in between leaves the character without it, never a picture without a character.
    remove_file(&folder.join(format!("{id}.jpg")))?;
    remove_file(&folder.join(format!("{id}.json")))
}

fn remove_file(path: &Path) -> Result<(), StorageError> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

/// Sets the character's picture from an image the window cropped to a square; it's decoded (so only a real picture
/// is kept), shrunk and stored as JPEG. `None` removes it.
pub fn set_picture(campaign_dir: &Path, id: &str, bytes: Option<&[u8]>) -> Result<Character, StorageError> {
    check_id(id, "character")?;
    let folder = campaign_dir.join(FOLDER);
    read(&folder, id)?;
    let path = folder.join(format!("{id}.jpg"));
    let Some(bytes) = bytes else {
        remove_file(&path)?;
        return read(&folder, id);
    };
    if bytes.len() > MAX_PICTURE_LEN {
        return Err(StorageError::Invalid(format!("The picture must be at most {} MB.", MAX_PICTURE_LEN / 1024 / 1024)));
    }
    store_picture_bytes(bytes, PICTURE_SIDE, &path)?;
    read(&folder, id)
}

/// "Fighter 3 / Wizard 2".
pub fn class_line(character: &Character) -> String {
    character.classes.iter().map(|c| format!("{} {}", c.name, c.level)).collect::<Vec<_>>().join(" / ")
}

#[tauri::command]
pub async fn list_characters(app: AppHandle, campaign: String) -> Result<CharacterList, StorageError> {
    blocking(move || list(&active_dir(&dirs(&app)?, &campaign)?)).await
}

#[tauri::command]
pub async fn save_character(app: AppHandle, campaign: String, character: Character) -> Result<Character, StorageError> {
    blocking(move || {
        let saved = save(&active_dir(&dirs(&app)?, &campaign)?, character, now_ms())?;
        search::changed();
        Ok(saved)
    })
    .await
}

#[tauri::command]
pub async fn delete_character(app: AppHandle, campaign: String, id: String) -> Result<(), StorageError> {
    blocking(move || {
        delete(&active_dir(&dirs(&app)?, &campaign)?, &id)?;
        search::changed();
        Ok(())
    })
    .await
}

/// `bytes`: the cropped picture (any format the image decoder reads), or none to remove the picture.
#[tauri::command]
pub async fn set_character_picture(app: AppHandle, campaign: String, id: String, bytes: Option<Vec<u8>>) -> Result<Character, StorageError> {
    blocking(move || set_picture(&active_dir(&dirs(&app)?, &campaign)?, &id, bytes.as_deref())).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("koldron-characters-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn character(name: &str, classes: &[(&str, u32)]) -> Character {
        Character {
            id: String::new(),
            name: name.into(),
            player: " Lotte ".into(),
            classes: classes.iter().map(|(n, l)| ClassLevel { name: (*n).into(), level: *l }).collect(),
            max_hp: Some(38),
            current_hp: Some(31),
            temp_hp: None,
            ac: Some(15),
            initiative: Some(-1),
            passive_perception: Some(14),
            passive_insight: None,
            passive_investigation: None,
            on_home: true,
            created_at: 0,
            updated_at: 0,
            picture: None,
        }
    }

    #[test]
    fn save_creates_then_replaces_keeping_the_id_and_creation_time() {
        let dir = temp_dir("save");
        let made = save(&dir, character(" Mira ", &[("Rogue", 3), ("Wizard", 2)]), 10).unwrap();
        assert_eq!((made.id.as_str(), made.name.as_str(), made.player.as_str(), made.created_at), ("a", "Mira", "Lotte", 10));
        assert_eq!(class_line(&made), "Rogue 3 / Wizard 2");
        let twin = save(&dir, character("Brannoc", &[("Fighter", 5)]), 10).unwrap();
        assert_eq!(twin.id, "a-2");
        let edited = save(&dir, Character { name: "Mira Ashvale".into(), created_at: 99, ..made }, 20).unwrap();
        assert_eq!((edited.id.as_str(), edited.created_at, edited.updated_at), ("a", 10, 20));
        let list = list(&dir).unwrap();
        assert_eq!(list.characters.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Brannoc", "Mira Ashvale"]);
        assert!(list.unreadable.is_empty());
        assert_eq!(list.characters[1].initiative, Some(-1)); // a negative modifier is kept
        // A file from before there was an initiative still reads, without one.
        fs::write(dir.join(FOLDER).join("old.json"), r#"{"name":"Tavi","classes":[{"name":"Rogue","level":1}],"maxHp":null,"currentHp":null,"tempHp":null,"ac":null,"passivePerception":null,"passiveInsight":null,"passiveInvestigation":null}"#).unwrap();
        assert_eq!(read(&dir.join(FOLDER), "old").unwrap().initiative, None);
    }

    #[test]
    fn rejects_every_broken_rule_and_unknown_or_tricky_ids() {
        let dir = temp_dir("rules");
        let Err(StorageError::Invalid(all)) = save(&dir, character(" ", &[("Rogue", 15), (" ", 0), ("Wizard", 6)]), 1) else { panic!() };
        for want in ["A name is required", "needs a name", "level must be 1 to 20", "total level must be at most 20"] {
            assert!(all.contains(want), "missing \"{want}\" in {all}");
        }
        assert!(matches!(save(&dir, character("A", &[]), 1), Err(StorageError::Invalid(_))));
        assert!(matches!(save(&dir, character(&"x".repeat(101), &[("Rogue", 1)]), 1), Err(StorageError::Invalid(_))));
        assert!(save(&dir, character("A", &[("Rogue", 10), ("Wizard", 10)]), 1).is_ok());
        let ghost = Character { id: "nope".into(), ..character("A", &[("Rogue", 1)]) };
        assert!(matches!(save(&dir, ghost, 1), Err(StorageError::NotFound(_))));
        let tricky = Character { id: "../x".into(), ..character("A", &[("Rogue", 1)]) };
        assert!(matches!(save(&dir, tricky, 1), Err(StorageError::Invalid(_))));
        assert!(matches!(delete(&dir, "nope"), Err(StorageError::NotFound(_))));
    }

    #[test]
    fn a_damaged_file_is_listed_as_unreadable_and_left_alone() {
        let dir = temp_dir("damaged");
        save(&dir, character("Mira", &[("Rogue", 1)]), 1).unwrap();
        fs::write(dir.join(FOLDER).join("b.json"), "{ nope").unwrap();
        let list = list(&dir).unwrap();
        assert_eq!((list.characters.len(), list.unreadable.as_slice()), (1, &["b.json".to_string()][..]));
        assert_eq!(fs::read_to_string(dir.join(FOLDER).join("b.json")).unwrap(), "{ nope");
    }

    #[test]
    fn pictures_are_decoded_shrunk_and_removed_with_the_character() {
        let dir = temp_dir("picture");
        let id = save(&dir, character("Mira", &[("Rogue", 1)]), 1).unwrap().id;
        assert!(matches!(set_picture(&dir, &id, Some(b"not a picture")), Err(StorageError::Invalid(_))));
        let mut png = vec![];
        image::RgbImage::new(900, 900).write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let with = set_picture(&dir, &id, Some(&png)).unwrap();
        let path = with.picture.unwrap().path;
        assert_eq!(image::image_dimensions(&path).unwrap(), (512, 512));
        assert!(set_picture(&dir, &id, None).unwrap().picture.is_none());
        set_picture(&dir, &id, Some(&png)).unwrap();
        delete(&dir, &id).unwrap();
        assert!(!Path::new(&path).exists());
        assert!(list(&dir).unwrap().characters.is_empty());
    }
}
