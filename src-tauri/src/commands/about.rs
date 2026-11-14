//! License texts for Settings → About: the app's own, then the one beside each pack. Addons' are in their summary.

use crate::storage::{blocking, subfolders, StorageError};
use serde::Serialize;
use std::fs;
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

/// Reads `<root>/LICENSE`, then the license of each folder in `<root>/packs` and in each of `more` (the installed
/// packs), by folder name. A folder already listed (an updated SRD) keeps its first entry; one without a license file
/// is left out; a missing app license is an error.
pub fn read_licenses(root: &Path, more: &[&Path]) -> Result<Vec<LicenseText>, StorageError> {
    let mut texts = vec![LicenseText { id: "app".into(), text: fs::read_to_string(root.join("LICENSE"))? }];
    for dir in [root.join("packs").as_path()].into_iter().chain(more.iter().copied()) {
        for pack in subfolders(dir)? {
            let id = pack.file_name().to_string_lossy().into_owned();
            if texts.iter().any(|t| t.id == id) {
                continue;
            }
            if let Some(text) = license_in(&pack.path()) {
                texts.push(LicenseText { id, text });
            }
        }
    }
    Ok(texts)
}

/// The license file in `folder`, if it has one.
pub fn license_in(folder: &Path) -> Option<String> {
    LICENSE_FILES.iter().find_map(|name| fs::read_to_string(folder.join(name)).ok())
}

#[tauri::command]
pub async fn license_texts(app: AppHandle) -> Result<Vec<LicenseText>, StorageError> {
    let root = app.path().resource_dir().map_err(|e| StorageError::Io(e.to_string()))?;
    let installed = crate::commands::packs::installed_dir(&app)?;
    blocking(move || read_licenses(&root, &[&installed])).await
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
        let got: Vec<_> = read_licenses(&root, &[]).unwrap().into_iter().map(|l| (l.id, l.text)).collect();
        let want = [("app", "MPL"), ("srd", "CC"), ("tabler-icons", "MIT")].map(|(a, b)| (a.to_string(), b.to_string()));
        assert_eq!(got, want);
    }

    #[test]
    fn installed_packs_add_their_license_unless_the_folder_is_already_listed() {
        let root = temp_root("installed");
        for (pack, text) in [("srd", "newer"), ("homebrew", "CC0")] {
            fs::create_dir_all(root.join(pack)).unwrap();
            fs::write(root.join(pack).join("LICENSE"), text).unwrap();
        }
        fs::write(root.join("LICENSE"), "MPL").unwrap();
        fs::create_dir_all(root.join("packs/srd")).unwrap();
        fs::write(root.join("packs/srd/LICENSE.md"), "CC").unwrap();
        let missing = root.join("missing");
        let texts = read_licenses(&root, &[&root, &missing]).unwrap();
        let got: Vec<_> = texts.iter().map(|l| (l.id.as_str(), l.text.as_str())).collect();
        assert_eq!(got, [("app", "MPL"), ("srd", "CC"), ("homebrew", "CC0")]); // root's own "packs" folder has no license
    }

    #[test]
    fn works_without_packs_but_not_without_the_app_license() {
        let root = temp_root("bare");
        assert!(matches!(read_licenses(&root, &[]), Err(StorageError::Io(_))));
        fs::write(root.join("LICENSE"), "MPL").unwrap();
        assert_eq!(read_licenses(&root, &[]).unwrap().len(), 1);
    }
}
