//! The installed packs, loaded once per run, and Settings → Rules: each pack, valid or not, with whether it's switched on.

use crate::commands::settings;
use crate::packs::{load_dir, Invalid, Pack};
use crate::storage::{data_dir, StorageError};
use serde::Serialize;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

/// At most this many errors per invalid pack; the rest are counted.
const MAX_ERRORS: usize = 20;

pub type Loaded = (Vec<Pack>, Vec<Invalid>);

// shortcut: the bundled packs can't change while the app runs, so they're read once; reload here when Add pack… (TODO 12) can change them.
static LOADED: OnceLock<Loaded> = OnceLock::new();

/// The bundled packs, read and validated on first use. Blocking: call it from `spawn_blocking`.
pub fn loaded(app: &AppHandle) -> Result<&'static Loaded, StorageError> {
    if let Some(loaded) = LOADED.get() {
        return Ok(loaded);
    }
    let dir = app.path().resource_dir().map_err(|e| StorageError::Io(e.to_string()))?.join("packs");
    let packs = load_dir(&dir)?;
    Ok(LOADED.get_or_init(|| packs))
}

/// Pack ids switched off in Settings → Rules. A damaged settings file was already copied aside by `load`; then all are on.
pub fn disabled(app: &AppHandle) -> Result<Vec<String>, StorageError> {
    Ok(settings::load(&data_dir(app)?.join(settings::FILE_NAME)).map(|s| s.disabled_packs).unwrap_or_default())
}

/// The valid packs that are switched on.
pub fn enabled<'a>(packs: &'a [Pack], disabled: &[String]) -> Vec<&'a Pack> {
    packs.iter().filter(|p| !disabled.contains(&p.manifest.id)).collect()
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackSummary {
    /// The manifest id, or the folder name of an invalid pack.
    pub id: String,
    pub name: String,
    pub version: String,
    pub license: String,
    pub entries: usize,
    pub enabled: bool,
    /// Empty for a loaded pack. A pack with errors isn't loaded.
    pub errors: Vec<String>,
}

pub fn summaries((packs, invalid): &Loaded, disabled: &[String]) -> Vec<PackSummary> {
    let valid = packs.iter().map(|p| PackSummary {
        enabled: !disabled.contains(&p.manifest.id),
        entries: p.entry_count(),
        id: p.manifest.id.clone(),
        name: p.manifest.name.clone(),
        version: p.manifest.version.clone(),
        license: p.manifest.license.clone(),
        errors: Vec::new(),
    });
    let invalid = invalid.iter().map(|i| {
        let mut errors: Vec<String> = i.errors.iter().take(MAX_ERRORS).cloned().collect();
        if i.errors.len() > MAX_ERRORS {
            errors.push(format!("…and {} more.", i.errors.len() - MAX_ERRORS));
        }
        PackSummary {
            name: i.name.clone().unwrap_or_else(|| i.folder.clone()),
            id: i.folder.clone(),
            version: String::new(),
            license: String::new(),
            entries: 0,
            enabled: false,
            errors,
        }
    });
    valid.chain(invalid).collect()
}

#[tauri::command]
pub async fn list_packs(app: AppHandle) -> Result<Vec<PackSummary>, StorageError> {
    tauri::async_runtime::spawn_blocking(move || Ok(summaries(loaded(&app)?, &disabled(&app)?)))
        .await
        .map_err(|e| StorageError::Io(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn lists_the_srd_pack_switched_on_or_off() {
        let loaded = load_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs")).unwrap();
        let on = summaries(&loaded, &[]);
        assert_eq!(on.len(), 1);
        assert_eq!((on[0].id.as_str(), on[0].enabled, on[0].errors.len()), ("srd", true, 0));
        assert!(on[0].entries > 2000);
        assert!(!summaries(&loaded, &["srd".into()])[0].enabled);
        assert!(enabled(&loaded.0, &["srd".into()]).is_empty());
    }
}
