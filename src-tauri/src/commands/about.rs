//! License texts for Settings → About: the app's own, then the one beside each pack (later also each plugin).

use crate::storage::StorageError;
use serde::Serialize;
use std::fs;
use std::io;
use std::path::Path;
use tauri::{AppHandle, Manager};

/// The file names a pack's license may have, in order of preference.
const LICENSE_FILES: [&str; 3] = ["LICENSE.md", "LICENSE", "LICENSE.txt"];

#[derive(Debug, Serialize, PartialEq)]
pub struct LicenseText {
    /// "app" for the app itself, otherwise the pack's folder name.
    pub id: String,
    pub text: String,
}

/// Reads `<root>/LICENSE` and the license of each folder in `<root>/packs`, sorted by folder name.
/// A pack without a license file is left out; a missing app license is an error.
pub fn read_licenses(root: &Path) -> Result<Vec<LicenseText>, StorageError> {
    let mut texts = vec![LicenseText { id: "app".into(), text: fs::read_to_string(root.join("LICENSE"))? }];
    let mut packs: Vec<_> = match fs::read_dir(root.join("packs")) {
        Ok(entries) => entries.filter_map(Result::ok).filter(|e| e.path().is_dir()).collect(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    packs.sort_by_key(|e| e.file_name());
    for pack in packs {
        if let Some(text) = LICENSE_FILES.iter().find_map(|name| fs::read_to_string(pack.path().join(name)).ok()) {
            texts.push(LicenseText { id: pack.file_name().to_string_lossy().into_owned(), text });
        }
    }
    Ok(texts)
}

#[tauri::command]
pub async fn license_texts(app: AppHandle) -> Result<Vec<LicenseText>, StorageError> {
    let root = app.path().resource_dir().map_err(|e| StorageError::Io(e.to_string()))?;
    tauri::async_runtime::spawn_blocking(move || read_licenses(&root))
        .await
        .map_err(|e| StorageError::Io(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("koldron-about-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn reads_the_app_license_then_each_pack_license_by_folder_name() {
        let root = temp_root("all");
        fs::write(root.join("LICENSE"), "MPL").unwrap();
        for (pack, file, text) in [("tabler-icons", "LICENSE", "MIT"), ("srd", "LICENSE.md", "CC"), ("no-license", "pack.json", "{}")] {
            fs::create_dir_all(root.join("packs").join(pack)).unwrap();
            fs::write(root.join("packs").join(pack).join(file), text).unwrap();
        }
        let got: Vec<_> = read_licenses(&root).unwrap().into_iter().map(|l| (l.id, l.text)).collect();
        let want = [("app", "MPL"), ("srd", "CC"), ("tabler-icons", "MIT")].map(|(a, b)| (a.to_string(), b.to_string()));
        assert_eq!(got, want);
    }

    #[test]
    fn works_without_packs_but_not_without_the_app_license() {
        let root = temp_root("bare");
        assert!(matches!(read_licenses(&root), Err(StorageError::Io(_))));
        fs::write(root.join("LICENSE"), "MPL").unwrap();
        assert_eq!(read_licenses(&root).unwrap().len(), 1);
    }
}
