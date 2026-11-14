//! Addons (`addons/addons.md`): finding them in the data folder and validating each manifest. Serving their pages and
//! answering their calls is in `commands/addons.rs`.

use crate::packs::{safe_path, Invalid};
use crate::storage::subfolders;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The addon API version this app speaks. An addon written for another one isn't loaded.
pub const API: u32 = 1;
const MANIFEST: &str = "addon.json";
/// The Settings sections an addon may add a panel to.
const SETTINGS_SECTIONS: [&str; 1] = ["rules"];
const PERMISSIONS: [&str; 2] = ["packs:read", "packs:write"];

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    pub api: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub license: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    pub settings: Option<Panel>,
}

/// A page of the addon shown inside a Settings section.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Panel {
    pub section: String,
    /// The page, relative to the addon folder.
    pub entry: String,
}

#[derive(Debug)]
pub struct Addon {
    pub manifest: Manifest,
    pub folder: PathBuf,
}

impl Addon {
    pub fn allows(&self, permission: &str) -> bool {
        self.manifest.permissions.iter().any(|p| p == permission)
    }
}

/// Loads every addon folder in `dir`, sorted by folder name. Folders without an `addon.json` are skipped; an invalid
/// addon is reported and doesn't stop the others. A missing `dir` means no addons.
pub fn load_dir(dir: &Path) -> io::Result<(Vec<Addon>, Vec<Invalid>)> {
    let (mut addons, mut invalid) = (Vec::new(), Vec::new());
    for folder in subfolders(dir)? {
        let name = folder.file_name().to_string_lossy().into_owned();
        match load_addon(&folder.path(), &name) {
            Ok(None) => {}
            Ok(Some(manifest)) => addons.push(Addon { manifest, folder: folder.path() }),
            Err((manifest_name, errors)) => invalid.push(Invalid { folder: name, name: manifest_name, errors }),
        }
    }
    Ok((addons, invalid))
}

type LoadError = (Option<String>, Vec<String>);

fn load_addon(folder: &Path, folder_name: &str) -> Result<Option<Manifest>, LoadError> {
    let text = match fs::read_to_string(folder.join(MANIFEST)) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err((None, vec![format!("{MANIFEST}: {e}")])),
    };
    let manifest: Manifest = serde_json::from_str(&text).map_err(|e| (None, vec![format!("{MANIFEST}: {e}")]))?;
    let errors = check(&manifest, folder, folder_name);
    if errors.is_empty() { Ok(Some(manifest)) } else { Err((Some(manifest.name), errors)) }
}

fn check(m: &Manifest, folder: &Path, folder_name: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if m.api != API {
        errors.push(format!("Addon API {} isn't supported; this app speaks API {API}.", m.api));
    }
    if m.id != folder_name {
        errors.push(format!("id \"{}\" must be the folder name, \"{folder_name}\".", m.id));
    }
    if crate::storage::check_id(&m.id, "addon").is_err() {
        errors.push(format!("id \"{}\" must be lowercase letters, digits and dashes.", m.id));
    }
    for (field, value) in [("name", &m.name), ("version", &m.version), ("license", &m.license)] {
        if value.trim().is_empty() {
            errors.push(format!("\"{field}\" is empty."));
        }
    }
    for p in &m.permissions {
        let known = PERMISSIONS.contains(&p.as_str()) || p.strip_prefix("net:").is_some_and(valid_host);
        if !known {
            errors.push(format!("Unknown permission \"{p}\"."));
        }
    }
    if let Some(panel) = &m.settings {
        if !SETTINGS_SECTIONS.contains(&panel.section.as_str()) {
            errors.push(format!("settings: \"{}\" isn't a Settings section an addon can add to.", panel.section));
        }
        if !safe_path(&panel.entry) || !panel.entry.ends_with(".html") {
            errors.push(format!("settings: \"{}\" must be an .html page inside the addon folder.", panel.entry));
        } else if !folder.join(&panel.entry).is_file() {
            errors.push(format!("settings: \"{}\" doesn't exist.", panel.entry));
        }
    }
    errors
}

fn valid_host(host: &str) -> bool {
    !host.is_empty() && host.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
}

/// The host of an `https://` URL, lowercased; `None` for anything else, or one with a user name or port.
pub fn https_host(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?.to_ascii_lowercase();
    valid_host(&host).then_some(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("koldron-addons-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_addon(dir: &Path, folder: &str, manifest: serde_json::Value) {
        fs::create_dir_all(dir.join(folder).join("ui")).unwrap();
        fs::write(dir.join(folder).join(MANIFEST), manifest.to_string()).unwrap();
        fs::write(dir.join(folder).join("ui/settings.html"), "<p>hi</p>").unwrap();
    }

    fn manifest(id: &str) -> serde_json::Value {
        serde_json::json!({
            "api": API, "id": id, "name": "Importer", "version": "1.0.0", "license": "MIT",
            "permissions": ["packs:read", "packs:write", "net:api.github.com"],
            "settings": { "section": "rules", "entry": "ui/settings.html" }
        })
    }

    #[test]
    fn loads_valid_addons_skips_plain_folders_and_reports_invalid_ones() {
        let dir = temp_dir("load");
        write_addon(&dir, "importer", manifest("importer"));
        fs::create_dir_all(dir.join("notes")).unwrap();
        let mut bad = manifest("other");
        bad["api"] = (API + 1).into();
        bad["permissions"] = serde_json::json!(["files:write", "net:", "net:Evil.com/x"]);
        bad["settings"] = serde_json::json!({ "section": "ui", "entry": "../x.html" });
        write_addon(&dir, "broken", bad);
        let (addons, invalid) = load_dir(&dir).unwrap();
        assert_eq!(addons.len(), 1);
        assert!(addons[0].allows("net:api.github.com") && !addons[0].allows("net:example.com"));
        assert_eq!((invalid[0].folder.as_str(), invalid[0].name.as_deref()), ("broken", Some("Importer")));
        assert_eq!(invalid[0].errors.len(), 7, "{:?}", invalid[0].errors); // api, id, 3 permissions, section, entry
    }

    #[test]
    fn a_missing_folder_means_no_addons_and_a_missing_page_is_an_error() {
        assert!(load_dir(&temp_dir("none").join("addons")).unwrap().0.is_empty());
        let dir = temp_dir("page");
        write_addon(&dir, "importer", manifest("importer"));
        fs::remove_file(dir.join("importer/ui/settings.html")).unwrap();
        assert!(load_dir(&dir).unwrap().1[0].errors[0].contains("doesn't exist"));
    }

    #[test]
    fn only_plain_https_hosts_pass() {
        assert_eq!(https_host("https://API.github.com/repos/x?y#z").as_deref(), Some("api.github.com"));
        for bad in ["http://api.github.com/", "https://user@host/", "https://host:8080/", "https:///x", "ftp://x"] {
            assert_eq!(https_host(bad), None, "{bad}");
        }
    }
}
