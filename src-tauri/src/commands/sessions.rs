//! Sessions: one file per session in its campaign's folder, `sessions/<id>.json`. Only active campaigns are changed (an
//! archived one is read-only). A session holds its scenes in order, plus the locations, NPCs and checklist shared
//! across them; a scene links those by id, and the campaign's encounters (in their own folder) by their ids.

use crate::commands::campaigns::{active_dir, dirs, now_ms};
use crate::storage::{blocking, check_id, lock_update, to_json, write_atomic, StorageError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;
use tauri::AppHandle;

const FOLDER: &str = "sessions";
const MAX_TITLE_LEN: usize = 100;
const MAX_NOTE_LEN: usize = 20_000;
const MAX_LINE_LEN: usize = 200;
/// The most scenes, locations, NPCs or checklist items a session (or a scene's checklist) can hold.
const MAX_ENTRIES: usize = 200;

/// A checklist item.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Check {
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Scene {
    /// Chosen by the window; unique within the session.
    pub id: String,
    pub title: String,
    /// What happens.
    pub summary: String,
    pub ready: bool,
    /// One of the session's locations, or none.
    pub location_id: Option<String>,
    pub npc_ids: Vec<String>,
    /// The campaign's encounters.
    pub encounter_ids: Vec<String>,
    pub checklist: Vec<Check>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Location {
    pub id: String,
    pub name: String,
    /// What the party sees, hears and smells.
    pub description: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Npc {
    pub id: String,
    pub name: String,
    pub wants: String,
    /// How they talk.
    pub voice: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// Empty when saving a new session; set by `save`.
    #[serde(default)]
    pub id: String,
    pub number: u32,
    pub title: String,
    /// `YYYY-MM-DD`, when there is a date.
    #[serde(default)]
    pub date: Option<String>,
    /// `HH:MM`, the start time.
    #[serde(default)]
    pub time: Option<String>,
    /// Plain text about the party, for this session.
    #[serde(default)]
    pub party_note: String,
    /// A recap of the last session.
    #[serde(default)]
    pub previously: String,
    /// What this session is about and how it opens.
    #[serde(default)]
    pub premise: String,
    #[serde(default)]
    pub scenes: Vec<Scene>,
    #[serde(default)]
    pub locations: Vec<Location>,
    #[serde(default)]
    pub npcs: Vec<Npc>,
    #[serde(default)]
    pub checklist: Vec<Check>,
    /// Milliseconds since the Unix epoch; set by `save`.
    #[serde(default)]
    pub updated: u64,
}

/// What the list shows of a session.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub date: Option<String>,
    pub time: Option<String>,
    pub ready: usize,
    pub scenes: usize,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct SessionList {
    /// Highest number first.
    pub sessions: Vec<SessionRow>,
    /// Files that couldn't be read as a session. They are left untouched.
    pub unreadable: Vec<String>,
}

fn valid_date(date: &str) -> bool {
    let b = date.as_bytes();
    let number = |range: std::ops::Range<usize>| date.get(range).and_then(|s| s.parse::<u32>().ok());
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && matches!(number(5..7), Some(1..=12))
        && matches!(number(8..10), Some(1..=31))
}

fn valid_time(time: &str) -> bool {
    let b = time.as_bytes();
    let number = |range: std::ops::Range<usize>| time.get(range).and_then(|s| s.parse::<u32>().ok());
    b.len() == 5
        && b[2] == b':'
        && b.iter().enumerate().all(|(i, c)| i == 2 || c.is_ascii_digit())
        && matches!(number(0..2), Some(0..=23))
        && matches!(number(3..5), Some(0..=59))
}

/// Trims the text; one over `max` characters is a problem.
fn check_text(text: &mut String, max: usize, what: &str, problems: &mut Vec<String>) {
    *text = text.trim().into();
    if text.chars().count() > max {
        problems.push(format!("{what} must be at most {max} characters."));
    }
}

fn check_count(len: usize, what: &str, problems: &mut Vec<String>) {
    if len > MAX_ENTRIES {
        problems.push(format!("{what} can hold at most {MAX_ENTRIES} entries."));
    }
}

/// Trims every item; a blank, too long or too many is a problem.
fn check_checklist(list: &mut [Check], what: &str, problems: &mut Vec<String>) {
    check_count(list.len(), what, problems);
    for item in list.iter_mut() {
        item.text = item.text.trim().into();
    }
    if list.iter().any(|c| c.text.is_empty() || c.text.chars().count() > MAX_LINE_LEN) {
        problems.push(format!("Every item in {what} needs a text of at most {MAX_LINE_LEN} characters."));
    }
}

/// The ids, checked to be valid and unique.
fn check_ids<'a>(ids: impl Iterator<Item = &'a String>, what: &str, problems: &mut Vec<String>) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut bad = false;
    for id in ids {
        bad |= check_id(id, what).is_err() || !seen.insert(id.clone());
    }
    if bad {
        problems.push(format!("A {what} has an invalid or repeated id."));
    }
    seen
}

/// Every link must point to something in the session, and once only.
fn check_links(links: &[String], known: &HashSet<String>, what: &str, scene: &str, problems: &mut Vec<String>) {
    let mut seen = HashSet::new();
    if links.iter().any(|id| !known.contains(id) || !seen.insert(id)) {
        problems.push(format!("{scene} links a {what} that isn't in the session, or links it twice."));
    }
}

fn check(session: &mut Session) -> Result<(), StorageError> {
    let mut problems = Vec::new();
    if session.number == 0 {
        problems.push("The number must be 1 or more.".to_string());
    }
    check_text(&mut session.title, MAX_TITLE_LEN, "The title", &mut problems);
    if session.title.is_empty() {
        problems.push("A title is required.".to_string());
    }
    if session.date.as_deref().is_some_and(|d| !valid_date(d)) {
        problems.push("The date must be YYYY-MM-DD.".into());
    }
    if session.time.as_deref().is_some_and(|t| !valid_time(t)) {
        problems.push("The time must be HH:MM.".into());
    }
    check_text(&mut session.party_note, MAX_NOTE_LEN, "The party note", &mut problems);
    check_text(&mut session.previously, MAX_NOTE_LEN, "Previously", &mut problems);
    check_text(&mut session.premise, MAX_NOTE_LEN, "The premise", &mut problems);
    check_checklist(&mut session.checklist, "the checklist", &mut problems);

    check_count(session.locations.len(), "Locations", &mut problems);
    let locations = check_ids(session.locations.iter().map(|l| &l.id), "location", &mut problems);
    for location in &mut session.locations {
        check_text(&mut location.name, MAX_TITLE_LEN, "A location's name", &mut problems);
        check_text(&mut location.description, MAX_NOTE_LEN, "A location's description", &mut problems);
    }
    check_count(session.npcs.len(), "NPCs", &mut problems);
    let npcs = check_ids(session.npcs.iter().map(|n| &n.id), "NPC", &mut problems);
    for npc in &mut session.npcs {
        check_text(&mut npc.name, MAX_TITLE_LEN, "An NPC's name", &mut problems);
        check_text(&mut npc.wants, MAX_LINE_LEN, "What an NPC wants", &mut problems);
        check_text(&mut npc.voice, MAX_LINE_LEN, "An NPC's voice", &mut problems);
    }

    check_count(session.scenes.len(), "Scenes", &mut problems);
    check_ids(session.scenes.iter().map(|s| &s.id), "scene", &mut problems);
    for scene in &mut session.scenes {
        // A scene gets its title after it's added, so an empty one is fine.
        check_text(&mut scene.title, MAX_TITLE_LEN, "A scene's title", &mut problems);
        let name = if scene.title.is_empty() { "A scene".to_string() } else { format!("Scene \"{}\"", scene.title) };
        check_text(&mut scene.summary, MAX_NOTE_LEN, &format!("What happens in {name}"), &mut problems);
        if scene.location_id.as_ref().is_some_and(|id| !locations.contains(id)) {
            problems.push(format!("{name} is at a location that isn't in the session."));
        }
        check_links(&scene.npc_ids, &npcs, "NPC", &name, &mut problems);
        let mut seen = HashSet::new();
        if scene.encounter_ids.iter().any(|id| check_id(id, "encounter").is_err() || !seen.insert(id)) {
            problems.push(format!("{name} links an encounter with an invalid id, or links it twice."));
        }
        check_checklist(&mut scene.checklist, &format!("the checklist of {name}"), &mut problems);
    }
    if problems.is_empty() { Ok(()) } else { Err(StorageError::Invalid(problems.join(" "))) }
}

fn read(folder: &Path, id: &str) -> Result<Session, StorageError> {
    let text = fs::read_to_string(folder.join(format!("{id}.json"))).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => StorageError::NotFound("That session doesn't exist.".into()),
        _ => e.into(),
    })?;
    let mut session: Session = serde_json::from_str(&text).map_err(|e| StorageError::Corrupt(e.to_string()))?;
    session.id = id.into(); // the file name is the id
    Ok(session)
}

