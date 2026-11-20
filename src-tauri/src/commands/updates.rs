//! App updates: the latest published GitHub release (`plugins.updater` in tauri.conf.json; GitHub's "latest" skips
//! drafts and pre-releases). The installer's signature is checked against the public key before it runs.

use crate::storage::StorageError;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

fn failed(e: tauri_plugin_updater::Error) -> StorageError {
    StorageError::Io(e.to_string())
}

async fn latest(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Update>, StorageError> {
    app.updater().map_err(failed)?.check().await.map_err(failed)
}

/// The newer version, if the latest release is newer than the running app.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<String>, StorageError> {
    Ok(latest(&app).await?.map(|update| update.version))
}

/// Downloads and installs the latest release, then restarts the app (Windows' installer may end it sooner).
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), StorageError> {
    let update = latest(&app).await?.ok_or_else(|| StorageError::NotFound("No newer version is available.".into()))?;
    update.download_and_install(|_, _| {}, || {}).await.map_err(failed)?;
    app.restart()
}
