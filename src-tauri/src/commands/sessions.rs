//! Sessions: one file per session in its campaign's folder, `sessions/<id>.json`, plus the campaign-wide open threads
//! in `threads.json`. Only active campaigns are changed (an archived one is read-only). A session is a list of prep
//! steps; an encounters step links encounters by id, the encounters themselves live in their own folder. A Map step's
//! picture is stored beside the session as `sessions/<session>.<step>.map.jpg`.

use crate::commands::campaigns::{active_dir, dirs, now_ms, picture_at, store_picture, Picture};
use crate::commands::encounters::{check_pins, Pin, MAP_MAX_SIDE};
use crate::storage::{blocking, check_id, lock_update, to_json, write_atomic, StorageError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const FOLDER: &str = "sessions";
const THREADS_FILE: &str = "threads.json";
const MAX_TITLE_LEN: usize = 100;
const MAX_NOTE_LEN: usize = 20_000;
const MAX_STEPS: usize = 100;
const MAX_ITEMS: usize = 500;
const MAX_LINES: usize = 200;
const MAX_LINE_LEN: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Note,
    List,
    Encounter,
    Map,
}

/// How a List step shows its items.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListStyle {
    #[default]
    Bulleted,
    Numbered,
    Check,
}

/// One entry of a step; which fields count depends on the step's kind (the rest are cleared on save):
/// note: `body` (HTML, the only item); list: `title` and `body` (plain text), plus `done` in a checklist.
/// An Encounter step has no items; it keeps its encounter's id in `Step::encounter`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Item {
    pub title: String,
    pub body: String,
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// Chosen by the window; unique within the session.
    pub id: String,
    pub kind: Kind,
    pub title: String,
    #[serde(default)]
    pub ready: bool,
    #[serde(default)]
    pub items: Vec<Item>,
    /// An Encounter step's encounter; none until one is picked.
    #[serde(default)]
    pub encounter: Option<String>,
    /// A List step's style; Bulleted for other kinds.
    #[serde(default)]
    pub list_style: ListStyle,
    /// A Map step's pins.
    #[serde(default)]
    pub pins: Vec<Pin>,
    /// A Map step's picture, found beside the session when it's read; never saved in the file or taken from the window.
    #[serde(default, skip_deserializing)]
    pub map: Option<Picture>,
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
    #[serde(default)]
    pub steps: Vec<Step>,
    /// Plain text about the party, for this session.
    #[serde(default)]
    pub party_note: String,
    #[serde(default)]
    pub to_bring: Vec<String>,
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
    pub steps: usize,
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

/// Trims every line; blank ones, too many or too long ones are problems.
fn check_lines(lines: &mut [String], what: &str, problems: &mut Vec<String>) {
    for line in lines.iter_mut() {
        *line = line.trim().into();
    }
    if lines.iter().any(|l| l.is_empty()) {
        problems.push(format!("An entry in {what} can't be empty."));
    }
    if lines.len() > MAX_LINES || lines.iter().any(|l| l.chars().count() > MAX_LINE_LEN) {
        problems.push(format!("{what} can hold at most {MAX_LINES} entries of {MAX_LINE_LEN} characters."));
    }
}