pub fn list(campaign_dir: &Path) -> Result<SessionList, StorageError> {
    let folder = campaign_dir.join(FOLDER);
    let entries = match fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(SessionList { sessions: vec![], unreadable: vec![] }),
        Err(e) => return Err(e.into()),
    };
    let (mut sessions, mut unreadable) = (Vec::new(), Vec::new());
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = name.strip_suffix(".json").filter(|id| check_id(id, "session").is_ok()) else { continue };
        match read(&folder, id) {
            Ok(s) => {
                let row = SessionRow {
                    ready: s.scenes.iter().filter(|sc| sc.ready).count(),
                    scenes: s.scenes.len(),
                    id: s.id,
                    number: s.number,
                    title: s.title,
                    date: s.date,
                    time: s.time,
                };
                sessions.push((s.updated, row));
            }
            Err(StorageError::Corrupt(_)) => unreadable.push(name),
            Err(e) => return Err(e),
        }
    }
    sessions.sort_by(|(ua, a), (ub, b)| b.number.cmp(&a.number).then(ub.cmp(ua)).then(a.id.cmp(&b.id)));
    unreadable.sort();
    Ok(SessionList { sessions: sessions.into_iter().map(|(_, row)| row).collect(), unreadable })
}

pub fn get(campaign_dir: &Path, id: &str) -> Result<Session, StorageError> {
    check_id(id, "session")?;
    read(&campaign_dir.join(FOLDER), id)
}

