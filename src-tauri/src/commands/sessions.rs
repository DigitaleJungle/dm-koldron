//! Sessions: one file per session in its campaign's folder, `sessions/<id>.json`. Only active campaigns are changed (an
//! archived one is read-only). A session holds its scenes in order and its checklist; a scene links the campaign's
//! locations and NPCs (`world.json`) and encounters (in their own folder) by their ids.

use crate::commands::campaigns::{active_dir, dirs, now_ms};
use crate::commands::world::{self, Location, Npc, World};
use crate::storage::{blocking, check_id, lock_update, to_json, write_atomic, StorageError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const FOLDER: &str = "sessions";
const MAX_TITLE_LEN: usize = 100;
const MAX_NOTE_LEN: usize = 20_000;
const MAX_LINE_LEN: usize = 200;
/// The most scenes or checklist items a session (or a scene's checklist) can hold.
const MAX_ENTRIES: usize = 200;
/// The most notes a session's log holds.
const MAX_LOG: usize = 2_000;

/// A checklist item.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Check {
    pub text: String,
    pub done: bool,
}

/// A note in the live session view's log.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LogEntry {
    /// When it was written (ms).
    pub at: u64,
    pub text: String,
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
    /// One of the campaign's locations, or none.
    pub location_id: Option<String>,
    pub npc_ids: Vec<String>,
    /// The campaign's encounters.
    pub encounter_ids: Vec<String>,
    pub checklist: Vec<Check>,
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
    /// What happened, once played.
    #[serde(default)]
    pub summary: String,
    /// When Run session was clicked (ms); set by the live session view.
    #[serde(default)]
    pub started: Option<u64>,
    /// When End session was clicked (ms); set by the live session view.
    #[serde(default)]
    pub ended: Option<u64>,
    /// When it was paused (ms), while it is.
    #[serde(default)]
    pub paused_at: Option<u64>,
    /// How long it was paused in all (ms), and between an end and running it again: left out of the time played.
    #[serde(default)]
    pub paused_ms: u64,
    /// How many scenes were played out: the length of `played_scene_ids` once the session has started.
    #[serde(default)]
    pub played_scenes: Option<u32>,
    /// The scenes played out, in any order; set by the live session view.
    #[serde(default)]
    pub played_scene_ids: Vec<String>,
    /// The scene the live session view shows.
    #[serde(default)]
    pub current_scene_id: Option<String>,
    /// The live session view's notes, newest first.
    #[serde(default)]
    pub log: Vec<LogEntry>,
    /// The notes up to this time (ms) are in the summary already: End session adds only later ones.
    #[serde(default)]
    pub log_added_until: Option<u64>,
    #[serde(default)]
    pub scenes: Vec<Scene>,
    /// The session's own locations and NPCs, as the earlier version saved them: read for `world::migrate`, never written.
    #[serde(default, skip_serializing)]
    pub locations: Vec<Location>,
    #[serde(default, skip_serializing)]
    pub npcs: Vec<Npc>,
    #[serde(default)]
    pub checklist: Vec<Check>,
    /// Milliseconds since the Unix epoch; set by `save`.
    #[serde(default)]
    pub updated: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// No scenes, or not every scene is ready.
    Drafting,
    /// Every scene is ready.
    Ready,
    /// Run session was clicked.
    Playing,
    /// End session was clicked.
    Played,
}

impl Session {
    pub fn status(&self) -> Status {
        if self.ended.is_some() {
            Status::Played
        } else if self.started.is_some() {
            Status::Playing
        } else if !self.scenes.is_empty() && self.scenes.iter().all(|s| s.ready) {
            Status::Ready
        } else {
            Status::Drafting
        }
    }
}

/// What the overview shows of a session.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub date: Option<String>,
    pub time: Option<String>,
    pub status: Status,
    pub premise: String,
    pub summary: String,
    pub ready: usize,
    pub scenes: usize,
    /// The campaign's locations and NPCs its scenes link, each once (links to removed ones left out).
    pub location_ids: Vec<String>,
    pub npc_ids: Vec<String>,
    /// How many encounters its scenes link, each counted once.
    pub encounters: usize,
    pub played_scenes: Option<u32>,
    /// From Run session to End session, the pauses left out, when both are known.
    pub played_ms: Option<u64>,
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

/// Every link must be a valid id, and linked once only.
fn check_links(links: &[String], what: &str, scene: &str, problems: &mut Vec<String>) {
    let mut seen = HashSet::new();
    if links.iter().any(|id| check_id(id, "link").is_err() || !seen.insert(id)) {
        problems.push(format!("{scene} links {what} with an invalid id, or links it twice."));
    }
}