fn check_step(step: &mut Step, problems: &mut Vec<String>) {
    step.title = step.title.trim().into();
    // A step gets its title after it's added, so an empty one is fine.
    let name = if step.title.is_empty() { "A step".to_string() } else { format!("Step \"{}\"", step.title) };
    if step.title.chars().count() > MAX_TITLE_LEN {
        problems.push(format!("{name} has a title over {MAX_TITLE_LEN} characters."));
    }
    if step.items.len() > MAX_ITEMS {
        problems.push(format!("{name} can hold at most {MAX_ITEMS} items."));
    }
    // Which fields the kind uses: title, body, done.
    let (wants_title, wants_body, wants_done) = match step.kind {
        Kind::Note => (false, true, false),
        Kind::List => (true, true, step.list_style == ListStyle::Check),
        Kind::Encounter | Kind::Map => (false, false, false),
    };
    if step.kind == Kind::Map {
        step.items.clear();
        check_pins(&step.pins, problems);
        if step.pins.iter().any(|p| p.text.chars().count() > MAX_NOTE_LEN) {
            problems.push(format!("{name} has a pin text over {MAX_NOTE_LEN} characters."));
        }
    } else {
        step.pins.clear();
    }
    if step.kind != Kind::List {
        step.list_style = ListStyle::Bulleted;
    }
    if step.kind != Kind::Encounter {
        step.encounter = None;
    } else {
        step.items.clear();
        if step.encounter.as_deref().is_some_and(|id| check_id(id, "encounter").is_err()) {
            problems.push(format!("{name} links an encounter with an invalid id."));
        }
    }
    for item in &mut step.items {
        item.title = item.title.trim().into();
        if !wants_title {
            item.title.clear();
        }
        if !wants_body {
            item.body.clear();
        }
        item.done &= wants_done;
        if wants_title && item.title.is_empty() {
            problems.push(format!("{name} has an item without a title."));
        }
        if item.title.chars().count() > MAX_TITLE_LEN {
            problems.push(format!("{name} has an item title over {MAX_TITLE_LEN} characters."));
        }
        if item.body.chars().count() > MAX_NOTE_LEN {
            problems.push(format!("{name} has a text over {MAX_NOTE_LEN} characters."));
        }
    }
    if step.kind == Kind::Note && step.items.len() > 1 {
        problems.push(format!("{name} is a note and holds one text."));
    }
}

