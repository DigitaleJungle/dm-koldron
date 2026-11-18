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
pub const MANIFEST: &str = "addon.json";
/// The Settings sections an addon may add a panel to.
const SETTINGS_SECTIONS: [&str; 3] = ["rules", "library", "integrations"];
const PERMISSIONS: [&str; 7] = ["packs:read", "packs:write", "characters:write", "folders:read", "folders:write", "secrets", "search"];
/// The longest label an addon's Add character item or Library tab may have.
const MAX_LABEL: usize = 40;

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
    /// Pages shown in Settings sections: one panel, or a list (one per section).
    #[serde(default, deserialize_with = "one_or_many")]
    pub settings: Vec<Panel>,
    /// An item in the Characters screen's Add character menu; its page opens in a dialog.
    pub characters: Option<MenuPage>,
    /// A page that runs hidden as long as the app is open.
    pub background: Option<String>,
    /// A page shown on the Start screen, beside the campaigns; it sets its own height (0 hides it).
    pub start: Option<String>,
    /// A page shown in the search screen's preview pane for the addon's search results (`{ id, state }` in its context).
    pub preview: Option<String>,
    /// A tab in the app-wide Library labelled `label`, showing the page `entry`.
    pub library: Option<Tab>,
}

/// An addon's tab in the Library.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Tab {
    pub label: String,
    /// The page, relative to the addon folder.
    pub entry: String,
    /// The page has a side column: the Library's header gets a button to hide it, sent as `sidebarHidden` in the context.
    #[serde(default)]
    pub sidebar: bool,
}

/// The addon's way to add characters: a menu item labelled `label` opening `entry`, and the fields of the characters it
/// adds that it owns (keeps up to date): the app can't change those while the addon is loaded.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MenuPage {
    pub label: String,
    /// The page, relative to the addon folder.
    pub entry: String,
    #[serde(default)]
    pub owns: Vec<String>,
}

/// A page of the addon shown inside a Settings section.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Panel {
    pub section: String,
    /// The page, relative to the addon folder.
    pub entry: String,
}

/// `settings` as one panel (as addons first wrote it) or a list of them.
fn one_or_many<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<Panel>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Panels {
        One(Panel),
        Many(Vec<Panel>),
    }
    Ok(match Panels::deserialize(deserializer)? {
        Panels::One(panel) => vec![panel],
        Panels::Many(list) => list,
    })
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

/// An addon not in `addons/` yet (unzipped to install): its manifest, checked as if its folder were already named
/// after its id. Errors list every problem found.
pub fn read_new(folder: &Path) -> Result<Manifest, Vec<String>> {
    let text = fs::read_to_string(folder.join(MANIFEST)).map_err(|e| vec![format!("{MANIFEST}: {e}")])?;
    let manifest: Manifest = serde_json::from_str(&text).map_err(|e| vec![format!("{MANIFEST}: {e}")])?;
    let errors = check(&manifest, folder, &manifest.id);
    if errors.is_empty() { Ok(manifest) } else { Err(errors) }
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
    for (i, panel) in m.settings.iter().enumerate() {
        if !SETTINGS_SECTIONS.contains(&panel.section.as_str()) {
            errors.push(format!("settings: \"{}\" isn't a Settings section an addon can add to.", panel.section));
        } else if m.settings[..i].iter().any(|p| p.section == panel.section) {
            errors.push(format!("settings: more than one panel in \"{}\".", panel.section));
        }
        check_page("settings", &panel.entry, folder, &mut errors);
    }
    if let Some(page) = &m.characters {
        check_label("characters", &page.label, &mut errors);
        check_page("characters", &page.entry, folder, &mut errors);
        for field in page.owns.iter().filter(|f| !crate::commands::characters::OWNABLE.contains(&f.as_str())) {
            errors.push(format!("characters: \"{field}\" isn't a character field an addon can own."));
        }
    }
    if let Some(page) = &m.background {
        check_page("background", page, folder, &mut errors);
    }
    if let Some(page) = &m.start {
        check_page("start", page, folder, &mut errors);
    }
    if let Some(page) = &m.preview {
        check_page("preview", page, folder, &mut errors);
    }
    if let Some(tab) = &m.library {
        check_label("library", &tab.label, &mut errors);
        check_page("library", &tab.entry, folder, &mut errors);
    }
    errors
}

fn check_label(field: &str, label: &str, errors: &mut Vec<String>) {
    let label = label.trim();
    if label.is_empty() || label.chars().count() > MAX_LABEL {
        errors.push(format!("{field}: the label must be 1 to {MAX_LABEL} characters."));
    }
}

/// `page` must be an existing .html file inside the addon folder.
fn check_page(field: &str, page: &str, folder: &Path, errors: &mut Vec<String>) {
    if !safe_path(page) || !page.ends_with(".html") {
        errors.push(format!("{field}: \"{page}\" must be an .html page inside the addon folder."));
    } else if !folder.join(page).is_file() {
        errors.push(format!("{field}: \"{page}\" doesn't exist."));
    }
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
            "permissions": ["packs:read", "packs:write", "characters:write", "folders:read", "folders:write", "secrets", "net:api.github.com"],
            "settings": [{ "section": "rules", "entry": "ui/settings.html" }, { "section": "integrations", "entry": "ui/settings.html" }],
            "characters": { "label": "Sync", "entry": "ui/settings.html", "owns": ["currentHp"] },
            "background": "ui/settings.html",
            "start": "ui/settings.html",
            "preview": "ui/settings.html",
            "library": { "label": "Tab", "entry": "ui/settings.html", "sidebar": true }
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
        bad["settings"] = serde_json::json!([{ "section": "ui", "entry": "../x.html" }, { "section": "library", "entry": "ui/settings.html" }, { "section": "library", "entry": "ui/settings.html" }]);
        bad["characters"] = serde_json::json!({ "label": " ", "entry": "ui/missing.html", "owns": ["currentHp", "picture"] });
        bad["background"] = "bg.js".into();
        bad["start"] = "start.js".into();
        bad["preview"] = "../preview.html".into();
        bad["library"] = serde_json::json!({ "label": "", "entry": "ui/tab.html" });
        write_addon(&dir, "broken", bad);
        let (addons, invalid) = load_dir(&dir).unwrap();
        assert_eq!(addons.len(), 1);
        assert!(addons[0].allows("net:api.github.com") && !addons[0].allows("net:example.com"));
        assert!(addons[0].manifest.library.as_ref().unwrap().sidebar);
        assert_eq!(addons[0].manifest.settings.len(), 2);
        assert_eq!((invalid[0].folder.as_str(), invalid[0].name.as_deref()), ("broken", Some("Importer")));
        // api, id, 3 permissions, section, entry, a second library panel, label, characters page, an unknown owned
        // field, background page, start page, preview page, library label and page
        assert_eq!(invalid[0].errors.len(), 16, "{:?}", invalid[0].errors);
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
