//! Shared local storage helpers: one error type and crash-safe file writes.

use serde::Serialize;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum StorageError {
    Io(String),
    /// A saved file could not be read as valid data. It is left untouched (or a copy is kept).
    Corrupt(String),
    Invalid(String),
    NotFound(String),
}

impl From<io::Error> for StorageError {
    fn from(e: io::Error) -> Self {
        StorageError::Io(e.to_string())
    }
}

// Concurrent saves to the same file share its temp file, so write one at a time.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

/// The app's data folder, where everything it saves lives.
pub fn data_dir(app: &AppHandle) -> Result<PathBuf, StorageError> {
    app.path().app_data_dir().map_err(|e| StorageError::Io(e.to_string()))
}

// Commands that read a file, change it and write it back hold this across both, so overlapping calls can't lose a change.
static UPDATE_LOCK: Mutex<()> = Mutex::new(());

/// Hold the returned guard from reading a file until it is written back.
pub fn lock_update() -> MutexGuard<'static, ()> {
    UPDATE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Writes atomically: temp file, flush to disk, rename over the old file. A crash leaves either the old or the new file.
pub fn write_atomic(path: &Path, contents: &str) -> Result<(), StorageError> {
    write_through(path, &path.with_extension("json.tmp"), contents.as_bytes())
}

/// `write_atomic` for any bytes (a picture); the temp file is the name plus ".tmp".
pub fn write_atomic_bytes(path: &Path, contents: &[u8]) -> Result<(), StorageError> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    write_through(path, Path::new(&tmp), contents)
}

fn write_through(path: &Path, tmp: &Path, contents: &[u8]) -> Result<(), StorageError> {
    let _guard = WRITE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    {
        let mut file = fs::File::create(tmp)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn to_json<T: Serialize>(value: &T) -> Result<String, StorageError> {
    serde_json::to_string_pretty(value).map_err(|e| StorageError::Io(e.to_string()))
}