fn check(session: &mut Session) -> Result<(), StorageError> {
    session.title = session.title.trim().into();
    let mut problems = Vec::new();
    if session.number == 0 {
        problems.push("The number must be 1 or more.".to_string());
    }
    if session.title.is_empty() {
        problems.push("A title is required.".to_string());
    } else if session.title.chars().count() > MAX_TITLE_LEN {
        problems.push(format!("The title must be at most {MAX_TITLE_LEN} characters."));
    }
    if session.date.as_deref().is_some_and(|d| !valid_date(d)) {
        problems.push("The date must be YYYY-MM-DD.".into());
    }
    if session.time.as_deref().is_some_and(|t| !valid_time(t)) {
        problems.push("The time must be HH:MM.".into());
    }
    if session.party_note.chars().count() > MAX_NOTE_LEN {
        problems.push(format!("The party note must be at most {MAX_NOTE_LEN} characters."));
    }
    check_lines(&mut session.to_bring, "To bring", &mut problems);
    if session.steps.len() > MAX_STEPS {
        problems.push(format!("A session can have at most {MAX_STEPS} steps."));
    }
    let mut ids = HashSet::new();
    for step in &mut session.steps {
        if check_id(&step.id, "step").is_err() || !ids.insert(step.id.clone()) {
            problems.push("A step has an invalid or repeated id.".into());
        }
        check_step(step, &mut problems);
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
    find_maps(folder, &mut session);
    Ok(session)
}

fn map_path(folder: &Path, session: &str, step: &str) -> PathBuf {
    folder.join(format!("{session}.{step}.map.jpg"))
}

/// Sets each Map step's picture from what's beside the session.
fn find_maps(folder: &Path, session: &mut Session) {
    for step in &mut session.steps {
        step.map = if step.kind == Kind::Map { picture_at(&map_path(folder, &session.id, &step.id)) } else { None };
    }
}

/// Removes the session's map pictures, except those of the steps named in `keep`.
fn remove_maps(folder: &Path, session: &str, keep: &[&str]) -> Result<(), StorageError> {
    let prefix = format!("{session}.");
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(step) = name.strip_prefix(&prefix).and_then(|rest| rest.strip_suffix(".map.jpg")) else { continue };
        if !keep.contains(&step) {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
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
                    ready: s.steps.iter().filter(|st| st.ready).count(),
                    steps: s.steps.len(),
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
    if session.id.is_empty() {
        let base = format!("{now:x}");
        let (mut id, mut n) = (base.clone(), 1);
        while folder.join(format!("{id}.json")).exists() {
            n += 1;
            id = format!("{base}-{n}");
        }
        session.id = id;
    } else {
        check_id(&session.id, "session")?;
        read(&folder, &session.id)?; // replacing only what exists, and never a file that can't be read
    }
    session.updated = now;
    let mut stored = session.clone();
    stored.steps.iter_mut().for_each(|s| s.map = None);
    write_atomic(&folder.join(format!("{}.json", session.id)), &to_json(&stored)?)?;
    // Pictures of Map steps that are gone (removed, or another kind now) go too.
    let maps: Vec<&str> = session.steps.iter().filter(|s| s.kind == Kind::Map).map(|s| s.id.as_str()).collect();
    remove_maps(&folder, &session.id, &maps)?;
    find_maps(&folder, &mut session);
    Ok(session)
}

/// Sets a saved Map step's picture from a file (shrunk, stored as JPEG), or removes it (`None`).
pub fn set_map(campaign_dir: &Path, session: &str, step: &str, source: Option<&Path>) -> Result<Option<Picture>, StorageError> {
    check_id(session, "session")?;
    check_id(step, "step")?;
    let folder = campaign_dir.join(FOLDER);
    let _guard = lock_update();
    let saved = read(&folder, session)?;
    if !saved.steps.iter().any(|s| s.id == step && s.kind == Kind::Map) {
        return Err(StorageError::NotFound("Save the session to add a map to this step.".into()));
    }
    let path = map_path(&folder, session, step);
    match source {
        Some(source) => store_picture(source, MAP_MAX_SIDE, &path)?,
        None => match fs::remove_file(&path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        },
    }
    Ok(picture_at(&path))
}

/// Deletes the session; its encounters stay.
pub fn delete(campaign_dir: &Path, id: &str) -> Result<(), StorageError> {
    check_id(id, "session")?;
    let folder = campaign_dir.join(FOLDER);
    let _guard = lock_update();
    read(&folder, id)?;
    fs::remove_file(folder.join(format!("{id}.json")))?;
    remove_maps(&folder, id, &[])?;
    Ok(())
}

/// The campaign's open threads (shared by every session); none before the first save.
pub fn get_threads(campaign_dir: &Path) -> Result<Vec<String>, StorageError> {
    match fs::read_to_string(campaign_dir.join(THREADS_FILE)) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| StorageError::Corrupt(e.to_string())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(e.into()),
    }
}

/// Replaces the open threads; returns them trimmed.
pub fn save_threads(campaign_dir: &Path, mut threads: Vec<String>) -> Result<Vec<String>, StorageError> {
    let mut problems = Vec::new();
    check_lines(&mut threads, "Open threads", &mut problems);
    if !problems.is_empty() {
        return Err(StorageError::Invalid(problems.join(" ")));
    }
    let _guard = lock_update();
    get_threads(campaign_dir)?; // never over a file that can't be read
    write_atomic(&campaign_dir.join(THREADS_FILE), &to_json(&threads)?)?;
    Ok(threads)
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

#[tauri::command]
pub async fn set_session_map(app: AppHandle, campaign: String, session: String, step: String, path: Option<String>) -> Result<Option<Picture>, StorageError> {
    blocking(move || set_map(&active_dir(&dirs(&app)?, &campaign)?, &session, &step, path.as_deref().map(Path::new))).await
}

#[tauri::command]
pub async fn get_threads_cmd(app: AppHandle, campaign: String) -> Result<Vec<String>, StorageError> {
    blocking(move || get_threads(&active_dir(&dirs(&app)?, &campaign)?)).await
}

#[tauri::command]
pub async fn save_threads_cmd(app: AppHandle, campaign: String, threads: Vec<String>) -> Result<Vec<String>, StorageError> {
    blocking(move || save_threads(&active_dir(&dirs(&app)?, &campaign)?, threads)).await
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
        Session { id: String::new(), number, title: title.into(), date: None, time: None, steps: vec![], party_note: String::new(), to_bring: vec![], updated: 0 }
    }

    fn step(id: &str, kind: Kind, items: Vec<Item>) -> Step {
        Step { id: id.into(), kind, title: format!("Step {id}"), ready: false, items, encounter: None, list_style: ListStyle::Bulleted, pins: vec![], map: None }
    }

    fn titled(title: &str) -> Item {
        Item { title: title.into(), ..Item::default() }
    }

    fn linked(id: &str) -> Step {
        Step { encounter: Some(id.into()), ..step("e", Kind::Encounter, vec![]) }
    }

    fn all_kinds() -> Vec<Step> {
        vec![
            Step { ready: true, ..step("a", Kind::Note, vec![Item { body: "<p>Prep</p>".into(), ..Item::default() }]) },
            step("b", Kind::List, vec![Item { title: "Chapel".into(), body: "Wet stone".into(), ..Item::default() }, titled("Vault")]),
            Step { list_style: ListStyle::Check, ..step("c", Kind::List, vec![Item { done: true, ..titled("The ring") }, titled("The bell")]) },
            Step { id: "d".into(), ..linked("y") },
        ]
    }

    #[test]
    fn saves_reads_back_and_lists_the_highest_number_first() {
        let dir = temp_dir("save");
        assert!(list(&dir).unwrap().sessions.is_empty());
        let full = Session {
            date: Some("2026-11-10".into()),
            time: Some("19:30".into()),
            steps: all_kinds(),
            party_note: "Tobb is hurt".into(),
            to_bring: vec!["Minis".into(), "Handout".into()],
            ..session(1, " Vault ")
        };
        let first = save(&dir, full, 10).unwrap();
        assert_eq!((first.id.as_str(), first.title.as_str(), first.updated), ("a", "Vault", 10));
        assert_eq!(get(&dir, &first.id).unwrap(), first);
        // The order of steps is kept, and an Encounter step keeps its encounter.
        assert_eq!(first.steps.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["a", "b", "c", "d"]);
        assert_eq!(first.steps[3].encounter.as_deref(), Some("y"));
        let second = save(&dir, session(2, "Bridge"), 11).unwrap();
        save(&dir, Session { title: "Vault II".into(), ..first }, 20).unwrap();
        let all = list(&dir).unwrap().sessions;
        assert_eq!(all.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(), ["Bridge", "Vault II"]);
        assert_eq!((all[1].ready, all[1].steps, all[1].time.as_deref()), (1, 4, Some("19:30")));
        assert_eq!(get(&dir, &second.id).unwrap(), second);
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
        let all = Session { date: Some("2026-13-01".into()), time: Some("24:00".into()), to_bring: vec![" ".into()], ..session(0, " ") };
        let Err(StorageError::Invalid(all)) = save(&dir, all, 1) else { panic!() };
        for part in ["number must be 1", "title is required", "YYYY-MM-DD", "HH:MM", "To bring"] {
            assert!(all.contains(part), "{all}");
        }
        let bad = |s: Session| matches!(save(&dir, s, 1), Err(StorageError::Invalid(_)));
        let with = |steps: Vec<Step>| Session { steps, ..session(1, "A") };
        assert!(bad(session(1, &"x".repeat(MAX_TITLE_LEN + 1))));
        assert!(bad(Session { party_note: "x".repeat(MAX_NOTE_LEN + 1), ..session(1, "A") }));
        assert!(!bad(with(vec![Step { title: " ".into(), ..step("a", Kind::Note, vec![]) }])));
        assert!(bad(with(vec![step("a", Kind::Note, vec![]), step("a", Kind::List, vec![])]))); // repeated step id
        assert!(bad(with(vec![step("../x", Kind::Note, vec![])])));
        assert!(bad(with(vec![step("a", Kind::Note, vec![Item::default(), Item::default()])])));
        assert!(bad(with(vec![step("a", Kind::Note, vec![Item { body: "x".repeat(MAX_NOTE_LEN + 1), ..Item::default() }])])));
        assert!(bad(with(vec![step("a", Kind::List, vec![titled(" ")])])));
        assert!(bad(with(vec![step("a", Kind::List, vec![titled(&"x".repeat(MAX_TITLE_LEN + 1))])])));
        assert!(bad(with(vec![linked("../x")])));
        assert!(!bad(with(vec![linked("x"), Step { id: "f".into(), ..linked("x") }]))); // the same encounter in two steps is fine
        assert!(!bad(with(vec![step("g", Kind::Encounter, vec![])]))); // none picked yet
        assert!(!bad(session(1, &"x".repeat(MAX_TITLE_LEN))));
        let ghost = Session { id: "nope".into(), ..session(1, "A") };
        assert!(matches!(save(&dir, ghost, 1), Err(StorageError::NotFound(_))));
        fs::write(dir.join(FOLDER).join("b.json"), "{ nope").unwrap();
        assert_eq!(list(&dir).unwrap().unreadable, ["b.json"]);
        let damaged = Session { id: "b".into(), ..session(1, "A") };
        assert!(matches!(save(&dir, damaged, 1), Err(StorageError::Corrupt(_))));
        assert_eq!(fs::read_to_string(dir.join(FOLDER).join("b.json")).unwrap(), "{ nope");
    }

    #[test]
    fn clears_fields_a_kind_does_not_use() {
        let dir = temp_dir("clear");
        let stray = Item { title: "t".into(), body: "b".into(), done: true };
        let mixed = Step { encounter: Some("e".into()), list_style: ListStyle::Check, ..step("a", Kind::List, vec![stray.clone()]) };
        let bulleted = step("c", Kind::List, vec![stray.clone()]);
        let note = Step { list_style: ListStyle::Numbered, ..step("d", Kind::Note, vec![]) };
        let enc = Step { items: vec![stray.clone()], ..linked("e") };
        let saved = save(&dir, Session { steps: vec![mixed, Step { id: "b".into(), ..enc }, bulleted, note], ..session(1, "A") }, 1).unwrap();
        assert_eq!(saved.steps[0].encounter, None);
        assert_eq!(saved.steps[0].items[0], Item { done: true, ..stray.clone() });
        assert_eq!(saved.steps[2].items[0], Item { done: false, ..stray.clone() });
        assert_eq!(saved.steps[3].list_style, ListStyle::Bulleted);
        assert!(saved.steps[1].items.is_empty());
        assert_eq!(saved.steps[1].encounter.as_deref(), Some("e"));
    }

    #[test]
    fn threads_round_trip_and_are_validated() {
        let dir = temp_dir("threads");
        assert!(get_threads(&dir).unwrap().is_empty());
        assert_eq!(save_threads(&dir, vec![" Who paid? ".into(), "The ring".into()]).unwrap(), ["Who paid?", "The ring"]);
        assert_eq!(get_threads(&dir).unwrap(), ["Who paid?", "The ring"]);
        for bad in [vec![String::new()], vec!["x".repeat(MAX_LINE_LEN + 1)], vec!["a".to_string(); MAX_LINES + 1]] {
            assert!(matches!(save_threads(&dir, bad), Err(StorageError::Invalid(_))));
        }
        assert_eq!(get_threads(&dir).unwrap().len(), 2); // a refused save changes nothing
        fs::write(dir.join(THREADS_FILE), "{ nope").unwrap();
        assert!(matches!(save_threads(&dir, vec!["New".into()]), Err(StorageError::Corrupt(_))));
        assert_eq!(fs::read_to_string(dir.join(THREADS_FILE)).unwrap(), "{ nope"); // a damaged file is kept
    }

    #[test]
    fn a_map_step_keeps_its_pins_and_picture_until_the_step_or_session_goes() {
        let dir = temp_dir("map");
        let pin = Pin { x: 0.5, y: 0.5, title: "Door".into(), text: "Locked".into() };
        let map_step = Step { pins: vec![pin.clone()], ..step("m", Kind::Map, vec![]) };
        let saved = save(&dir, Session { steps: vec![map_step], ..session(1, "A") }, 1).unwrap();
        assert_eq!(saved.steps[0].pins, std::slice::from_ref(&pin));
        let off = Step { pins: vec![Pin { x: 2.0, ..pin.clone() }], ..step("m", Kind::Map, vec![]) };
        assert!(matches!(save(&dir, Session { steps: vec![off], ..saved.clone() }, 2), Err(StorageError::Invalid(_))));

        let png = dir.join("map.png");
        image::RgbImage::new(64, 32).save(&png).unwrap();
        assert!(matches!(set_map(&dir, &saved.id, "unsaved", Some(&png)), Err(StorageError::NotFound(_))));
        let map = set_map(&dir, &saved.id, "m", Some(&png)).unwrap().unwrap();
        assert_eq!(get(&dir, &saved.id).unwrap().steps[0].map.as_ref().map(|m| &m.path), Some(&map.path));
        let file = dir.join(FOLDER).join(format!("{}.json", saved.id));
        assert!(!fs::read_to_string(&file).unwrap().contains("map.jpg"));

        // The step removed: its picture goes on save. The session deleted: all of its pictures go.
        save(&dir, Session { steps: vec![], ..saved.clone() }, 3).unwrap();
        assert!(!Path::new(&map.path).exists());
        let again = save(&dir, saved.clone(), 4).unwrap();
        let map = set_map(&dir, &again.id, "m", Some(&png)).unwrap().unwrap();
        delete(&dir, &again.id).unwrap();
        assert!(!Path::new(&map.path).exists());
    }
}
