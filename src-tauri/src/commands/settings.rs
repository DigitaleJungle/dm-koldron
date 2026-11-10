//! App-wide settings (not per campaign), saved as `settings.json` in the app data folder.

use crate::storage::{blocking, data_dir, lock_update, to_json, write_atomic, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

pub const FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow the operating system's light/dark setting.
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Animations {
    /// Follow the operating system's reduce-motion setting.
    #[default]
    System,
    On,
    Off,
}

// Missing fields fall back to their defaults, so older files keep loading when settings are added.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub theme: Theme,
    pub animations: Animations,
    /// Pack ids switched off in Settings → Rules; packs are on unless listed.
    pub disabled_packs: Vec<String>,
    /// Pack ids with Enable images ticked in Settings → Rules; images are off unless listed.
    pub image_packs: Vec<String>,
}

/// Merges the given fields into the saved settings and saves them; the rest stay as they are.
pub fn update(path: &Path, patch: Value) -> Result<AppSettings, StorageError> {
    let Value::Object(patch) = patch else {
        return Err(StorageError::Invalid("Settings must be an object.".into()));
    };
    let _guard = lock_update();
    // A damaged file was already copied aside by `load`, so saving over it with defaults loses nothing.
    let current = match load(path) {
        Err(StorageError::Corrupt(_)) => AppSettings::default(),
        other => other?,
    };
    let mut merged = serde_json::to_value(current).map_err(|e| StorageError::Io(e.to_string()))?;
    if let Value::Object(map) = &mut merged {
        map.extend(patch);
    }
    let settings: AppSettings = serde_json::from_value(merged).map_err(|e| StorageError::Invalid(e.to_string()))?;
    write_atomic(path, &to_json(&settings)?)?;
    Ok(settings)
}

/// Defaults when nothing has been saved yet.
pub fn load(path: &Path) -> Result<AppSettings, StorageError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(AppSettings::default()),
        Err(e) => return Err(e.into()),
    };
    serde_json::from_str(&text).or_else(|e| {
        // Keep the bad file, so the next save never destroys something we couldn't read; once is enough (every
        // command that reads the settings comes here while it stays damaged).
        let copy = path.with_extension("corrupt.json");
        if fs::read(&copy).ok().as_deref() != Some(text.as_bytes()) {
            fs::copy(path, &copy)?;
        }
        Err(StorageError::Corrupt(e.to_string()))
    })
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, StorageError> {
    Ok(data_dir(app)?.join(FILE_NAME))
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> Result<AppSettings, StorageError> {
    let path = settings_path(&app)?;
    blocking(move || load(&path)).await
}

#[tauri::command]
pub async fn set_settings(app: AppHandle, settings: Value) -> Result<AppSettings, StorageError> {
    let path = settings_path(&app)?;
    blocking(move || update(&path, settings)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("koldron-settings-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join(FILE_NAME)
    }

    #[test]
    fn defaults_to_system_when_nothing_saved() {
        assert_eq!(load(&temp_file("missing")).unwrap(), AppSettings::default());
    }

    #[test]
    fn update_only_changes_the_given_fields_and_survives_a_reload() {
        let path = temp_file("patch");
        update(&path, serde_json::json!({ "animations": "off" })).unwrap();
        let after = update(&path, serde_json::json!({ "theme": "dark" })).unwrap();
        assert_eq!(after, AppSettings { theme: Theme::Dark, animations: Animations::Off, ..AppSettings::default() });
        assert_eq!(load(&path).unwrap(), after);
        assert!(fs::read_to_string(&path).unwrap().contains("\"dark\""));
    }

    #[test]
    fn rejects_bad_values_and_keeps_the_file() {
        let path = temp_file("bad");
        update(&path, serde_json::json!({ "theme": "light" })).unwrap();
        assert!(matches!(update(&path, serde_json::json!({ "theme": "purple" })), Err(StorageError::Invalid(_))));
        assert!(matches!(update(&path, serde_json::json!([])), Err(StorageError::Invalid(_))));
        assert_eq!(load(&path).unwrap().theme, Theme::Light);
    }

    #[test]
    fn an_unreadable_file_is_kept_and_reported() {
        let path = temp_file("corrupt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ nope").unwrap();
        assert!(matches!(load(&path), Err(StorageError::Corrupt(_))));
        assert_eq!(fs::read_to_string(path.with_extension("corrupt.json")).unwrap(), "{ nope");
        // Reading it again doesn't copy it again (the copy keeps its time).
        let before = fs::metadata(path.with_extension("corrupt.json")).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(matches!(load(&path), Err(StorageError::Corrupt(_))));
        assert_eq!(fs::metadata(path.with_extension("corrupt.json")).unwrap().modified().unwrap(), before);
    }

    #[test]
    fn saving_replaces_an_unreadable_file_and_keeps_its_copy() {
        let path = temp_file("corrupt-save");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ nope").unwrap();
        let saved = update(&path, serde_json::json!({ "theme": "dark" })).unwrap();
        assert_eq!(saved, AppSettings { theme: Theme::Dark, ..AppSettings::default() });
        assert_eq!(load(&path).unwrap(), saved);
        assert_eq!(fs::read_to_string(path.with_extension("corrupt.json")).unwrap(), "{ nope");
    }
}
