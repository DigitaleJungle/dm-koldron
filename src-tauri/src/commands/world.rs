//! The campaign's locations and NPCs, as two trees, in `world.json` in its folder. Sessions link them by id from their
//! scenes. Sessions saved before this held their own lists; `migrate` moves those here once (see docs/SPEC.md).
//! A location's pictures (its images and its map) are files in `locations/<id>/`, stored as soon as they're chosen.

use crate::commands::campaigns::{active_dir, dirs, now_ms, picture_at, store_picture, Picture};
use crate::commands::encounters::{check_pins, Pin, MAP_MAX_SIDE};
use crate::commands::sessions::{self, Session};
use crate::storage::{blocking, check_id, lock_update, to_json, write_atomic, StorageError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const FILE: &str = "world.json";
/// `World::version` once every session's own locations and NPCs have moved here.
const MIGRATED: u32 = 1;
const MAX_NAME_LEN: usize = 100;
const MAX_NOTE_LEN: usize = 20_000;
const MAX_LINE_LEN: usize = 200;
/// The most locations, or NPCs, a campaign can hold.
const MAX_ENTRIES: usize = 2000;
/// The most images a location can have.
const MAX_IMAGES: usize = 50; // the editor stops adding at this many (LocationFields.svelte)
const PICTURES: &str = "locations";
const MAP_FILE: &str = "map.jpg";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Location {
    pub id: String,
    /// The location it's in; none at the top.
    pub parent: Option<String>,
    pub name: String,
    /// What the party sees, hears and smells (rich text). Saved as `description` before there were more fields.
    #[serde(alias = "description")]
    pub atmosphere: String,
    /// Why it's there, what it's for (rich text).
    pub purpose: String,
    /// What only the DM knows (rich text).
    pub secrets: String,
    /// The campaign's NPCs linked to it.
    pub npc_ids: Vec<String>,
    /// The pins on its map.
    pub pins: Vec<Pin>,
    /// Its images: file names in `locations/<id>/`, in order.
    pub images: Vec<String>,
    /// The images as pictures, one per name; set by Rust, never saved.
    #[serde(skip_deserializing, skip_serializing_if = "Vec::is_empty")]
    pub pictures: Vec<Picture>,
    /// The map picture; set by Rust, never saved.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub map: Option<Picture>,
}

/// An image just stored for a location: its file name (for `Location::images`) and the picture to show.
#[derive(Debug, PartialEq, Serialize)]
pub struct StoredImage {
    pub name: String,
    pub picture: Picture,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Npc {
    pub id: String,
    /// The NPC (or group) it's under; none at the top.
    pub parent: Option<String>,
    pub name: String,
    pub wants: String,
    /// How they talk.
    pub voice: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct World {
    /// Set by `save` and `migrate`; 0 (or no file) until the sessions' own lists have moved here.
    pub version: u32,
    pub locations: Vec<Location>,
    pub npcs: Vec<Npc>,
}

fn pictures_dir(campaign_dir: &Path, location: &str) -> PathBuf {
    campaign_dir.join(PICTURES).join(location)
}

/// An image file name as `add_image` makes them.
fn valid_image(name: &str) -> bool {
    name.strip_suffix(".jpg").is_some_and(|stem| check_id(stem, "image").is_ok())
}

/// Fills in each location's pictures from its files; an image whose file is gone is left out.
fn with_pictures(campaign_dir: &Path, mut world: World) -> World {
    for l in &mut world.locations {
        let dir = pictures_dir(campaign_dir, &l.id);
        let found: Vec<(String, Picture)> = l.images.iter().filter_map(|n| picture_at(&dir.join(n)).map(|p| (n.clone(), p))).collect();
        (l.images, l.pictures) = found.into_iter().unzip();
        l.map = picture_at(&dir.join(MAP_FILE));
    }
    world
}

/// Removes picture files no location refers to: a removed location's folder, a removed image. Run when the world is
/// loaded, never while it's being edited (an image just added isn't in the saved world yet).
fn clean_pictures(campaign_dir: &Path, world: &World) -> Result<(), StorageError> {
    let entries = match fs::read_dir(campaign_dir.join(PICTURES)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    for entry in entries.filter_map(Result::ok) {
        let id = entry.file_name().to_string_lossy().into_owned();
        let Some(location) = world.locations.iter().find(|l| l.id == id) else {
            fs::remove_dir_all(entry.path())?;
            continue;
        };
        for file in fs::read_dir(entry.path())?.filter_map(Result::ok) {
            let name = file.file_name().to_string_lossy().into_owned();
            if name != MAP_FILE && !location.images.contains(&name) {
                fs::remove_file(file.path())?;
            }
        }
    }
    Ok(())
}

/// Stores the picture file at `source` as a new image of the location (which needn't be saved yet).
pub fn add_image(campaign_dir: &Path, location: &str, source: &Path, now: u64) -> Result<StoredImage, StorageError> {
    check_id(location, "location")?;
    let dir = pictures_dir(campaign_dir, location);
    let (mut name, mut n) = (format!("{now:x}.jpg"), 1);
    while dir.join(&name).exists() {
        n += 1;
        name = format!("{now:x}-{n}.jpg");
    }
    store_picture(source, MAP_MAX_SIDE, &dir.join(&name))?;
    let picture = picture_at(&dir.join(&name)).ok_or_else(|| StorageError::Io("The image wasn't stored.".into()))?;
    Ok(StoredImage { name, picture })
}

/// Makes the picture file at `source` the location's map, or removes it (none).
pub fn set_map(campaign_dir: &Path, location: &str, source: Option<&Path>) -> Result<Option<Picture>, StorageError> {
    check_id(location, "location")?;
    let path = pictures_dir(campaign_dir, location).join(MAP_FILE);
    match source {
        Some(source) => store_picture(source, MAP_MAX_SIDE, &path)?,
        None => match fs::remove_file(&path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        },
    }
    Ok(picture_at(&path))
}

/// The saved world; an empty one (version 0) when there's no file yet.
pub fn read(campaign_dir: &Path) -> Result<World, StorageError> {
    match fs::read_to_string(campaign_dir.join(FILE)) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| StorageError::Corrupt(e.to_string())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(World::default()),
        Err(e) => Err(e.into()),
    }
}

/// Trims the text; one over `max` characters is a problem.
fn check_text(text: &mut String, max: usize, what: &str, problems: &mut Vec<String>) {
    *text = text.trim().into();
    if text.chars().count() > max {
        problems.push(format!("{what} must be at most {max} characters."));
    }
}

/// Ids valid and unique, every parent one of the others, and no entry under itself.
fn check_tree<'a>(entries: impl Iterator<Item = (&'a String, &'a Option<String>)> + Clone, what: &str, problems: &mut Vec<String>) {
    let mut seen = HashSet::new();
    if entries.clone().any(|(id, _)| check_id(id, what).is_err() || !seen.insert(id.as_str())) {
        problems.push(format!("A {what} has an invalid or repeated id."));
        return;
    }
    let parents: HashMap<&str, Option<&str>> = entries.map(|(id, parent)| (id.as_str(), parent.as_deref())).collect();
    if parents.values().flatten().any(|p| !parents.contains_key(p)) {
        problems.push(format!("A {what} is inside one that doesn't exist."));
        return;
    }
    for start in parents.keys() {
        let (mut at, mut steps) = (parents[start], 0);
        while let Some(p) = at {
            steps += 1;
            if p == *start || steps > parents.len() {
                problems.push(format!("A {what} is inside itself."));
                return;
            }
            at = parents[p];
        }
    }
}

fn check(world: &mut World) -> Result<(), StorageError> {
    let mut problems = Vec::new();
    for (len, what) in [(world.locations.len(), "Locations"), (world.npcs.len(), "NPCs")] {
        if len > MAX_ENTRIES {
            problems.push(format!("{what} can hold at most {MAX_ENTRIES} entries."));
        }
    }
    check_tree(world.locations.iter().map(|l| (&l.id, &l.parent)), "location", &mut problems);
    check_tree(world.npcs.iter().map(|n| (&n.id, &n.parent)), "NPC", &mut problems);
    let npcs: HashSet<String> = world.npcs.iter().map(|n| n.id.clone()).collect();
    for l in &mut world.locations {
        check_text(&mut l.name, MAX_NAME_LEN, "A location's name", &mut problems);
        check_text(&mut l.atmosphere, MAX_NOTE_LEN, "A location's atmosphere", &mut problems);
        check_text(&mut l.purpose, MAX_NOTE_LEN, "A location's purpose", &mut problems);
        check_text(&mut l.secrets, MAX_NOTE_LEN, "A location's secrets", &mut problems);
        if l.pins.iter().any(|p| p.text.chars().count() > MAX_NOTE_LEN) {
            problems.push(format!("A pin's text must be at most {MAX_NOTE_LEN} characters."));
        }
        check_pins(&l.pins, &mut problems);
        let mut seen = HashSet::new();
        if l.images.len() > MAX_IMAGES || l.images.iter().any(|n| !valid_image(n) || !seen.insert(n.clone())) {
            problems.push(format!("A location can have at most {MAX_IMAGES} images, each once."));
        }
        // An NPC removed from the campaign is unlinked; each is linked once.
        let mut seen = HashSet::new();
        l.npc_ids.retain(|id| npcs.contains(id) && seen.insert(id.clone()));
    }
    for n in &mut world.npcs {
        check_text(&mut n.name, MAX_NAME_LEN, "An NPC's name", &mut problems);
        check_text(&mut n.wants, MAX_LINE_LEN, "What an NPC wants", &mut problems);
        check_text(&mut n.voice, MAX_LINE_LEN, "An NPC's voice", &mut problems);
    }
    if problems.is_empty() { Ok(()) } else { Err(StorageError::Invalid(problems.join(" "))) }
}

/// Replaces the saved world. Never call before `migrate` (a world saved early would skip it).
pub fn save(campaign_dir: &Path, mut world: World) -> Result<World, StorageError> {
    check(&mut world)?;
    world.version = MIGRATED;
    let _guard = lock_update();
    read(campaign_dir)?; // never over a file that can't be read
    for l in &mut world.locations {
        // What the window sent back; filled in again from the files.
        l.pictures.clear();
        l.map = None;
    }
    write_atomic(&campaign_dir.join(FILE), &to_json(&world)?)?;
    Ok(with_pictures(campaign_dir, world))
}

/// Adds `more` to `text` unless it's empty or already one of its parts; keeps `text` when the two together are too long.
fn join(text: &mut String, more: &str, between: &str, max: usize) {
    let more = more.trim();
    if more.is_empty() || text.split(between).any(|part| part.trim() == more) {
        return;
    }
    let joined = if text.is_empty() { more.to_string() } else { format!("{text}{between}{more}") };
    if joined.chars().count() <= max {
        *text = joined;
    }
}

/// The entry `id` becomes: the one with that id, else the first with the same name (any case), else a new one.
/// Running it again over the same entry changes nothing, so an interrupted migration can run again.
fn merge_into<T>(list: &mut Vec<T>, entry: T, id: impl Fn(&T) -> &str, name: impl Fn(&T) -> &str, fill: impl Fn(&mut T, &T)) -> String {
    let key = name(&entry).trim().to_lowercase();
    let found = list.iter().position(|e| id(e) == id(&entry)).or_else(|| (!key.is_empty()).then(|| list.iter().position(|e| name(e).trim().to_lowercase() == key)).flatten());
    match found {
        Some(at) => {
            fill(&mut list[at], &entry);
            id(&list[at]).to_string()
        }
        None => {
            let new_id = id(&entry).to_string();
            list.push(entry);
            new_id
        }
    }
}

/// Moves the locations and NPCs that sessions still hold into the world; see docs/SPEC.md. Every session file it
/// changes is first copied to `backups/sessions-<now>/`, and the world is written before any session loses its lists.
/// Unreadable session files are left alone, and caught on a later call once they can be read.
pub fn migrate(campaign_dir: &Path, now: u64) -> Result<(), StorageError> {
    let _guard = lock_update();
    let mut world = read(campaign_dir)?;
    let mut legacy: Vec<Session> = sessions::read_all(campaign_dir)?.into_iter().filter(|s| !s.locations.is_empty() || !s.npcs.is_empty()).collect();
    if legacy.is_empty() && world.version >= MIGRATED {
        return Ok(());
    }
    legacy.sort_by(|a, b| a.number.cmp(&b.number).then(a.id.cmp(&b.id)));

    if !legacy.is_empty() {
        let backup = campaign_dir.join("backups").join(format!("sessions-{now}"));
        fs::create_dir_all(&backup)?;
        for s in &legacy {
            let name = format!("{}.json", s.id);
            fs::copy(sessions::folder(campaign_dir).join(&name), backup.join(&name))?;
        }

        let mut renamed: Vec<(HashMap<String, String>, HashMap<String, String>)> = Vec::new();
        for s in &legacy {
            let locations = s.locations.iter().map(|l| {
                let id = merge_into(&mut world.locations, Location { parent: None, ..l.clone() }, |e| &e.id, |e| &e.name, |into, from| {
                    if into.name.is_empty() {
                        into.name = from.name.clone();
                    }
                    join(&mut into.atmosphere, &from.atmosphere, "\n\n", MAX_NOTE_LEN);
                });
                (l.id.clone(), id)
            });
            let locations = locations.collect();
            let npcs = s.npcs.iter().map(|n| {
                let id = merge_into(&mut world.npcs, Npc { parent: None, ..n.clone() }, |e| &e.id, |e| &e.name, |into, from| {
                    if into.name.is_empty() {
                        into.name = from.name.clone();
                    }
                    join(&mut into.wants, &from.wants, " / ", MAX_LINE_LEN);
                    join(&mut into.voice, &from.voice, " / ", MAX_LINE_LEN);
                });
                (n.id.clone(), id)
            });
            renamed.push((locations, npcs.collect()));
        }
        write_atomic(&campaign_dir.join(FILE), &to_json(&world)?)?; // the sessions keep their lists until rewritten: safe to run again from here

        for (mut s, (locations, npcs)) in legacy.into_iter().zip(renamed) {
            for scene in &mut s.scenes {
                scene.location_id = scene.location_id.take().and_then(|id| locations.get(&id).cloned());
                let mut seen = HashSet::new();
                scene.npc_ids = scene.npc_ids.iter().filter_map(|id| npcs.get(id).cloned()).filter(|id| seen.insert(id.clone())).collect();
            }
            s.locations.clear();
            s.npcs.clear();
            sessions::write(campaign_dir, &s)?;
        }
    }
    world.version = MIGRATED;
    write_atomic(&campaign_dir.join(FILE), &to_json(&world)?)
}

/// The world, with its pictures; picture files nothing refers to any more are removed first.
#[tauri::command]
pub async fn get_world(app: AppHandle, campaign: String) -> Result<World, StorageError> {
    blocking(move || {
        let dir = active_dir(&dirs(&app)?, &campaign)?;
        migrate(&dir, now_ms())?;
        let world = read(&dir)?;
        clean_pictures(&dir, &world)?;
        Ok(with_pictures(&dir, world))
    })
    .await
}

/// `path`: the picture the DM chose.
#[tauri::command]
pub async fn add_location_image(app: AppHandle, campaign: String, location: String, path: String) -> Result<StoredImage, StorageError> {
    blocking(move || add_image(&active_dir(&dirs(&app)?, &campaign)?, &location, Path::new(&path), now_ms())).await
}

/// `path`: the picture the DM chose, or none to remove the map.
#[tauri::command]
pub async fn set_location_map(app: AppHandle, campaign: String, location: String, path: Option<String>) -> Result<Option<Picture>, StorageError> {
    blocking(move || set_map(&active_dir(&dirs(&app)?, &campaign)?, &location, path.as_deref().map(Path::new))).await
}

#[tauri::command]
pub async fn save_world(app: AppHandle, campaign: String, world: World) -> Result<World, StorageError> {
    blocking(move || {
        let dir = active_dir(&dirs(&app)?, &campaign)?;
        migrate(&dir, now_ms())?;
        save(&dir, world)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::sessions::Scene;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("koldron-world-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn loc(id: &str, parent: Option<&str>) -> Location {
        Location { id: id.into(), parent: parent.map(Into::into), name: id.to_uppercase(), ..Location::default() }
    }

    fn npc(id: &str, name: &str, wants: &str) -> Npc {
        Npc { id: id.into(), name: name.into(), wants: wants.into(), ..Npc::default() }
    }

    #[test]
    fn saves_and_reads_back_a_tree() {
        let dir = temp_dir("save");
        assert_eq!(read(&dir).unwrap(), World::default());
        let world = World { locations: vec![loc("coast", None), loc("abbey", Some("coast"))], npcs: vec![npc("wenna", " Wenna ", "")], version: 0 };
        let saved = save(&dir, world).unwrap();
        assert_eq!((saved.version, saved.npcs[0].name.as_str()), (MIGRATED, "Wenna"));
        assert_eq!(read(&dir).unwrap(), saved);
    }

    #[test]
    fn rejects_broken_trees() {
        let bad = |locations: Vec<Location>| matches!(check(&mut World { locations, ..World::default() }), Err(StorageError::Invalid(_)));
        assert!(!bad(vec![loc("a", None), loc("b", Some("a"))]));
        assert!(bad(vec![loc("a", None), loc("a", None)])); // repeated id
        assert!(bad(vec![loc("../a", None)]));
        assert!(bad(vec![loc("a", Some("nowhere"))]));
        assert!(bad(vec![loc("a", Some("a"))]));
        assert!(bad(vec![loc("a", Some("b")), loc("b", Some("a"))])); // a loop
        assert!(bad(vec![Location { secrets: "x".repeat(MAX_NOTE_LEN + 1), ..loc("a", None) }]));
    }

    /// A session file as the earlier version wrote it, with its own locations and NPCs.
    fn legacy(dir: &Path, id: &str, number: u32, locations: Vec<Location>, npcs: Vec<Npc>, scenes: Vec<Scene>) {
        let s = serde_json::json!({ "number": number, "title": format!("S{number}"), "locations": locations, "npcs": npcs, "scenes": scenes });
        fs::create_dir_all(sessions::folder(dir)).unwrap();
        fs::write(sessions::folder(dir).join(format!("{id}.json")), s.to_string()).unwrap();
    }

    #[test]
    fn migrates_session_lists_merging_same_names_and_repointing_links() {
        let dir = temp_dir("migrate");
        let chapel = |id: &str, d: &str| Location { id: id.into(), name: "Chapel".into(), atmosphere: d.into(), ..Location::default() };
        let scene = |loc: &str, npcs: &[&str]| Scene { id: "a".into(), location_id: Some(loc.into()), npc_ids: npcs.iter().map(|s| s.to_string()).collect(), ..Scene::default() };
        legacy(&dir, "two", 2, vec![chapel("c2", "Rotten pews")], vec![npc("w2", "wenna", "Ale"), npc("x2", "Wenna ", "")], vec![scene("c2", &["w2", "x2"])]);
        legacy(&dir, "one", 1, vec![chapel("c1", "Stained glass")], vec![npc("w1", "Wenna", "Coin")], vec![scene("c1", &["w1"])]);
        fs::write(sessions::folder(&dir).join("bad.json"), "{ nope").unwrap();

        migrate(&dir, 7).unwrap();
        let world = read(&dir).unwrap();
        assert_eq!(world.version, MIGRATED);
        assert_eq!(world.locations, vec![chapel("c1", "Stained glass\n\nRotten pews")]);
        assert_eq!((world.npcs.len(), world.npcs[0].id.as_str(), world.npcs[0].wants.as_str()), (1, "w1", "Coin / Ale"));
        let two = sessions::get(&dir, "two").unwrap();
        assert!(two.locations.is_empty() && two.npcs.is_empty());
        assert_eq!((two.scenes[0].location_id.as_deref(), two.scenes[0].npc_ids.clone()), (Some("c1"), vec!["w1".to_string()]));
        let raw = fs::read_to_string(sessions::folder(&dir).join("two.json")).unwrap();
        assert!(!raw.contains("\"npcs\""), "{raw}");
        assert!(dir.join("backups/sessions-7/two.json").exists() && dir.join("backups/sessions-7/one.json").exists());
        assert_eq!(fs::read_to_string(sessions::folder(&dir).join("bad.json")).unwrap(), "{ nope");

        migrate(&dir, 8).unwrap(); // once only
        assert!(!dir.join("backups/sessions-8").exists());
        assert_eq!(read(&dir).unwrap(), world);
    }

    #[test]
    fn an_interrupted_migration_runs_again_without_doubling() {
        let dir = temp_dir("resume");
        legacy(&dir, "one", 1, vec![], vec![npc("w1", "Wenna", "Coin")], vec![]);
        legacy(&dir, "two", 2, vec![], vec![npc("w2", "Wenna", "Ale")], vec![]);
        // Stopped after writing the world (version 0) and rewriting session one only.
        let half = World { version: 0, locations: vec![], npcs: vec![npc("w1", "Wenna", "Coin / Ale")] };
        fs::write(dir.join(FILE), to_json(&half).unwrap()).unwrap();
        let mut one = sessions::get(&dir, "one").unwrap();
        one.npcs.clear();
        sessions::write(&dir, &one).unwrap();

        migrate(&dir, 9).unwrap();
        let world = read(&dir).unwrap();
        assert_eq!((world.version, world.npcs.len(), world.npcs[0].wants.as_str()), (MIGRATED, 1, "Coin / Ale"));
    }

    #[test]
    fn a_session_unreadable_during_the_migration_moves_once_it_can_be_read() {
        let dir = temp_dir("late");
        legacy(&dir, "one", 1, vec![], vec![npc("w1", "Wenna", "Coin")], vec![]);
        let path = sessions::folder(&dir).join("one.json");
        let good = fs::read_to_string(&path).unwrap();
        fs::write(&path, "{ nope").unwrap();
        migrate(&dir, 1).unwrap();
        assert_eq!(read(&dir).unwrap().npcs, vec![]);

        fs::write(&path, good).unwrap(); // repaired by hand
        migrate(&dir, 2).unwrap();
        assert_eq!(read(&dir).unwrap().npcs[0].id, "w1");
        assert!(sessions::get(&dir, "one").unwrap().npcs.is_empty());
        assert!(dir.join("backups/sessions-2/one.json").exists());
    }

    #[test]
    fn joins_only_texts_that_are_not_already_a_part() {
        let mut text = "Golden bell".to_string();
        join(&mut text, "Gold", " / ", 200);
        join(&mut text, " Golden bell ", " / ", 200);
        assert_eq!(text, "Golden bell / Gold");
    }

    #[test]
    fn reads_an_old_description_as_the_atmosphere() {
        let old: Location = serde_json::from_str(r#"{"id":"inn","name":"Inn","description":"Low beams"}"#).unwrap();
        assert_eq!(old.atmosphere, "Low beams");
    }

    #[test]
    fn checks_a_locations_fields_and_unlinks_removed_npcs() {
        let dir = temp_dir("fields");
        let pin = Pin { x: 0.5, y: 0.5, title: "Trapdoor".into(), text: "To the crypt".into() };
        let inn = Location { npc_ids: vec!["wenna".into(), "gone".into(), "wenna".into()], pins: vec![pin.clone()], purpose: " Rest ".into(), ..loc("inn", None) };
        let world = World { locations: vec![inn], npcs: vec![npc("wenna", "Wenna", "")], version: 0 };
        let saved = save(&dir, world.clone()).unwrap();
        assert_eq!((saved.locations[0].npc_ids.clone(), saved.locations[0].purpose.as_str()), (vec!["wenna".to_string()], "Rest"));
        assert_eq!(read(&dir).unwrap().locations[0].pins, vec![pin.clone()]);
        let bad = |l: Location| matches!(check(&mut World { locations: vec![l], ..World::default() }), Err(StorageError::Invalid(_)));
        assert!(bad(Location { pins: vec![Pin { x: 1.5, ..pin.clone() }], ..loc("a", None) }));
        assert!(bad(Location { images: vec!["../x.jpg".into()], ..loc("a", None) }));
        assert!(bad(Location { images: vec!["a.jpg".into(), "a.jpg".into()], ..loc("a", None) }));
        assert!(!bad(Location { images: vec!["19a2b.jpg".into()], ..loc("a", None) }));
    }

    #[test]
    fn stores_images_and_a_map_and_cleans_up_what_nothing_refers_to() {
        let dir = temp_dir("pictures");
        let png = dir.join("pic.png");
        image::RgbImage::new(4000, 1000).save(&png).unwrap();
        let first = add_image(&dir, "inn", &png, 10).unwrap();
        let second = add_image(&dir, "inn", &png, 10).unwrap(); // the same moment: its own name
        assert_ne!(first.name, second.name);
        let stored = image::open(&first.picture.path).unwrap();
        assert_eq!((stored.width(), stored.height()), (MAP_MAX_SIDE, 512));
        assert!(set_map(&dir, "inn", Some(&png)).unwrap().is_some());
        add_image(&dir, "gone", &png, 11).unwrap();
        assert!(matches!(add_image(&dir, "../x", &png, 12), Err(StorageError::Invalid(_))));

        // Only the first image was kept; the "gone" location never saved.
        let saved = save(&dir, World { locations: vec![Location { images: vec![first.name.clone()], ..loc("inn", None) }], ..World::default() }).unwrap();
        assert_eq!((saved.locations[0].pictures.len(), saved.locations[0].map.is_some()), (1, true));
        assert!(!fs::read_to_string(dir.join(FILE)).unwrap().contains("pictures"));
        clean_pictures(&dir, &read(&dir).unwrap()).unwrap();
        let inn = pictures_dir(&dir, "inn");
        assert!(inn.join(&first.name).exists() && inn.join(MAP_FILE).exists() && !inn.join(&second.name).exists());
        assert!(!pictures_dir(&dir, "gone").exists());

        fs::remove_file(inn.join(&first.name)).unwrap(); // a file gone from under it
        assert!(with_pictures(&dir, read(&dir).unwrap()).locations[0].images.is_empty());
        assert_eq!(set_map(&dir, "inn", None).unwrap(), None);
    }

    #[test]
    fn a_campaign_without_sessions_just_gets_a_world() {
        let dir = temp_dir("empty");
        migrate(&dir, 1).unwrap();
        assert_eq!(read(&dir).unwrap().version, MIGRATED);
        assert!(!dir.join("backups").exists());
    }
}
