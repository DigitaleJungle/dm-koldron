//! Settings → Rules: the installed packs, valid or not, with whether each is switched on.

use crate::commands::settings;
use crate::packs::{load_dir, Invalid, Pack};
use crate::storage::{data_dir, StorageError};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Manager};

/// At most this many errors per invalid pack; the rest are counted.
const MAX_ERRORS: usize = 20;

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

pub fn summaries(packs_dir: &Path, disabled: &[String]) -> Result<Vec<PackSummary>, StorageError> {
    let (packs, invalid) = load_dir(packs_dir)?;
    let valid = packs.into_iter().map(|p: Pack| PackSummary {
        enabled: !disabled.contains(&p.manifest.id),
        entries: p.entry_count(),
        id: p.manifest.id,
        name: p.manifest.name,
        version: p.manifest.version,
        license: p.manifest.license,
        errors: Vec::new(),
    });
    let invalid = invalid.into_iter().map(|i: Invalid| {
        let mut errors = i.errors;
        if errors.len() > MAX_ERRORS {
            let more = errors.len() - MAX_ERRORS;
            errors.truncate(MAX_ERRORS);
            errors.push(format!("…and {more} more."));
        }
        PackSummary {
            name: i.name.unwrap_or_else(|| i.folder.clone()),
            id: i.folder,
            version: String::new(),
            license: String::new(),
            entries: 0,
            enabled: false,
            errors,
        }
    });
    Ok(valid.chain(invalid).collect())
}

#[tauri::command]
pub async fn list_packs(app: AppHandle) -> Result<Vec<PackSummary>, StorageError> {
    let packs_dir = app.path().resource_dir().map_err(|e| StorageError::Io(e.to_string()))?.join("packs");
    // A damaged settings file was already copied aside by `load`; list the packs as all on rather than none.
    let disabled = settings::load(&data_dir(&app)?.join(settings::FILE_NAME)).map(|s| s.disabled_packs).unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || summaries(&packs_dir, &disabled))
        .await
        .map_err(|e| StorageError::Io(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_the_srd_pack_switched_on_or_off() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs");
        let on = summaries(&dir, &[]).unwrap();
        assert_eq!(on.len(), 1);
        assert_eq!((on[0].id.as_str(), on[0].enabled, on[0].errors.len()), ("srd", true, 0));
        assert!(on[0].entries > 2000);
        assert!(!summaries(&dir, &["srd".into()]).unwrap()[0].enabled);
    }
}
