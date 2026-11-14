//! The order and widths of the widgets on a campaign's Home, saved in its folder (`home-layout.json`). A missing file
//! means nothing was saved yet: Home shows its default layout. A file that can't be read is kept as
//! `home-layout.corrupt.json`, so the next save never destroys it.

use crate::commands::campaigns::{active_dir, dirs};
use crate::storage::{blocking, to_json, write_atomic, StorageError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use tauri::AppHandle;

/// Every widget Home knows; each appears at most once.
pub const WIDGET_IDS: [&str; 4] = ["party", "nextSession", "summary", "encounter"];
const FILE_NAME: &str = "home-layout.json";

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Layout {
    widgets: Vec<String>,
    /// Columns (of 3) a widget spans; one without spans 1.
    #[serde(default)]
    widths: BTreeMap<String, u8>,
    /// Locked: the widgets can't be moved or resized.
    #[serde(default)]
    locked: bool,
    /// Widgets the DM took off Home; they stay off until put back (a file without this shows every widget).
    #[serde(default)]
    removed: Vec<String>,
}

fn validate(layout: &Layout) -> Result<(), String> {
    for (i, id) in layout.widgets.iter().enumerate() {
        if !WIDGET_IDS.contains(&id.as_str()) {
            return Err(format!("Unknown widget \"{id}\"."));
        }
        if layout.widgets[..i].contains(id) {
            return Err(format!("Widget \"{id}\" appears twice."));
        }
    }
    for (i, id) in layout.removed.iter().enumerate() {
        if !WIDGET_IDS.contains(&id.as_str()) {
            return Err(format!("Unknown widget \"{id}\"."));
        }
        if layout.removed[..i].contains(id) || layout.widgets.contains(id) {
            return Err(format!("Widget \"{id}\" is removed and also listed."));
        }
    }
    for (id, width) in &layout.widths {
        if !WIDGET_IDS.contains(&id.as_str()) {
            return Err(format!("Unknown widget \"{id}\"."));
        }
        if !(1..=3).contains(width) {
            return Err(format!("Widget \"{id}\" has width {width}, not 1 to 3."));
        }
    }
    Ok(())
}

/// `None` when nothing was saved yet.
pub fn load(campaign_dir: &Path) -> Result<Option<Layout>, StorageError> {
    let path = campaign_dir.join(FILE_NAME);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match serde_json::from_str::<Layout>(&text).map_err(|e| e.to_string()).and_then(|l| validate(&l).map(|()| l)) {
        Ok(layout) => Ok(Some(layout)),
        Err(reason) => {
            fs::copy(&path, path.with_extension("corrupt.json"))?;
            Err(StorageError::Corrupt(reason))
        }
    }
}

pub fn save(campaign_dir: &Path, layout: &Layout) -> Result<(), StorageError> {
    validate(layout).map_err(StorageError::Invalid)?;
    write_atomic(&campaign_dir.join(FILE_NAME), &to_json(layout)?)
}

#[tauri::command]
pub async fn get_home_layout(app: AppHandle, campaign: String) -> Result<Option<Layout>, StorageError> {
    blocking(move || load(&active_dir(&dirs(&app)?, &campaign)?)).await
}

#[tauri::command]
pub async fn set_home_layout(app: AppHandle, campaign: String, layout: Layout) -> Result<(), StorageError> {
    blocking(move || save(&active_dir(&dirs(&app)?, &campaign)?, &layout)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("koldron-home-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn layout(ids: &[&str], widths: &[(&str, u8)]) -> Layout {
        Layout {
            widgets: ids.iter().map(|s| s.to_string()).collect(),
            widths: widths.iter().map(|(id, w)| (id.to_string(), *w)).collect(),
            locked: false,
            removed: vec![],
        }
    }

    #[test]
    fn nothing_saved_then_saves_and_reads_back() {
        let dir = temp_dir("roundtrip");
        assert_eq!(load(&dir), Ok(None));
        let saved = Layout { locked: true, removed: vec!["encounter".into()], ..layout(&["summary", "party"], &[("party", 3)]) };
        save(&dir, &saved).unwrap();
        assert_eq!(load(&dir), Ok(Some(saved)));
    }

    #[test]
    fn rejects_unknown_repeated_or_badly_sized_widgets() {
        let dir = temp_dir("invalid");
        let removing = |ids: &[&str]| Layout { removed: ids.iter().map(|s| s.to_string()).collect(), ..layout(&["party"], &[]) };
        for bad in [
            layout(&["tools"], &[]),
            layout(&["party", "party"], &[]),
            layout(&["party"], &[("party", 4)]),
            removing(&["tools"]),
            removing(&["party"]),
            removing(&["summary", "summary"]),
        ] {
            assert!(matches!(save(&dir, &bad), Err(StorageError::Invalid(_))));
        }
        assert_eq!(load(&dir), Ok(None)); // nothing written
    }

    #[test]
    fn keeps_a_file_it_cannot_read() {
        let dir = temp_dir("corrupt");
        fs::write(dir.join(FILE_NAME), "{ not json").unwrap();
        assert!(matches!(load(&dir), Err(StorageError::Corrupt(_))));
        assert_eq!(fs::read_to_string(dir.join("home-layout.corrupt.json")).unwrap(), "{ not json");
    }
}
