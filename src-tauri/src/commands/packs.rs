//! The packs, loaded on first use and again after an addon changes them, and Settings → Rules: each pack, valid or not, with whether it's switched on.

use crate::commands::campaigns;
use crate::commands::settings;
use crate::commands::images;
use crate::commands::search;
use crate::packs::{load_all, Invalid, Pack};
use crate::storage::{blocking, data_dir, StorageError};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

/// At most this many errors per invalid pack or addon; the rest are counted.
const MAX_ERRORS: usize = 20;

pub type Loaded = (Vec<Pack>, Vec<Invalid>);

/// Read once, then again after an addon installs or removes a pack (`reload`).
static LOADED: Mutex<Option<Arc<Loaded>>> = Mutex::new(None);

/// Where addons install packs, in the data folder.
pub fn installed_dir(app: &AppHandle) -> Result<PathBuf, StorageError> {
    Ok(data_dir(app)?.join("packs"))
}

/// Where the app writes the homebrew books, in the data folder.
pub fn homebrew_dir(app: &AppHandle) -> Result<PathBuf, StorageError> {
    Ok(data_dir(app)?.join("homebrew"))
}

/// Where a homebrew save writes the book before swapping it in.
pub fn homebrew_staging(app: &AppHandle) -> Result<PathBuf, StorageError> {
    Ok(data_dir(app)?.join("homebrew-staging"))
}

/// The bundled and installed packs, read and validated on first use. Blocking: call it inside `blocking`.
pub fn loaded(app: &AppHandle) -> Result<Arc<Loaded>, StorageError> {
    // Held while loading, so a `reload` can't be undone by a load that read the folders before the change.
    let mut cache = LOADED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(loaded) = cache.as_ref() {
        return Ok(loaded.clone());
    }
    let bundled = app.path().resource_dir().map_err(|e| StorageError::Io(e.to_string()))?.join("packs");
    // A homebrew book a crash left moved aside comes back before anything reads the folder.
    crate::packs::recover(&homebrew_dir(app)?, &crate::packs::old_dir(&homebrew_staging(app)?))?;
    let loaded = Arc::new(load_all(&bundled, &installed_dir(app)?, &homebrew_dir(app)?)?);
    *cache = Some(loaded.clone());
    Ok(loaded)
}

/// Call after the installed packs changed: they're read again on next use, and the search index is rebuilt.
pub fn reload() {
    *LOADED.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    search::changed();
}

/// Pack ids switched off in Settings → Rules. A damaged settings file was already copied aside by `load`; then all are on.
pub fn disabled(app: &AppHandle) -> Result<Vec<String>, StorageError> {
    Ok(settings::load(&data_dir(app)?.join(settings::FILE_NAME)).map(|s| s.disabled_packs).unwrap_or_default())
}

/// Pack ids switched off app-wide, plus those `campaign` leaves out: what a campaign's screens may show.
pub fn disabled_in(app: &AppHandle, campaign: Option<&str>) -> Result<Vec<String>, StorageError> {
    let mut off = disabled(app)?;
    if let Some(id) = campaign {
        off.extend(campaigns::packs_off(&campaigns::dirs(app)?, id)?);
    }
    Ok(off)
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
    pub ruleset: String,
    pub entries: usize,
    pub enabled: bool,
    /// Installed by an addon in the data folder; false for a bundled pack.
    pub installed: bool,
    /// A homebrew book (also `installed`).
    pub homebrew: bool,
    /// Empty for a loaded pack. A pack with errors isn't loaded.
    pub errors: Vec<String>,
}

/// The first `MAX_ERRORS` errors, then how many more there are.
pub fn capped(errors: &[String]) -> Vec<String> {
    let mut shown: Vec<String> = errors.iter().take(MAX_ERRORS).cloned().collect();
    if errors.len() > MAX_ERRORS {
        shown.push(format!("…and {} more.", errors.len() - MAX_ERRORS));
    }
    shown
}

pub fn summaries((packs, invalid): &Loaded, disabled: &[String]) -> Vec<PackSummary> {
    let valid = packs.iter().map(|p| PackSummary {
        enabled: !disabled.contains(&p.manifest.id),
        installed: p.installed,
        homebrew: p.homebrew,
        ruleset: p.manifest.ruleset.clone(),
        entries: p.entry_count(),
        id: p.manifest.id.clone(),
        name: p.manifest.name.clone(),
        version: p.manifest.version.clone(),
        license: p.manifest.license.clone(),
        errors: Vec::new(),
    });
    let invalid = invalid.iter().map(|i| {
        let errors = capped(&i.errors);
        PackSummary {
            name: i.name.clone().unwrap_or_else(|| i.folder.clone()),
            id: i.folder.clone(),
            version: String::new(),
            license: String::new(),
            ruleset: String::new(),
            entries: 0,
            enabled: false,
            installed: false,
            homebrew: false,
            errors,
        }
    });
    valid.chain(invalid).collect()
}

#[tauri::command]
pub async fn list_packs(app: AppHandle) -> Result<Vec<PackSummary>, StorageError> {
    blocking(move || Ok(summaries(&*loaded(&app)?, &disabled(&app)?))).await
}

/// One pack install or removal at a time (they share the staging folder); its own lock, so campaign saves never wait on one.
static PACK_CHANGE: Mutex<()> = Mutex::new(());

pub fn lock_pack_change() -> std::sync::MutexGuard<'static, ()> {
    PACK_CHANGE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Removes an installed pack (a bundled one with the same id comes back) and its kept images, unless that bundled pack
/// still uses them. Blocking: call it inside `blocking`.
pub fn remove_installed(app: &AppHandle, pack: &str) -> Result<(), StorageError> {
    let _guard = lock_pack_change();
    crate::packs::remove(&installed_dir(app)?, pack)?;
    reload();
    if !loaded(app)?.0.iter().any(|p| p.manifest.id == pack) {
        remove_images(app, pack)?;
    }
    Ok(())
}

/// Deletes a pack's kept images (`images/<pack>/`), if it has any.
fn remove_images(app: &AppHandle, pack: &str) -> Result<(), StorageError> {
    match std::fs::remove_dir_all(data_dir(app)?.join(images::DIR).join(pack)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

/// Settings → Rules: removes an installed pack, or deletes a homebrew book and everything in it.
#[tauri::command]
pub async fn remove_pack(app: AppHandle, pack: String) -> Result<(), StorageError> {
    blocking(move || {
        if loaded(&app)?.0.iter().any(|p| p.homebrew && p.manifest.id == pack) {
            let _guard = lock_pack_change();
            crate::packs::remove(&homebrew_dir(&app)?, &pack)?;
            reload();
            return remove_images(&app, &pack);
        }
        remove_installed(&app, &pack)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::load_dir;
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