/// Drops the scenes' links to locations and NPCs the campaign no longer has, and the live view's marks on scenes the
/// session no longer has.
fn prune(session: &mut Session, world: &World) {
    let scenes: HashSet<String> = session.scenes.iter().map(|s| s.id.clone()).collect();
    let mut seen = HashSet::new();
    session.played_scene_ids.retain(|id| scenes.contains(id) && seen.insert(id.clone()));
    session.current_scene_id = session.current_scene_id.take().filter(|id| scenes.contains(id));
    if session.started.is_some() {
        session.played_scenes = Some(session.played_scene_ids.len() as u32);
    }
    // An empty world is more likely a missing file (a restore, a sync not finished) than every entry deleted: the links
    // stay, as `world::clean_pictures` keeps the pictures.
    if world.locations.is_empty() && world.npcs.is_empty() {
        return;
    }
    let locations: HashSet<&str> = world.locations.iter().map(|l| l.id.as_str()).collect();
    let npcs: HashSet<&str> = world.npcs.iter().map(|n| n.id.as_str()).collect();
    for scene in &mut session.scenes {
        scene.location_id = scene.location_id.take().filter(|id| locations.contains(id.as_str()));
        scene.npc_ids.retain(|id| npcs.contains(id.as_str()));
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
    check_text(&mut session.summary, MAX_NOTE_LEN, "The summary", &mut problems);
    check_checklist(&mut session.checklist, "the checklist", &mut problems);

    check_count(session.scenes.len(), "Scenes", &mut problems);
    check_ids(session.scenes.iter().map(|s| &s.id), "scene", &mut problems);
    if session.log.len() > MAX_LOG {
        problems.push(format!("The session notes can hold at most {MAX_LOG} entries."));
    }
    for entry in &mut session.log {
        entry.text = entry.text.trim().into();
    }
    if session.log.iter().any(|e| e.text.is_empty() || e.text.chars().count() > MAX_NOTE_LEN) {
        problems.push(format!("Every session note needs a text of at most {MAX_NOTE_LEN} characters."));
    }
    for scene in &mut session.scenes {
        // A scene gets its title after it's added, so an empty one is fine.
        check_text(&mut scene.title, MAX_TITLE_LEN, "A scene's title", &mut problems);
        let name = if scene.title.is_empty() { "A scene".to_string() } else { format!("Scene \"{}\"", scene.title) };
        check_text(&mut scene.summary, MAX_NOTE_LEN, &format!("What happens in {name}"), &mut problems);
        if scene.location_id.as_ref().is_some_and(|id| check_id(id, "location").is_err()) {
            problems.push(format!("{name} is at a location with an invalid id."));
        }
        check_links(&scene.npc_ids, "an NPC", &name, &mut problems);
        check_links(&scene.encounter_ids, "an encounter", &name, &mut problems);
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

pub fn folder(campaign_dir: &Path) -> PathBuf {
    campaign_dir.join(FOLDER)
}

/// Every session file that can be read, and the names of those that can't.
fn read_folder(campaign_dir: &Path) -> Result<(Vec<Session>, Vec<String>), StorageError> {
    let folder = folder(campaign_dir);
    let entries = match fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok((vec![], vec![])),
        Err(e) => return Err(e.into()),
    };
    let (mut sessions, mut unreadable) = (Vec::new(), Vec::new());
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = name.strip_suffix(".json").filter(|id| check_id(id, "session").is_ok()) else { continue };
        match read(&folder, id) {
            Ok(s) => sessions.push(s),
            Err(StorageError::Corrupt(_)) => unreadable.push(name),
            Err(e) => return Err(e),
        }
    }
    Ok((sessions, unreadable))
}

/// Every session that can be read.
pub fn read_all(campaign_dir: &Path) -> Result<Vec<Session>, StorageError> {
    Ok(read_folder(campaign_dir)?.0)
}

/// Writes the session as it is, unchecked: for `world::migrate`.
pub fn write(campaign_dir: &Path, session: &Session) -> Result<(), StorageError> {
    write_atomic(&folder(campaign_dir).join(format!("{}.json", session.id)), &to_json(session)?)
}

/// Each id once, in the order first linked, and only those `known`.
fn distinct<'a>(ids: impl Iterator<Item = &'a String>, known: &HashSet<&str>) -> Vec<String> {
    let mut seen = HashSet::new();
    ids.filter(|id| known.contains(id.as_str()) && seen.insert(id.as_str())).cloned().collect()
}