/// Creates the session (empty `id`) or replaces the saved one.
pub fn save(campaign_dir: &Path, mut session: Session, now: u64) -> Result<Session, StorageError> {
    check(&mut session)?;
    let folder = campaign_dir.join(FOLDER);
    let _guard = lock_update();
    let saved_number = if session.id.is_empty() {
        None
    } else {
        check_id(&session.id, "session")?;
        Some(read(&folder, &session.id)?.number) // replacing only what exists, and never a file that can't be read
    };
    // Only a new or changed number is checked, so sessions that already share one can still be saved.
    if saved_number != Some(session.number) && list(campaign_dir)?.sessions.iter().any(|s| s.number == session.number) {
        return Err(StorageError::Invalid(format!("Session {} already exists.", session.number)));
    }
    if session.id.is_empty() {
        let base = format!("{now:x}");
        let (mut id, mut n) = (base.clone(), 1);
        while folder.join(format!("{id}.json")).exists() {
            n += 1;
            id = format!("{base}-{n}");
        }
        session.id = id;
    }
    session.updated = now;
    write_atomic(&folder.join(format!("{}.json", session.id)), &to_json(&session)?)?;
    Ok(session)
}

/// Deletes the session; its encounters stay.
pub fn delete(campaign_dir: &Path, id: &str) -> Result<(), StorageError> {
    check_id(id, "session")?;
    let folder = campaign_dir.join(FOLDER);
    let _guard = lock_update();
    read(&folder, id)?;
    fs::remove_file(folder.join(format!("{id}.json")))?;
    Ok(())
}

#[tauri::command]
pub async fn list_sessions(app: AppHandle, campaign: String) -> Result<SessionList, StorageError> {
    blocking(move || list(&active_dir(&dirs(&app)?, &campaign)?)).await
}