fn row(s: Session, world: &World) -> SessionRow {
    let locations: HashSet<&str> = world.locations.iter().map(|l| l.id.as_str()).collect();
    let npcs: HashSet<&str> = world.npcs.iter().map(|n| n.id.as_str()).collect();
    let encounters: HashSet<&String> = s.scenes.iter().flat_map(|sc| &sc.encounter_ids).collect();
    SessionRow {
        status: s.status(),
        ready: s.scenes.iter().filter(|sc| sc.ready).count(),
        scenes: s.scenes.len(),
        location_ids: distinct(s.scenes.iter().filter_map(|sc| sc.location_id.as_ref()), &locations),
        npc_ids: distinct(s.scenes.iter().flat_map(|sc| &sc.npc_ids), &npcs),
        encounters: encounters.len(),
        played_scenes: s.played_scenes,
        played_ms: s.started.zip(s.ended).map(|(a, b)| b.saturating_sub(a).saturating_sub(s.paused_ms)),
        id: s.id,
        number: s.number,
        title: s.title,
        date: s.date,
        time: s.time,
        premise: s.premise,
        summary: s.summary,
    }
}

pub fn list(campaign_dir: &Path) -> Result<SessionList, StorageError> {
    let (all, mut unreadable) = read_folder(campaign_dir)?;
    let world = world::read(campaign_dir)?;
    let mut sessions: Vec<(u64, SessionRow)> = all.into_iter().map(|s| (s.updated, row(s, &world))).collect();
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
    let folder = folder(campaign_dir);
    let _guard = lock_update();
    prune(&mut session, &world::read(campaign_dir)?);
    // Only the earlier version's files have these; the commands have moved them to the campaign by now.
    session.locations.clear();
    session.npcs.clear();
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

/// The campaign's folder, once its sessions' own locations and NPCs have moved to the campaign.
pub(crate) fn migrated(app: &AppHandle, campaign: &str) -> Result<PathBuf, StorageError> {
    let dir = active_dir(&dirs(app)?, campaign)?;
    world::migrate(&dir, now_ms())?;
    Ok(dir)
}

#[tauri::command]
pub async fn list_sessions(app: AppHandle, campaign: String) -> Result<SessionList, StorageError> {
    blocking(move || list(&migrated(&app, &campaign)?)).await
}

#[tauri::command]
pub async fn get_session(app: AppHandle, campaign: String, id: String) -> Result<Session, StorageError> {
    blocking(move || get(&migrated(&app, &campaign)?, &id)).await
}

#[tauri::command]
pub async fn save_session(app: AppHandle, campaign: String, session: Session) -> Result<Session, StorageError> {
    blocking(move || save(&migrated(&app, &campaign)?, session, now_ms())).await
}

#[tauri::command]
pub async fn delete_session(app: AppHandle, campaign: String, id: String) -> Result<(), StorageError> {
    blocking(move || delete(&migrated(&app, &campaign)?, &id)).await
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
            summary: String::new(),
            started: None,
            ended: None,
            paused_at: None,
            paused_ms: 0,
            played_scenes: None,
            played_scene_ids: vec![],
            current_scene_id: None,
            log: vec![],
            log_added_until: None,
            scenes: vec![],
            locations: vec![],
            npcs: vec![],
            checklist: vec![],
            updated: 0,
        }
    }

    /// The campaign's ossuary and abbot, which `full` links.
    fn world_with_links(dir: &Path) {
        let world = World {
            locations: vec![Location { id: "ossuary".into(), name: "Ossuary".into(), ..Location::default() }],
            npcs: vec![Npc { id: "abbot".into(), name: "The abbot's ghost".into(), ..Npc::default() }],
            ..World::default()
        };
        world::save(dir, world).unwrap();
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
        world_with_links(&dir);
        assert!(list(&dir).unwrap().sessions.is_empty());
        let first = save(&dir, full(), 10).unwrap();
        assert_eq!((first.id.as_str(), first.title.as_str(), first.updated), ("a", "Into the Crypt", 10));
        assert_eq!(get(&dir, &first.id).unwrap(), first);
        let second = save(&dir, session(2, "Bridge"), 11).unwrap();
        save(&dir, Session { title: "Crypt II".into(), ..first }, 20).unwrap();
        let all = list(&dir).unwrap().sessions;
        assert_eq!(all.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(), ["Bridge", "Crypt II"]);
        let crypt = &all[1];
        assert_eq!((crypt.ready, crypt.scenes, crypt.time.as_deref(), crypt.premise.as_str()), (1, 2, Some("19:30"), "Into the crypt."));
        assert_eq!((crypt.location_ids.clone(), crypt.npc_ids.clone(), crypt.encounters), (vec!["ossuary".to_string()], vec!["abbot".to_string()], 1));
        assert_eq!((crypt.status, crypt.played_ms), (Status::Drafting, None));
        assert_eq!(get(&dir, &second.id).unwrap(), second);
    }

    #[test]
    fn drops_links_to_removed_locations_and_npcs() {
        let dir = temp_dir("prune");
        world_with_links(&dir);
        let saved = save(&dir, full(), 1).unwrap();
        assert_eq!(saved.scenes[0].location_id.as_deref(), Some("ossuary"));
        // Both removed (another location is left, so the world isn't empty).
        let chapel = World { locations: vec![Location { id: "chapel".into(), name: "Chapel".into(), ..Location::default() }], ..World::default() };
        world::save(&dir, chapel).unwrap();
        assert_eq!(list(&dir).unwrap().sessions[0].npc_ids, Vec::<String>::new()); // never shown
        let saved = save(&dir, saved, 2).unwrap();
        assert_eq!((saved.scenes[0].location_id.clone(), saved.scenes[0].npc_ids.clone()), (None, vec![]));
        assert_eq!(saved.scenes[0].encounter_ids, ["e1"]); // encounters are the builder's
    }

    #[test]
    fn keeps_links_when_the_world_is_empty_or_missing() {
        let dir = temp_dir("prune-empty");
        world_with_links(&dir);
        let saved = save(&dir, full(), 1).unwrap();
        fs::remove_file(dir.join(world::FILE)).unwrap(); // e.g. a sync not finished
        let saved = save(&dir, saved, 2).unwrap();
        assert_eq!((saved.scenes[0].location_id.as_deref(), saved.scenes[0].npc_ids.clone()), (Some("ossuary"), vec!["abbot".to_string()]));
    }

    #[test]
    fn status_follows_scenes_then_running() {
        let ready = Scene { ready: true, ..scene("a") };
        assert_eq!(session(1, "A").status(), Status::Drafting); // no scenes
        assert_eq!(Session { scenes: vec![ready.clone(), scene("b")], ..session(1, "A") }.status(), Status::Drafting);
        assert_eq!(Session { scenes: vec![ready.clone()], ..session(1, "A") }.status(), Status::Ready);
        let playing = Session { started: Some(1_000), ..session(1, "A") };
        assert_eq!(playing.status(), Status::Playing);
        let played = Session { ended: Some(61_000), played_scenes: Some(3), ..playing };
        assert_eq!(played.status(), Status::Played);
        let r = row(played.clone(), &World::default());
        assert_eq!((r.played_ms, r.played_scenes), (Some(60_000), Some(3)));
        let r = row(Session { paused_ms: 15_000, ..played }, &World::default());
        assert_eq!(r.played_ms, Some(45_000)); // the pauses left out
    }

    #[test]
    fn keeps_the_live_views_marks_only_on_scenes_it_has() {
        let dir = temp_dir("live");
        world_with_links(&dir);
        let running = Session {
            started: Some(5),
            played_scene_ids: vec!["b".into(), "gone".into(), "b".into(), "a".into()],
            current_scene_id: Some("gone".into()),
            log: vec![LogEntry { at: 7, text: " Recap read ".into() }],
            ..full()
        };
        let saved = save(&dir, running, 9).unwrap();
        assert_eq!((saved.played_scene_ids.clone(), saved.current_scene_id.clone(), saved.played_scenes), (vec!["b".to_string(), "a".to_string()], None, Some(2)));
        assert_eq!(saved.log, [LogEntry { at: 7, text: "Recap read".into() }]);
        let blank = Session { log: vec![LogEntry { at: 8, text: " ".into() }], ..saved };
        assert!(matches!(save(&dir, blank, 10), Err(StorageError::Invalid(_))));
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
        assert!(bad(with_scene(Scene { location_id: Some("../x".into()), ..scene("a") })));
        assert!(bad(with_scene(Scene { npc_ids: vec!["Bad Id".into()], ..scene("a") })));
        assert!(bad(with_scene(Scene { npc_ids: vec!["abbot".into(), "abbot".into()], ..scene("a") }))); // linked twice
        assert!(bad(with_scene(Scene { encounter_ids: vec!["e1".into(), "e1".into()], ..scene("a") }))); // linked twice
        assert!(bad(with_scene(Scene { encounter_ids: vec!["../e".into()], ..scene("a") })));
        assert!(bad(with_scene(Scene { checklist: vec![check_item("")], ..scene("a") })));
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
        s.scenes[0].checklist[0].text = " The ghost appears ".into();
        let saved = save(&dir, s, 1).unwrap();
        assert_eq!(saved.title, "Into the Crypt");
        assert_eq!(saved.scenes[0].checklist[0].text, "The ghost appears");
    }
}