#[tauri::command]
pub async fn get_session(app: AppHandle, campaign: String, id: String) -> Result<Session, StorageError> {
    blocking(move || get(&active_dir(&dirs(&app)?, &campaign)?, &id)).await
}

#[tauri::command]
pub async fn save_session(app: AppHandle, campaign: String, session: Session) -> Result<Session, StorageError> {
    blocking(move || save(&active_dir(&dirs(&app)?, &campaign)?, session, now_ms())).await
}

#[tauri::command]
pub async fn delete_session(app: AppHandle, campaign: String, id: String) -> Result<(), StorageError> {
    blocking(move || delete(&active_dir(&dirs(&app)?, &campaign)?, &id)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("koldron-sessions-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn session(number: u32, title: &str) -> Session {
        Session {
            id: String::new(),
            number,
            title: title.into(),
            date: None,
            time: None,
            party_note: String::new(),
            previously: String::new(),
            premise: String::new(),
            scenes: vec![],
            locations: vec![],
            npcs: vec![],
            checklist: vec![],
            updated: 0,
        }
    }

    fn scene(id: &str) -> Scene {
        Scene { id: id.into(), title: format!("Scene {id}"), ..Scene::default() }
    }

    fn check_item(text: &str) -> Check {
        Check { text: text.into(), done: false }
    }

    /// A session with one of everything, linked up.
    fn full() -> Session {
        Session {
            date: Some("2026-11-10".into()),
            time: Some("19:30".into()),
            party_note: "Tobb is hurt".into(),
            previously: "They reached the abbey.".into(),
            premise: "Into the crypt.".into(),
            locations: vec![Location { id: "ossuary".into(), name: "Ossuary".into(), description: "Skulls in rows".into() }],
            npcs: vec![Npc { id: "abbot".into(), name: "The abbot's ghost".into(), wants: "Peace".into(), voice: "Scripture".into() }],
            checklist: vec![Check { text: "Minis".into(), done: true }],
            scenes: vec![
                Scene {
                    ready: true,
                    location_id: Some("ossuary".into()),
                    npc_ids: vec!["abbot".into()],
                    encounter_ids: vec!["e1".into()],
                    checklist: vec![check_item("The ghost appears")],
                    ..scene("a")
                },
                scene("b"),
            ],
            ..session(1, " Into the Crypt ")
        }
    }

    #[test]
    fn saves_reads_back_and_lists_the_highest_number_first() {
        let dir = temp_dir("save");
        assert!(list(&dir).unwrap().sessions.is_empty());
        let first = save(&dir, full(), 10).unwrap();
        assert_eq!((first.id.as_str(), first.title.as_str(), first.updated), ("a", "Into the Crypt", 10));
        assert_eq!(get(&dir, &first.id).unwrap(), first);
        let second = save(&dir, session(2, "Bridge"), 11).unwrap();
        save(&dir, Session { title: "Crypt II".into(), ..first }, 20).unwrap();
        let all = list(&dir).unwrap().sessions;
        assert_eq!(all.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(), ["Bridge", "Crypt II"]);
        assert_eq!((all[1].ready, all[1].scenes, all[1].time.as_deref()), (1, 2, Some("19:30")));
        assert_eq!(get(&dir, &second.id).unwrap(), second);
    }

    #[test]
    fn rejects_a_number_another_session_has() {
        let dir = temp_dir("number");
        let first = save(&dir, session(1, "Vault"), 1).unwrap();
        let second = save(&dir, session(2, "Bridge"), 2).unwrap();
        let Err(StorageError::Invalid(why)) = save(&dir, session(1, "Again"), 3) else { panic!() };
        assert!(why.contains("Session 1 already exists"), "{why}");
        assert!(matches!(save(&dir, Session { number: 1, ..second.clone() }, 4), Err(StorageError::Invalid(_))));
        save(&dir, Session { title: "Vault II".into(), ..first.clone() }, 5).unwrap(); // keeping its own number is fine
        save(&dir, Session { number: 3, ..second }, 6).unwrap();
        // Sessions that already share a number (saved before this rule) can still be saved.
        let mut twin = first.clone();
        twin.id = "twin".into();
        fs::write(dir.join(FOLDER).join("twin.json"), to_json(&twin).unwrap()).unwrap();
        save(&dir, Session { title: "Twin".into(), ..twin }, 7).unwrap();
    }

    #[test]
    fn deletes_only_the_session() {
        let dir = temp_dir("delete");
        let saved = save(&dir, session(1, "Vault"), 1).unwrap();
        delete(&dir, &saved.id).unwrap();
        assert!(list(&dir).unwrap().sessions.is_empty());
        assert!(matches!(delete(&dir, &saved.id), Err(StorageError::NotFound(_))));
        assert!(matches!(delete(&dir, "../x"), Err(StorageError::Invalid(_))));
    }

    #[test]
    fn rejects_bad_sessions_unknown_ids_and_keeps_damaged_files() {
        let dir = temp_dir("rules");
        let all = Session { date: Some("2026-13-01".into()), time: Some("24:00".into()), checklist: vec![check_item(" ")], ..session(0, " ") };
        let Err(StorageError::Invalid(all)) = save(&dir, all, 1) else { panic!() };
        for part in ["number must be 1", "title is required", "YYYY-MM-DD", "HH:MM", "the checklist"] {
            assert!(all.contains(part), "{all}");
        }
        let bad = |s: Session| matches!(check(&mut s.clone()), Err(StorageError::Invalid(_)));
        let with_scene = |sc: Scene| Session { scenes: vec![sc], ..full() };
        assert!(!bad(full()));
        assert!(bad(session(1, &"x".repeat(MAX_TITLE_LEN + 1))));
        assert!(bad(Session { premise: "x".repeat(MAX_NOTE_LEN + 1), ..full() }));
        assert!(!bad(with_scene(Scene { title: " ".into(), ..scene("a") }))); // untitled is fine
        assert!(bad(Session { scenes: vec![scene("a"), scene("a")], ..full() })); // repeated scene id
        assert!(bad(with_scene(scene("../x"))));
        assert!(bad(with_scene(Scene { location_id: Some("nowhere".into()), ..scene("a") })));
        assert!(bad(with_scene(Scene { npc_ids: vec!["stranger".into()], ..scene("a") })));
        assert!(bad(with_scene(Scene { npc_ids: vec!["abbot".into(), "abbot".into()], ..scene("a") }))); // linked twice
        assert!(bad(with_scene(Scene { encounter_ids: vec!["e1".into(), "e1".into()], ..scene("a") }))); // linked twice
        assert!(bad(with_scene(Scene { encounter_ids: vec!["../e".into()], ..scene("a") })));
        assert!(bad(with_scene(Scene { checklist: vec![check_item("")], ..scene("a") })));
        assert!(bad(Session { npcs: vec![Npc { id: "a".into(), ..Npc::default() }, Npc { id: "a".into(), ..Npc::default() }], ..session(1, "A") }));
        let ghost = Session { id: "nope".into(), ..session(1, "A") };
        assert!(matches!(save(&dir, ghost, 1), Err(StorageError::NotFound(_))));
        fs::create_dir_all(dir.join(FOLDER)).unwrap();
        fs::write(dir.join(FOLDER).join("b.json"), "{ nope").unwrap();
        assert_eq!(list(&dir).unwrap().unreadable, ["b.json"]);
        let damaged = Session { id: "b".into(), ..session(1, "A") };
        assert!(matches!(save(&dir, damaged, 1), Err(StorageError::Corrupt(_))));
        assert_eq!(fs::read_to_string(dir.join(FOLDER).join("b.json")).unwrap(), "{ nope");
    }

    #[test]
    fn trims_what_it_saves() {
        let dir = temp_dir("trim");
        let mut s = full();
        s.locations[0].name = "  Ossuary ".into();
        s.scenes[0].checklist[0].text = " The ghost appears ".into();
        let saved = save(&dir, s, 1).unwrap();
        assert_eq!(saved.locations[0].name, "Ossuary");
        assert_eq!(saved.scenes[0].checklist[0].text, "The ghost appears");
    }
}
