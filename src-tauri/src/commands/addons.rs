//! The addon runtime: the addons in the data folder, the `koldron-addon` scheme that serves their pages, and the calls
//! their sandboxed frames make through the window's bridge. Every call names the addon and is checked here against the
//! permissions its manifest declares; the window is not trusted to have checked.

use crate::addons::{https_host, load_dir, read_new, Addon, Manifest, MANIFEST};
use crate::commands::about::license_in;
use crate::commands::campaigns::{active_dir, dirs, now_ms};
use crate::commands::characters::{self, Character, Editor};
use crate::commands::search;
use crate::commands::packs::{capped, disabled, installed_dir, loaded, reload, summaries, PackSummary};
use crate::packs::{self, safe_path, Invalid, NewPack};
use crate::storage::{blocking, data_dir, write_atomic, write_atomic_bytes, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, UNIX_EPOCH};
use tauri::http::{header, Request, Response, StatusCode};
use tauri::ipc;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

pub const SCHEME: &str = "koldron-addon";
/// The most `net.fetch` reads, and the largest file `files.pickFile` hands over.
const MAX_BYTES: u64 = 200 * 1024 * 1024;
/// `files.pickFolder`: at most this many files, together at most `MAX_BYTES`.
const MAX_FILES: usize = 10_000;
/// The most `storage.set` keeps.
const MAX_STORAGE: usize = 10 * 1024 * 1024;

type Loaded = (Vec<Addon>, Vec<Invalid>);

/// One pack install or removal at a time (they share the staging folder); its own lock, so campaign saves never wait on one.
static PACK_CHANGE: Mutex<()> = Mutex::new(());

fn lock_pack_change() -> std::sync::MutexGuard<'static, ()> {
    PACK_CHANGE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// shortcut: addons are read once per run, so installing, enabling, disabling or uninstalling one needs a restart
// (Settings → Addons offers one). Reload here if that gets in the way.
static ADDONS: OnceLock<Loaded> = OnceLock::new();

/// The addons running this session: the valid, enabled ones as they were at start (and the invalid ones). Blocking:
/// call it inside `blocking`.
fn addons(app: &AppHandle) -> Result<&'static Loaded, StorageError> {
    if let Some(loaded) = ADDONS.get() {
        return Ok(loaded);
    }
    let data = data_dir(app)?;
    let (mut valid, invalid) = load_dir(&data.join("addons"))?;
    let off = disabled_addons(&data);
    valid.retain(|a| !off.contains(&a.manifest.id));
    Ok(ADDONS.get_or_init(|| (valid, invalid)))
}

/// The user's disabled addons (Settings → Addons): ids, in the data folder. They aren't loaded.
const DISABLED_FILE: &str = "disabled-addons.json";

/// For loading: a list that can't be read disables nothing (changing it is refused, see `edit_list`).
fn disabled_addons(data: &Path) -> Vec<String> {
    read_list(&data.join(DISABLED_FILE)).unwrap_or_default()
}

fn set_disabled(data: &Path, id: &str, off: bool) -> Result<(), StorageError> {
    edit_list(&data.join(DISABLED_FILE), id, off)
}

/// A JSON list of names: empty when there's no file, `Corrupt` when it can't be read.
fn read_list(path: &Path) -> Result<Vec<String>, StorageError> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| StorageError::Corrupt(format!("{}: {e}", path.display()))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.into()),
    }
}

/// Adds `item` to the list in `path` (`keep`) or takes it out. A list that can't be read is never saved over.
fn edit_list(path: &Path, item: &str, keep: bool) -> Result<(), StorageError> {
    let mut items = read_list(path)?;
    if items.iter().any(|i| i == item) == keep {
        return Ok(());
    }
    items.retain(|i| i != item);
    if keep {
        items.push(item.to_string());
    }
    write_atomic(path, &serde_json::to_string(&items).map_err(|e| StorageError::Io(e.to_string()))?)
}

/// Each loaded addon's id with the character fields it owns (a character linked to an addon that isn't loaded is an
/// ordinary one). Blocking.
pub fn owned_fields(app: &AppHandle) -> Result<Vec<(String, Vec<String>)>, StorageError> {
    Ok(addons(app)?.0.iter().map(|a| (a.manifest.id.clone(), owns(a).to_vec())).collect())
}

fn owns(addon: &Addon) -> &[String] {
    addon.manifest.characters.as_ref().map_or(&[], |c| c.owns.as_slice())
}

/// A running addon. One uninstalled since start (its folder gone) is refused, so it can't write back data the
/// uninstall deleted.
fn addon(app: &AppHandle, id: &str) -> Result<&'static Addon, StorageError> {
    addons(app)?.0.iter().find(|a| a.manifest.id == id && a.folder.is_dir()).ok_or_else(|| StorageError::NotFound(format!("No addon \"{id}\" is loaded.")))
}

fn require(addon: &Addon, permission: &str) -> Result<(), StorageError> {
    if addon.allows(permission) {
        Ok(())
    } else {
        Err(StorageError::Invalid(format!("{} has no permission \"{permission}\".", addon.manifest.name)))
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonSummary {
    /// The manifest id, or the folder name of an invalid addon.
    pub id: String,
    pub name: String,
    pub version: String,
    pub license: String,
    pub settings: Vec<crate::addons::Panel>,
    pub characters: Option<crate::addons::MenuPage>,
    pub background: Option<String>,
    pub start: Option<String>,
    pub preview: Option<String>,
    pub library: Option<crate::addons::Tab>,
    pub home: Option<crate::addons::Widget>,
    pub campaign: Option<crate::addons::CampaignScreen>,
    /// The campaigns the addon showed (true) or hid (false) its `campaign` screen in; others follow the manifest's
    /// `visible`.
    pub campaign_visibility: BTreeMap<String, bool>,
    /// Whether the `library` tab is shown (`library.setVisible`); true for an addon without one.
    pub library_visible: bool,
    /// The license file in its folder, if it has one.
    pub license_text: Option<String>,
    /// Empty for a valid addon. An addon with errors isn't loaded.
    pub errors: Vec<String>,
    /// Whether the user has it enabled; a disabled one isn't loaded on the next start.
    pub enabled: bool,
    /// The version running this session; `None` when it isn't running (disabled, invalid, or installed since start).
    pub loaded_version: Option<String>,
    /// Uninstalled since start: its pages and calls stop at once, the rest goes on the restart.
    pub removed: bool,
}

fn summary(addon: &Addon, data: &Path) -> AddonSummary {
    let m = &addon.manifest;
    AddonSummary {
        id: m.id.clone(),
        name: m.name.clone(),
        version: m.version.clone(),
        license: m.license.clone(),
        settings: m.settings.clone(),
        characters: m.characters.clone(),
        background: m.background.clone(),
        start: m.start.clone(),
        preview: m.preview.clone(),
        library: m.library.clone(),
        home: m.home.clone(),
        campaign: m.campaign.clone(),
        campaign_visibility: campaign_visibility(data, &m.id),
        library_visible: library_visible(data, &m.id),
        license_text: license_in(&addon.folder),
        errors: Vec::new(),
        enabled: true,
        loaded_version: None,
        removed: false,
    }
}

/// The addons folder as it is now (`on_disk`), next to the addons running since start. `data`: the data folder.
fn summaries_of(on_disk: &Loaded, running: &[Addon], data: &Path) -> Vec<AddonSummary> {
    let (addons, invalid) = on_disk;
    let dir = data.join("addons");
    let off = disabled_addons(data);
    let loaded_version = |id: &str| running.iter().find(|r| r.manifest.id == id).map(|r| r.manifest.version.clone());
    let valid = addons.iter().map(|a| AddonSummary {
        enabled: !off.contains(&a.manifest.id),
        loaded_version: loaded_version(&a.manifest.id),
        ..summary(a, data)
    });
    let invalid_rows = invalid.iter().map(|i| AddonSummary {
        name: i.name.clone().unwrap_or_else(|| i.folder.clone()),
        license_text: license_in(&dir.join(&i.folder)),
        id: i.folder.clone(),
        version: String::new(),
        license: String::new(),
        settings: Vec::new(),
        characters: None,
        background: None,
        start: None,
        preview: None,
        library: None,
        home: None,
        campaign: None,
        campaign_visibility: BTreeMap::new(),
        library_visible: true,
        errors: capped(&i.errors),
        enabled: !off.contains(&i.folder),
        loaded_version: loaded_version(&i.folder),
        removed: false,
    });
    let present = |id: &str| addons.iter().any(|a| a.manifest.id == id) || invalid.iter().any(|i| i.folder == id);
    let removed = running.iter().filter(|r| !present(&r.manifest.id)).map(|r| AddonSummary {
        loaded_version: Some(r.manifest.version.clone()),
        removed: true,
        ..summary(r, data)
    });
    valid.chain(invalid_rows).chain(removed).collect()
}

#[tauri::command]
pub async fn list_addons(app: AppHandle) -> Result<Vec<AddonSummary>, StorageError> {
    blocking(move || {
        let data = data_dir(&app)?;
        Ok(summaries_of(&load_dir(&data.join("addons"))?, &addons(&app)?.0, &data))
    })
    .await
}

/// `packs.list`: every pack, valid or not.
#[tauri::command]
pub async fn addon_packs(app: AppHandle, addon: String) -> Result<Vec<PackSummary>, StorageError> {
    blocking(move || {
        require(self::addon(&app, &addon)?, "packs:read")?;
        Ok(summaries(&*loaded(&app)?, &disabled(&app)?))
    })
    .await
}

/// `packs.install`: validated completely, then swapped in; the installed pack with that id is replaced.
#[tauri::command]
pub async fn addon_install_pack(app: AppHandle, addon: String, pack: NewPack) -> Result<PackSummary, StorageError> {
    blocking(move || {
        require(self::addon(&app, &addon)?, "packs:write")?;
        let id = {
            let _guard = lock_pack_change();
            let staging = data_dir(&app)?.join("packs-staging").join(&addon);
            let id = packs::install(&installed_dir(&app)?, &staging, &pack)?.manifest.id;
            reload();
            id
        };
        let summary = summaries(&*loaded(&app)?, &disabled(&app)?).into_iter().find(|p| p.id == id);
        summary.ok_or_else(|| StorageError::Io(format!("The pack \"{id}\" was installed but didn't load.")))
    })
    .await
}

/// `packs.remove`: an installed pack only; a bundled one with the same id comes back.
#[tauri::command]
pub async fn addon_remove_pack(app: AppHandle, addon: String, pack: String) -> Result<(), StorageError> {
    blocking(move || {
        require(self::addon(&app, &addon)?, "packs:write")?;
        let _guard = lock_pack_change();
        packs::remove(&installed_dir(&app)?, &pack)?;
        reload();
        Ok(())
    })
    .await
}

/// `net.fetch` may send at most this many request headers, each at most this long.
const MAX_HEADERS: usize = 20;
const MAX_HEADER_LEN: usize = 8 * 1024;

/// The address without its query, for messages: a query can hold a key.
fn shown_url(url: &str) -> &str {
    url.split(['?', '#']).next().unwrap_or(url)
}

/// Checks that the addon may reach `url` (`net:<host>`).
fn require_net(addon: &Addon, url: &str) -> Result<(), StorageError> {
    let host = https_host(url).ok_or_else(|| StorageError::Invalid(format!("\"{}\" isn't a plain https:// address.", shown_url(url))))?;
    require(addon, &format!("net:{host}"))
}

/// Request headers an addon may send: plain names, values without line breaks. The connection's own headers are the
/// app's.
fn check_headers(headers: &[(String, String)]) -> Result<(), StorageError> {
    const RESERVED: [&str; 6] = ["host", "content-length", "transfer-encoding", "connection", "cookie", "upgrade"];
    if headers.len() > MAX_HEADERS {
        return Err(StorageError::Invalid(format!("net.fetch sends at most {MAX_HEADERS} headers.")));
    }
    for (name, value) in headers {
        let plain_name = !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b));
        let plain_value = value.len() <= MAX_HEADER_LEN && value.bytes().all(|b| b == b'\t' || (b' '..=b'~').contains(&b));
        if !plain_name || !plain_value || RESERVED.contains(&name.to_ascii_lowercase().as_str()) {
            return Err(StorageError::Invalid(format!("net.fetch can't send the header \"{name}\".")));
        }
    }
    Ok(())
}

/// `net.fetch`: a GET (or a POST without a body) over https to a host the addon's manifest names (`net:<host>`), with
/// the headers it gives. The body comes back as bytes; an error status is an error that names it.
#[tauri::command]
pub async fn addon_fetch(app: AppHandle, addon: String, url: String, method: Option<String>, headers: Option<Vec<(String, String)>>) -> Result<ipc::Response, StorageError> {
    blocking(move || {
        require_net(self::addon(&app, &addon)?, &url)?;
        let headers = headers.unwrap_or_default();
        check_headers(&headers)?;
        static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
        let agent = AGENT.get_or_init(|| ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(120))).build().into());
        let failed = |e: ureq::Error| StorageError::Io(format!("{}: {e}", shown_url(&url)));
        let mut response = match method.as_deref().unwrap_or("GET") {
            "GET" => headers.iter().fold(agent.get(&url), |r, (k, v)| r.header(k, v)).call(),
            "POST" => headers.iter().fold(agent.post(&url), |r, (k, v)| r.header(k, v)).send_empty(),
            other => return Err(StorageError::Invalid(format!("net.fetch can't send a {other} request."))),
        }
        .map_err(failed)?;
        let body = response.body_mut().with_config().limit(MAX_BYTES).read_to_vec().map_err(failed)?;
        Ok(ipc::Response::new(body))
    })
    .await
}

// --- Search ----------------------------------------------------------------------------------------------------------

/// `search.setEntries`: at most this many entries per addon, together at most this much text.
const MAX_SEARCH_ENTRIES: usize = 50_000;
const MAX_SEARCH_BYTES: usize = 50 * 1024 * 1024;

/// Something an addon makes findable in global search (`search`): found by its name, then its tags, then its summary
/// and text; opening it opens the addon's Library tab at `state`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub state: Option<String>,
}

fn check_search_entries(entries: &[SearchEntry]) -> Result<(), StorageError> {
    if entries.len() > MAX_SEARCH_ENTRIES {
        return Err(StorageError::Invalid(format!("search.setEntries takes at most {MAX_SEARCH_ENTRIES} entries.")));
    }
    let size: usize = entries.iter().map(|e| e.id.len() + e.name.len() + e.tags.iter().map(String::len).sum::<usize>() + e.summary.len() + e.text.len() + e.state.as_ref().map_or(0, String::len)).sum();
    if size > MAX_SEARCH_BYTES {
        return Err(StorageError::Invalid(format!("search.setEntries takes at most {} MB of text.", MAX_SEARCH_BYTES / 1024 / 1024)));
    }
    for e in entries {
        // In characters, as documented (and as an addon's JavaScript counts them), not bytes.
        let chars = |s: &str| s.chars().count();
        let fits = (1..=200).contains(&chars(&e.id))
            && !e.name.trim().is_empty()
            && chars(&e.name) <= 500
            && e.tags.len() <= 50
            && e.tags.iter().all(|t| chars(t) <= 100)
            && chars(&e.summary) <= 2_000
            && chars(&e.text) <= 20_000
            && e.state.as_ref().is_none_or(|s| chars(s) <= 2_000);
        if !fits {
            return Err(StorageError::Invalid(format!("Search entry \"{}\" is too long or has no id or name.", e.id.chars().take(40).collect::<String>())));
        }
    }
    Ok(())
}

fn search_file(app: &AppHandle, addon: &str) -> Result<PathBuf, StorageError> {
    Ok(data_dir(app)?.join("addon-data").join(addon).join("search.json"))
}

fn read_search_entries(file: &Path) -> Vec<SearchEntry> {
    // shortcut: an unreadable file just leaves the addon out of search until it sends its entries again.
    fs::read_to_string(file).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

/// Every loaded addon with `search`, with its name and its entries: for the search index.
pub fn search_entries(app: &AppHandle) -> Result<Vec<(String, String, Vec<SearchEntry>)>, StorageError> {
    let mut all = Vec::new();
    // Not one uninstalled since start: its entries go at once, like its pages.
    for a in addons(app)?.0.iter().filter(|a| a.allows("search") && a.folder.is_dir()) {
        all.push((a.manifest.id.clone(), a.manifest.name.clone(), read_search_entries(&search_file(app, &a.manifest.id)?)));
    }
    Ok(all)
}

/// `search.setEntries`: replaces what the addon makes findable; the next search uses it.
#[tauri::command]
pub async fn addon_set_search(app: AppHandle, addon: String, entries: Vec<SearchEntry>) -> Result<(), StorageError> {
    blocking(move || {
        require(self::addon(&app, &addon)?, "search")?;
        check_search_entries(&entries)?;
        let file = search_file(&app, &addon)?;
        let json = serde_json::to_string(&entries).expect("entries serialize");
        // The same list again (a page that loaded) changes nothing: no rebuild of the index before the next search.
        if fs::read_to_string(&file).is_ok_and(|saved| saved == json) {
            return Ok(());
        }
        fs::create_dir_all(file.parent().expect("has a folder"))?;
        write_atomic(&file, &json)?;
        search::changed();
        Ok(())
    })
    .await
}

// --- Secrets ---------------------------------------------------------------------------------------------------------

/// The longest secret an addon can keep, in bytes: within what Windows' Credential Manager stores (2,560).
const MAX_SECRET: usize = 2048;

/// The addon's secret `name` in the OS keychain, apart from every other addon's (`secrets`).
fn secret(app: &AppHandle, addon: &str, name: &str) -> Result<keyring::Entry, StorageError> {
    require(self::addon(app, addon)?, "secrets")?;
    let plain = (1..=64).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b));
    if !plain {
        return Err(StorageError::Invalid("A secret's name is 1–64 letters, digits, '-', '_' or '.'.".into()));
    }
    keychain_entry(app, addon, name)
}

fn keychain_entry(app: &AppHandle, addon: &str, name: &str) -> Result<keyring::Entry, StorageError> {
    keyring::Entry::new(&app.config().identifier, &format!("addon:{addon}:{name}")).map_err(|e| keychain_error(&e))
}

/// The names of an addon's secrets, so deleting its data can delete them too: the keychain can't list them.
// shortcut: secrets set before names were recorded (before v0.7) aren't listed, so they stay in the keychain; record
// them some other way if that matters.
fn secret_names_file(data: &Path, addon: &str) -> PathBuf {
    data.join("addon-data").join(addon).join("secrets.json")
}

fn note_secret(data: &Path, addon: &str, name: &str, kept: bool) -> Result<(), StorageError> {
    edit_list(&secret_names_file(data, addon), name, kept)
}

fn keychain_error(e: &keyring::Error) -> StorageError {
    StorageError::Io(format!("The system keychain refused: {e}"))
}

/// `secrets.get`: the secret, or null when there's none.
#[tauri::command]
pub async fn addon_secret_get(app: AppHandle, addon: String, name: String) -> Result<Option<String>, StorageError> {
    blocking(move || match secret(&app, &addon, &name)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(keychain_error(&e)),
    })
    .await
}

/// `secrets.set`
#[tauri::command]
pub async fn addon_secret_set(app: AppHandle, addon: String, name: String, value: String) -> Result<(), StorageError> {
    blocking(move || {
        if value.is_empty() || value.len() > MAX_SECRET {
            return Err(StorageError::Invalid(format!("A secret is 1–{MAX_SECRET} bytes.")));
        }
        secret(&app, &addon, &name)?.set_password(&value).map_err(|e| keychain_error(&e))?;
        note_secret(&data_dir(&app)?, &addon, &name, true)
    })
    .await
}

/// `secrets.delete`: also when there's none.
#[tauri::command]
pub async fn addon_secret_delete(app: AppHandle, addon: String, name: String) -> Result<(), StorageError> {
    blocking(move || {
        match secret(&app, &addon, &name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(keychain_error(&e)),
        }
        note_secret(&data_dir(&app)?, &addon, &name, false)
    })
    .await
}

fn dialog(app: &AppHandle) -> tauri_plugin_dialog::FileDialogBuilder<tauri::Wry> {
    let builder = app.dialog().file();
    match app.get_webview_window("main") {
        Some(main) => builder.set_parent(&main),
        None => builder,
    }
}

fn picked_path(path: Option<tauri_plugin_dialog::FilePath>) -> Result<Option<PathBuf>, StorageError> {
    path.map(|p| p.into_path().map_err(|e| StorageError::Io(e.to_string()))).transpose()
}

/// `files.pickFile`: the user picks a file (of `extensions`, when given); its bytes come back, never its path.
// shortcut: cancelling hands back no bytes, the same as an empty file; neither is anything an addon can use.
#[tauri::command]
pub async fn addon_pick_file(app: AppHandle, addon: String, extensions: Vec<String>) -> Result<ipc::Response, StorageError> {
    blocking(move || {
        self::addon(&app, &addon)?;
        let mut builder = dialog(&app);
        if !extensions.is_empty() {
            builder = builder.add_filter(extensions.join(", "), &extensions.iter().map(String::as_str).collect::<Vec<_>>());
        }
        let Some(path) = picked_path(builder.blocking_pick_file())? else { return Ok(ipc::Response::new(Vec::new())) };
        if fs::metadata(&path)?.len() > MAX_BYTES {
            return Err(StorageError::Invalid(format!("The file is larger than {} MB.", MAX_BYTES / 1024 / 1024)));
        }
        Ok(ipc::Response::new(fs::read(path)?))
    })
    .await
}

/// A file of a picked folder: its path inside the folder (with `/`), and its text when it's UTF-8 text.
#[derive(Debug, Serialize, PartialEq)]
pub struct PickedFile {
    pub path: String,
    pub text: Option<String>,
}

/// `files.pickFolder`: the user picks a folder; every file in it (and its subfolders) comes back, never its path.
#[tauri::command]
pub async fn addon_pick_folder(app: AppHandle, addon: String) -> Result<Option<Vec<PickedFile>>, StorageError> {
    blocking(move || {
        self::addon(&app, &addon)?;
        let Some(root) = picked_path(dialog(&app).blocking_pick_folder())? else { return Ok(None) };
        read_folder(&root).map(Some)
    })
    .await
}

fn read_folder(root: &Path) -> Result<Vec<PickedFile>, StorageError> {
    let (mut files, mut total) = (Vec::new(), 0u64);
    walk(root, |path, entry| {
        total += entry.metadata()?.len();
        if files.len() == MAX_FILES || total > MAX_BYTES {
            return Err(StorageError::Invalid(format!("The folder has more than {MAX_FILES} files or {} MB.", MAX_BYTES / 1024 / 1024)));
        }
        files.push(PickedFile { path, text: String::from_utf8(fs::read(entry.path())?).ok() });
        Ok(())
    })?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Calls `visit` with every file under `root`, with its path inside `root` (with `/`). Links are skipped: they could
/// lead out of the folder the user picked.
fn walk(root: &Path, mut visit: impl FnMut(String, &fs::DirEntry) -> Result<(), StorageError>) -> Result<(), StorageError> {
    let mut todo = vec![root.to_path_buf()];
    while let Some(dir) = todo.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                todo.push(entry.path());
            } else if kind.is_file() {
                let relative = entry.path().strip_prefix(root).expect("found under root").to_owned();
                visit(relative.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"), &entry)?;
            }
        }
    }
    Ok(())
}

/// `characters.list`: the campaign's characters linked to this addon.
#[tauri::command]
pub async fn addon_characters(app: AppHandle, addon: String, campaign: String) -> Result<Vec<Character>, StorageError> {
    blocking(move || {
        require(self::addon(&app, &addon)?, "characters:write")?;
        let list = characters::list(&active_dir(&dirs(&app)?, &campaign)?)?;
        Ok(list.characters.into_iter().filter(|c| c.linked.as_deref() == Some(addon.as_str())).collect())
    })
    .await
}

/// `characters.save`: creates a character linked to this addon (empty id) or updates one linked to it.
#[tauri::command]
pub async fn addon_save_character(app: AppHandle, addon: String, campaign: String, character: Character) -> Result<Character, StorageError> {
    blocking(move || {
        require(self::addon(&app, &addon)?, "characters:write")?;
        let found = self::addon(&app, &addon)?;
        let saved = characters::save(&active_dir(&dirs(&app)?, &campaign)?, character, now_ms(), Editor::Addon(&addon, owns(found)))?;
        search::changed();
        Ok(saved)
    })
    .await
}

/// `characters.setPicture`: the picture of a character linked to this addon, from an image file's bytes.
#[tauri::command]
pub async fn addon_set_character_picture(app: AppHandle, addon: String, campaign: String, id: String, bytes: Vec<u8>) -> Result<Character, StorageError> {
    blocking(move || {
        require(self::addon(&app, &addon)?, "characters:write")?;
        characters::set_picture(&active_dir(&dirs(&app)?, &campaign)?, &id, Some(&bytes), Editor::Addon(&addon, &[]))
    })
    .await
}

fn storage_file(app: &AppHandle, addon: &str) -> Result<PathBuf, StorageError> {
    Ok(data_dir(app)?.join("addon-data").join(addon).join("storage.json"))
}

/// `storage.get`: what the addon saved last, or null.
#[tauri::command]
pub async fn addon_storage_get(app: AppHandle, addon: String) -> Result<Value, StorageError> {
    blocking(move || {
        self::addon(&app, &addon)?;
        match fs::read_to_string(storage_file(&app, &addon)?) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| StorageError::Corrupt(e.to_string())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Value::Null),
            Err(e) => Err(e.into()),
        }
    })
    .await
}

/// `storage.set`: replaces what the addon saved, atomically.
#[tauri::command]
pub async fn addon_storage_set(app: AppHandle, addon: String, value: Value) -> Result<(), StorageError> {
    blocking(move || {
        self::addon(&app, &addon)?;
        let text = serde_json::to_string(&value).expect("a JSON value serializes");
        if text.len() > MAX_STORAGE {
            return Err(StorageError::Invalid(format!("An addon can save at most {} MB.", MAX_STORAGE / 1024 / 1024)));
        }
        let file = storage_file(&app, &addon)?;
        fs::create_dir_all(file.parent().expect("has a folder"))?;
        write_atomic(&file, &text)
    })
    .await
}

/// `storage.putFile`: the largest file an addon can keep.
const MAX_STORED_FILE: usize = 50 * 1024 * 1024;

/// A name `storage.*File` accepts: 1–200 ASCII letters, digits, `.`, `_`, `-`, not starting with `.`. Also refused,
/// so a name always means one plain file in the addon's folder: a trailing `.` (Windows drops it), `.tmp` (the
/// temp file of an atomic write) and Windows device names (`CON`, `nul.txt`…).
fn stored_name_ok(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && stem.as_bytes()[3].is_ascii_digit());
    (1..=200).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        && !name.starts_with('.')
        && !name.ends_with('.')
        && !name.to_ascii_lowercase().ends_with(".tmp")
        && !device
}

fn files_dir(app: &AppHandle, addon: &str) -> Result<PathBuf, StorageError> {
    Ok(data_dir(app)?.join("addon-data").join(addon).join("files"))
}

/// The addon's stored file called `name`, once the addon and the name are checked.
fn stored_file(app: &AppHandle, addon: &str, name: &str) -> Result<PathBuf, StorageError> {
    self::addon(app, addon)?;
    if !stored_name_ok(name) {
        return Err(StorageError::Invalid(format!("\"{name}\" isn't a valid file name: 1–200 letters, digits, '.', '_' or '-', not starting with '.'.")));
    }
    Ok(files_dir(app, addon)?.join(name))
}

/// `storage.putFile`: keeps the bytes as the addon's file `name`, atomically. The bytes come as the raw request body
/// (the addon and name in headers), not as JSON: covers and cached pages are saved while reading.
#[tauri::command]
pub async fn addon_put_file(app: AppHandle, request: ipc::Request<'_>) -> Result<(), StorageError> {
    let header = |key: &str| request.headers().get(key).and_then(|v| v.to_str().ok()).map(str::to_string);
    let (Some(addon), Some(name)) = (header("addon"), header("name")) else {
        return Err(StorageError::Invalid("storage.putFile needs the addon and file name.".into()));
    };
    let ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(StorageError::Invalid("storage.putFile needs the file as raw bytes.".into()));
    };
    let bytes = bytes.clone();
    blocking(move || {
        let file = stored_file(&app, &addon, &name)?;
        if bytes.len() > MAX_STORED_FILE {
            return Err(StorageError::Invalid(format!("A stored file can be at most {} MB.", MAX_STORED_FILE / 1024 / 1024)));
        }
        write_atomic_bytes(&file, &bytes)
    })
    .await
}

/// `storage.getFile`: the file's bytes after a leading 1, or nothing at all when there's no such file (so an empty
/// file and a missing one differ; the bridge strips the 1).
#[tauri::command]
pub async fn addon_get_file(app: AppHandle, addon: String, name: String) -> Result<ipc::Response, StorageError> {
    blocking(move || match fs::read(stored_file(&app, &addon, &name)?) {
        Ok(bytes) => Ok(ipc::Response::new([&[1u8][..], &bytes].concat())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(ipc::Response::new(Vec::new())),
        Err(e) => Err(e.into()),
    })
    .await
}

/// `storage.deleteFile`: a missing file is fine.
#[tauri::command]
pub async fn addon_delete_file(app: AppHandle, addon: String, name: String) -> Result<(), StorageError> {
    blocking(move || match fs::remove_file(stored_file(&app, &addon, &name)?) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    })
    .await
}

/// A file `storage.listFiles` found: its name, size, and last change in ms.
#[derive(Debug, Serialize, PartialEq)]
pub struct StoredFile {
    name: String,
    size: u64,
    modified: u64,
}

/// `storage.listFiles`: the addon's stored files, sorted by name.
#[tauri::command]
pub async fn addon_list_files(app: AppHandle, addon: String) -> Result<Vec<StoredFile>, StorageError> {
    blocking(move || {
        self::addon(&app, &addon)?;
        list_stored(&files_dir(&app, &addon)?)
    })
    .await
}

/// The valid-named files in `dir` (a temp file of an interrupted write isn't one); none when it doesn't exist.
fn list_stored(dir: &Path) -> Result<Vec<StoredFile>, StorageError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !entry.file_type()?.is_file() || !stored_name_ok(&name) {
            continue;
        }
        let meta = entry.metadata()?;
        files.push(StoredFile { name, size: meta.len(), modified: modified_ms(&meta) });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

fn modified_ms(meta: &fs::Metadata) -> u64 {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as u64)
}

/// Whether the addon's `library` tab is shown. Missing or unreadable means shown: that's the default, and
/// `library.setVisible` writes the file again.
fn library_visible(data: &Path, addon: &str) -> bool {
    #[derive(Deserialize)]
    struct Saved {
        visible: bool,
    }
    let file = data.join("addon-data").join(addon).join("library.json");
    fs::read_to_string(file).ok().and_then(|t| serde_json::from_str::<Saved>(&t).ok()).is_none_or(|s| s.visible)
}

/// `library.setVisible`: shows or hides the addon's `library` tab, kept across restarts.
#[tauri::command]
pub async fn addon_set_library_visible(app: AppHandle, addon: String, visible: bool) -> Result<(), StorageError> {
    blocking(move || {
        let found = self::addon(&app, &addon)?;
        if found.manifest.library.is_none() {
            return Err(StorageError::Invalid(format!("{} has no Library tab.", found.manifest.name)));
        }
        let file = data_dir(&app)?.join("addon-data").join(&addon).join("library.json");
        write_atomic(&file, &serde_json::json!({ "visible": visible }).to_string())
    })
    .await
}

/// The campaigns an addon showed or hid its `campaign` screen in. Missing or unreadable means none: then every
/// campaign follows the manifest's `visible`, and `campaign.setVisible` writes the file again.
fn campaign_visibility(data: &Path, addon: &str) -> BTreeMap<String, bool> {
    let file = data.join("addon-data").join(addon).join("campaign.json");
    fs::read_to_string(file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// `campaign.setVisible`: shows or hides the addon's `campaign` screen in one campaign, kept across restarts.
#[tauri::command]
pub async fn addon_set_campaign_visible(app: AppHandle, addon: String, campaign: String, visible: bool) -> Result<(), StorageError> {
    static WRITING: Mutex<()> = Mutex::new(());
    blocking(move || {
        let found = self::addon(&app, &addon)?;
        if found.manifest.campaign.is_none() {
            return Err(StorageError::Invalid(format!("{} has no campaign screen.", found.manifest.name)));
        }
        crate::storage::check_id(&campaign, "campaign")?;
        let data = data_dir(&app)?;
        let _one_at_a_time = WRITING.lock().unwrap_or_else(|e| e.into_inner());
        let mut shown = campaign_visibility(&data, &addon);
        shown.insert(campaign, visible);
        write_atomic(&data.join("addon-data").join(&addon).join("campaign.json"), &serde_json::to_string(&shown).expect("a map serializes"))
    })
    .await
}

/// `folders.read`: the most bytes one call reads.
const MAX_READ: u64 = 16 * 1024 * 1024;
/// `folders.scan`: at most this many files per folder.
const MAX_SCAN: usize = 50_000;

/// A folder the user gave an addon (`folders:read`). Its path stays in the app; the addon only gets `id` and `name`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Granted {
    id: String,
    name: String,
    path: PathBuf,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct FolderSummary {
    id: String,
    name: String,
}

/// A file `folders.scan` found: its path inside the folder (with `/`), its size, and its last change in ms.
#[derive(Debug, Serialize, PartialEq)]
pub struct ScannedFile {
    path: String,
    size: u64,
    modified: u64,
}

/// The addon's folders file and the folders in it, once the addon is checked for `folders:read`. Kept apart from
/// `storage.*`, which the addon writes itself.
fn granted(app: &AppHandle, addon: &str) -> Result<(PathBuf, Vec<Granted>), StorageError> {
    require(self::addon(app, addon)?, "folders:read")?;
    let file = data_dir(app)?.join("addon-data").join(addon).join("folders.json");
    let list = match fs::read_to_string(&file) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| StorageError::Corrupt(e.to_string()))?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    Ok((file, list))
}

fn save_granted(file: &Path, list: &[Granted]) -> Result<(), StorageError> {
    fs::create_dir_all(file.parent().expect("has a folder"))?;
    write_atomic(file, &serde_json::to_string_pretty(list).expect("folders serialize"))
}

fn folder_summary(g: &Granted) -> FolderSummary {
    FolderSummary { id: g.id.clone(), name: g.name.clone() }
}

fn find_folder<'a>(list: &'a [Granted], id: &str) -> Result<&'a Granted, StorageError> {
    list.iter().find(|g| g.id == id).ok_or_else(|| StorageError::NotFound(format!("No folder \"{id}\".")))
}

/// Every file under `root`, sorted by path.
fn scan(root: &Path) -> Result<Vec<ScannedFile>, StorageError> {
    let mut files = Vec::new();
    walk(root, |path, entry| {
        if files.len() == MAX_SCAN {
            return Err(StorageError::Invalid(format!("The folder has more than {MAX_SCAN} files.")));
        }
        let meta = entry.metadata()?;
        files.push(ScannedFile { path, size: meta.len(), modified: modified_ms(&meta) });
        Ok(())
    })?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// `folders.list`
#[tauri::command]
pub async fn addon_folders(app: AppHandle, addon: String) -> Result<Vec<FolderSummary>, StorageError> {
    blocking(move || Ok(granted(&app, &addon)?.1.iter().map(folder_summary).collect())).await
}

/// `folders.add`: the user picks a folder, which the addon may read from then on. Null when cancelled; a folder
/// given before comes back as it was.
#[tauri::command]
pub async fn addon_add_folder(app: AppHandle, addon: String) -> Result<Option<FolderSummary>, StorageError> {
    blocking(move || {
        let (file, mut list) = granted(&app, &addon)?;
        let Some(path) = picked_path(dialog(&app).blocking_pick_folder())? else { return Ok(None) };
        if let Some(known) = list.iter().find(|g| g.path == path) {
            return Ok(Some(folder_summary(known)));
        }
        let name = path.file_name().map_or_else(|| path.to_string_lossy().into_owned(), |n| n.to_string_lossy().into_owned());
        let added = Granted { id: now_ms().to_string(), name, path };
        let result = folder_summary(&added);
        list.push(added);
        save_granted(&file, &list)?;
        Ok(Some(result))
    })
    .await
}

/// `folders.remove`: the addon can't read the folder any more. Its files are never touched.
#[tauri::command]
pub async fn addon_remove_folder(app: AppHandle, addon: String, folder: String) -> Result<(), StorageError> {
    blocking(move || {
        let (file, mut list) = granted(&app, &addon)?;
        list.retain(|g| g.id != folder);
        save_granted(&file, &list)
    })
    .await
}

/// `folders.scan`
#[tauri::command]
pub async fn addon_scan_folder(app: AppHandle, addon: String, folder: String) -> Result<Vec<ScannedFile>, StorageError> {
    blocking(move || scan(&find_folder(&granted(&app, &addon)?.1, &folder)?.path)).await
}

/// `folders.read`: at most `length` bytes of a file in the folder, from `offset` (fewer at its end).
#[tauri::command]
pub async fn addon_read_file(app: AppHandle, addon: String, folder: String, path: String, offset: u64, length: u64) -> Result<ipc::Response, StorageError> {
    blocking(move || {
        if length > MAX_READ {
            return Err(StorageError::Invalid(format!("At most {} MB can be read at once.", MAX_READ / 1024 / 1024)));
        }
        let root = find_folder(&granted(&app, &addon)?.1, &folder)?.path.clone();
        let full = file_inside(&root, &path).ok_or_else(|| StorageError::NotFound(format!("\"{path}\" isn't a file in the folder.")))?;
        let mut file = fs::File::open(full)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = Vec::new();
        file.take(length).read_to_end(&mut bytes)?;
        Ok(ipc::Response::new(bytes))
    })
    .await
}

/// The MD5 checksum of a file, as lowercase hex.
fn md5_of(path: &Path) -> io::Result<String> {
    use md5::{Digest, Md5};
    let mut file = fs::File::open(path)?;
    let mut hasher = Md5::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            return Ok(format!("{:x}", hasher.finalize()));
        }
        hasher.update(&buf[..n]);
    }
}

/// `folders.hash`: the MD5 checksum of a file in the folder.
#[tauri::command]
pub async fn addon_hash_file(app: AppHandle, addon: String, folder: String, path: String) -> Result<String, StorageError> {
    blocking(move || {
        let root = find_folder(&granted(&app, &addon)?.1, &folder)?.path.clone();
        let full = file_inside(&root, &path).ok_or_else(|| StorageError::NotFound(format!("\"{path}\" isn't a file in the folder.")))?;
        Ok(md5_of(&full)?)
    })
    .await
}

/// `folders.download`: one file, at most this big.
const MAX_DOWNLOAD: u64 = 8 * 1024 * 1024 * 1024;
/// The end of a file still downloading, beside its place.
const PARTIAL: &str = ".download";
/// A partial file not written to for this long is left over (a crash), not another download still running.
const PARTIAL_STALE: Duration = Duration::from_secs(60);

/// The partial file, new. One already there that's still being written (another download of the same file, which goes
/// on in the app when the page that asked for it closes) refuses; one left over from a crash is started again.
fn open_partial(partial: &Path) -> Result<fs::File, StorageError> {
    match fs::OpenOptions::new().write(true).create_new(true).open(partial) {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            // Asked through the file, not its folder: Windows updates a file's time in its folder only once it's closed.
            let age = fs::File::open(partial)?.metadata()?.modified()?.elapsed().unwrap_or_default();
            if age < PARTIAL_STALE {
                return Err(StorageError::Invalid("This file is already being downloaded.".into()));
            }
            Ok(fs::File::create(partial)?)
        }
        opened => Ok(opened?),
    }
}

/// Where `path` goes inside `root`, its folders made: never outside it (no `..`, no link leading out), never onto a
/// folder.
fn download_target(root: &Path, path: &str) -> Result<PathBuf, StorageError> {
    let outside = || StorageError::Invalid(format!("\"{path}\" isn't a place in the folder."));
    if !safe_path(path) || path.ends_with(PARTIAL) {
        return Err(outside());
    }
    let root = root.canonicalize()?;
    let target = root.join(path);
    let (Some(parent), Some(name)) = (target.parent(), target.file_name()) else { return Err(outside()) };
    // The deepest folder already there must be inside too, so a link can't make folders elsewhere.
    let existing = parent.ancestors().find(|dir| dir.exists()).ok_or_else(outside)?.canonicalize()?;
    if !existing.starts_with(&root) {
        return Err(outside());
    }
    fs::create_dir_all(parent)?;
    let parent = parent.canonicalize()?;
    if !parent.starts_with(&root) || parent.join(name).is_dir() {
        return Err(outside());
    }
    Ok(parent.join(name))
}

/// Streams `response` into `target` through a file beside it, which only replaces `target` once it is complete. Returns
/// the file's MD5.
fn save_download(mut response: ureq::http::Response<ureq::Body>, target: &Path) -> Result<String, StorageError> {
    use md5::{Digest, Md5};
    let mut partial = target.as_os_str().to_owned();
    partial.push(PARTIAL);
    let partial = PathBuf::from(partial);
    // Before the cleanup below: a partial file another download is writing isn't this one's to remove.
    let mut out = open_partial(&partial)?;
    let result = (|| {
        let mut reader = response.body_mut().with_config().limit(MAX_DOWNLOAD).reader();
        let (mut hasher, mut buf) = (Md5::new(), vec![0u8; 1 << 16]);
        loop {
            let n = reader.read(&mut buf).map_err(|e| StorageError::Io(format!("The download broke off: {e}")))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            io::Write::write_all(&mut out, &buf[..n])?;
        }
        out.sync_all()?;
        drop(out);
        fs::rename(&partial, target)?;
        Ok(format!("{:x}", hasher.finalize()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&partial); // only a leftover partial file otherwise
    }
    result
}

/// `folders.download`: downloads an https file (from a `net:<host>` the addon names) to `path` in the folder
/// (`folders:write`), making its folders; a file already there is replaced only by a complete download.
#[tauri::command]
pub async fn addon_download_file(app: AppHandle, addon: String, folder: String, path: String, url: String) -> Result<String, StorageError> {
    blocking(move || {
        let found = self::addon(&app, &addon)?;
        require(found, "folders:write")?;
        require_net(found, &url)?;
        let root = find_folder(&granted(&app, &addon)?.1, &folder)?.path.clone();
        let target = download_target(&root, &path)?;
        static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
        // A big book may take a while, but one file 30 minutes at most.
        let agent = AGENT.get_or_init(|| {
            ureq::Agent::config_builder()
                .timeout_connect(Some(Duration::from_secs(20)))
                .timeout_recv_response(Some(Duration::from_secs(60)))
                .timeout_recv_body(Some(Duration::from_secs(30 * 60)))
                .build()
                .into()
        });
        let response = agent.get(&url).call().map_err(|e| StorageError::Io(format!("{}: {e}", shown_url(&url))))?;
        save_download(response, &target)
    })
    .await
}

/// `folders.reveal`: shows the file in the system file manager, selected.
#[tauri::command]
pub async fn addon_reveal_file(app: AppHandle, addon: String, folder: String, path: String) -> Result<(), StorageError> {
    blocking(move || {
        let root = find_folder(&granted(&app, &addon)?.1, &folder)?.path.clone();
        reveal(&revealable(&root, &path)?)
    })
    .await
}

/// The file to reveal: inside the folder only, as a path the file manager reads (without Windows' `\\?\` prefix,
/// which `canonicalize` adds and Explorer doesn't understand).
fn revealable(root: &Path, path: &str) -> Result<PathBuf, StorageError> {
    let full = file_inside(root, path).ok_or_else(|| StorageError::NotFound(format!("\"{path}\" isn't a file in the folder.")))?;
    let text = full.to_string_lossy();
    Ok(match text.strip_prefix(r"\\?\UNC\") {
        Some(share) => PathBuf::from(format!(r"\\{share}")),
        None => PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text)),
    })
}

#[cfg(target_os = "windows")]
fn reveal(file: &Path) -> Result<(), StorageError> {
    use std::os::windows::process::CommandExt;
    // Raw, because Explorer wants `/select,"<path>"` as is; a Windows path can't hold a `"`. Its exit code means
    // nothing (1 on success), so only failing to start counts.
    std::process::Command::new("explorer").raw_arg(format!("/select,\"{}\"", file.display())).status()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn reveal(file: &Path) -> Result<(), StorageError> {
    std::process::Command::new("open").arg("-R").arg(file).status()?;
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn reveal(_file: &Path) -> Result<(), StorageError> {
    Err(StorageError::Invalid("Showing a file in the file manager isn't supported on this system.".into()))
}

/// `files.saveFile`: the user picks where the bytes go (`name` suggested). False when cancelled.
#[tauri::command]
pub async fn addon_save_file(app: AppHandle, addon: String, name: String, bytes: Vec<u8>) -> Result<bool, StorageError> {
    blocking(move || {
        self::addon(&app, &addon)?;
        let Some(path) = picked_path(dialog(&app).set_file_name(&name).blocking_save_file())? else { return Ok(false) };
        write_atomic_bytes(&path, &bytes)?;
        Ok(true)
    })
    .await
}

// --- Campaigns and links ---------------------------------------------------------------------------------------------

/// One active campaign as an addon sees it.
#[derive(Debug, Serialize)]
pub struct CampaignSummary {
    pub id: String,
    pub name: String,
}

/// `campaigns.list`: the active campaigns, most recently opened first.
#[tauri::command]
pub async fn addon_campaigns(app: AppHandle, addon: String) -> Result<Vec<CampaignSummary>, StorageError> {
    blocking(move || {
        require(self::addon(&app, &addon)?, "campaigns:read")?;
        let list = crate::commands::campaigns::list(&dirs(&app)?)?;
        Ok(list.campaigns.into_iter().map(|c| CampaignSummary { id: c.id, name: c.name }).collect())
    })
    .await
}

/// The longest address `ui.openExternal` opens.
const MAX_LINK: usize = 2000;

/// An address `ui.openExternal` may hand to the system: plain `https://`, at most `MAX_LINK` long, and only URL
/// characters (no spaces, quotes or shell characters), so the system's opener reads it as one address. Its host.
fn link_host(url: &str) -> Option<String> {
    const OK: &[u8] = b"-._~:/?#[]@!$&'()*+,;=%";
    let plain = url.len() <= MAX_LINK && url.bytes().all(|b| b.is_ascii_alphanumeric() || OK.contains(&b));
    if plain { https_host(url) } else { None }
}

/// `ui.openExternal`: opens an https address in the system's browser, to a host the manifest names (`links:<host>`).
#[tauri::command]
pub async fn addon_open_external(app: AppHandle, addon: String, url: String) -> Result<(), StorageError> {
    blocking(move || {
        let host = link_host(&url).ok_or_else(|| StorageError::Invalid(format!("\"{}\" isn't a plain https:// address.", shown_url(&url))))?;
        require(self::addon(&app, &addon)?, &format!("links:{host}"))?;
        open_in_browser(&url)
    })
    .await
}

#[cfg(target_os = "windows")]
fn open_in_browser(url: &str) -> Result<(), StorageError> {
    // The URL handler, not `cmd /c start`, so nothing in the address is read by a shell.
    std::process::Command::new("rundll32").arg("url.dll,FileProtocolHandler").arg(url).status()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_in_browser(url: &str) -> Result<(), StorageError> {
    std::process::Command::new("open").arg(url).status()?;
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn open_in_browser(_url: &str) -> Result<(), StorageError> {
    Err(StorageError::Invalid("Opening a link isn't supported on this system.".into()))
}

/// The addon pages' own policy: scripts, styles and images from the addon scheme only; no network (`net.fetch` is the
/// way out), no frames, no forms.
const ADDON_CSP: &str = "default-src 'none'; \
    script-src koldron-addon: http://koldron-addon.localhost 'wasm-unsafe-eval'; \
    style-src koldron-addon: http://koldron-addon.localhost 'unsafe-inline'; \
    img-src koldron-addon: http://koldron-addon.localhost data: blob:; \
    font-src koldron-addon: http://koldron-addon.localhost data:; \
    connect-src koldron-addon: http://koldron-addon.localhost; \
    worker-src koldron-addon: http://koldron-addon.localhost blob:; \
    frame-src 'none'; form-action 'none'; base-uri 'none'";

/// Answers `koldron-addon://localhost/<addon id>/<file>` with that file from the addon's folder.
pub fn serve(app: &AppHandle, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let answer = || -> Result<(Vec<u8>, &'static str), StatusCode> {
        if request.method() != "GET" {
            return Err(StatusCode::METHOD_NOT_ALLOWED);
        }
        let (id, file) = request.uri().path().trim_start_matches('/').split_once('/').ok_or(StatusCode::NOT_FOUND)?;
        let found = addon(app, id).map_err(|_| StatusCode::NOT_FOUND)?;
        read_inside(&found.folder, file).map(|bytes| (bytes, content_type(file)))
    };
    let response = Response::builder()
        .header(header::CONTENT_SECURITY_POLICY, ADDON_CSP)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        // The frames have an opaque origin (no allow-same-origin), so module scripts and workers need this.
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
    match answer() {
        Ok((bytes, kind)) => response.header(header::CONTENT_TYPE, kind).body(bytes),
        Err(status) => response.status(status).body(Vec::new()),
    }
    .expect("static headers are valid")
}

/// The file at `path` inside `folder`, never outside it (no `..`, no link leading out).
// shortcut: no percent-decoding, so addon file names are plain ASCII without spaces; decode here if one needs more.
fn read_inside(folder: &Path, path: &str) -> Result<Vec<u8>, StatusCode> {
    fs::read(file_inside(folder, path).ok_or(StatusCode::NOT_FOUND)?).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// The file at `path` inside `folder`, if it is one there (no `..`, no link leading out).
fn file_inside(folder: &Path, path: &str) -> Option<PathBuf> {
    if !safe_path(path) {
        return None;
    }
    let root = folder.canonicalize().ok()?;
    let file = root.join(path).canonicalize().ok()?;
    (file.starts_with(&root) && file.is_file()).then_some(file)
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).as_deref() {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("woff2") => "font/woff2",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

// ---- Installing an addon from a .zip (Settings → Addons): the main window's calls, not an addon's. ---------------

/// One install at a time: they share the staging folder.
static ADDON_INSTALL: Mutex<()> = Mutex::new(());

fn lock_install() -> std::sync::MutexGuard<'static, ()> {
    ADDON_INSTALL.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Where a picked .zip is unzipped until it's installed or discarded.
fn staging_dir(app: &AppHandle) -> Result<PathBuf, StorageError> {
    Ok(data_dir(app)?.join("addon-staging"))
}

/// Unzips into `dest`: only paths inside it, no links, at most `MAX_FILES` files and `limit` bytes unzipped.
fn unzip(zip: impl Read + Seek, dest: &Path, limit: u64) -> Result<(), StorageError> {
    let unreadable = |e: zip::result::ZipError| StorageError::Invalid(format!("Not a readable .zip file: {e}"));
    let mut archive = zip::ZipArchive::new(zip).map_err(unreadable)?;
    if archive.len() > MAX_FILES {
        return Err(StorageError::Invalid(format!("The .zip has more than {MAX_FILES} files.")));
    }
    let mut left = limit;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(unreadable)?;
        let path = entry.enclosed_name().filter(|_| !entry.is_symlink());
        let Some(path) = path else { return Err(StorageError::Invalid(format!("\"{}\" points outside the addon.", entry.name().unwrap_or_default()))) };
        if path.as_os_str().is_empty() {
            continue; // "." or "./": the folder itself
        }
        let target = dest.join(path);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        // Counted while unzipping: the sizes a .zip states can lie.
        let written = io::copy(&mut (&mut entry).take(left + 1), &mut fs::File::create(&target)?)?;
        if written > left {
            return Err(StorageError::Invalid(format!("The addon is larger than {} MB unzipped.", limit / 1024 / 1024)));
        }
        left -= written;
    }
    Ok(())
}

/// The unzipped addon's folder: the .zip's top level, or the one folder in it with a manifest (a zipped folder).
fn addon_root(dir: &Path) -> Result<PathBuf, StorageError> {
    if dir.join(MANIFEST).is_file() {
        return Ok(dir.to_path_buf());
    }
    let mut found = fs::read_dir(dir)?.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.join(MANIFEST).is_file());
    match (found.next(), found.next()) {
        (Some(root), None) => Ok(root),
        _ => Err(StorageError::Invalid(format!("The .zip has no {MANIFEST} at its top or in one folder."))),
    }
}

fn checked(root: &Path) -> Result<Manifest, StorageError> {
    read_new(root).map_err(|errors| StorageError::Invalid(format!("This addon can't be installed. {}", capped(&errors).join(" "))))
}

fn discard(staging: &Path) -> Result<(), StorageError> {
    match fs::remove_dir_all(staging) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

/// Unzips `zip` into `staging` (replacing an earlier unzipped addon) and checks the addon in it. Only its own
/// subfolder is cleared, so a leftover of an uninstall that's still locked can't block an install.
fn stage(zip: &Path, staging: &Path) -> Result<Manifest, StorageError> {
    let unzipped = staging.join("unzipped");
    discard(&unzipped)?;
    fs::create_dir_all(&unzipped)?;
    let result = unzip(fs::File::open(zip)?, &unzipped, MAX_BYTES).and_then(|()| checked(&addon_root(&unzipped)?));
    if result.is_err() {
        let _ = discard(&unzipped);
    }
    result
}

/// Windows refuses to move a folder another program has open (an editor, File Explorer): say that, not "os error 5".
fn move_addon_folder(from: &Path, to: &Path) -> Result<(), StorageError> {
    fs::rename(from, to).map_err(|e| match e.kind() {
        io::ErrorKind::PermissionDenied => StorageError::Io(
            "Another program has the addon's folder open, such as an editor or File Explorer. Close it there and try again.".into(),
        ),
        _ => e.into(),
    })
}

/// Moves the staged addon to `addons/<id>`, replacing the one with that id; on failure the old one is put back.
/// Its saved data (`addon-data/<id>/`) is never touched.
fn install(staging: &Path, addons: &Path) -> Result<(), StorageError> {
    let root = addon_root(&staging.join("unzipped"))?;
    let target = addons.join(&checked(&root)?.id);
    let old = staging.join("old");
    let _ = fs::remove_dir_all(&old);
    fs::create_dir_all(addons)?;
    let replacing = target.exists();
    if replacing {
        move_addon_folder(&target, &old)?;
    }
    if let Err(e) = fs::rename(&root, &target) {
        if replacing {
            let _ = fs::rename(&old, &target);
        }
        return Err(e.into());
    }
    let _ = discard(staging);
    Ok(())
}

/// What the user confirms before an install.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedAddon {
    pub id: String,
    pub name: String,
    pub version: String,
    pub permissions: Vec<String>,
    /// The version of the addon with this id that it replaces ("" when that one's manifest can't be read); `None`
    /// when there's none.
    pub replaces: Option<String>,
}

/// The user picks an addon's .zip; it's unzipped and checked, waiting for `install_staged_addon`. `None`: cancelled.
#[tauri::command]
pub async fn stage_addon(app: AppHandle) -> Result<Option<StagedAddon>, StorageError> {
    blocking(move || {
        let Some(zip) = picked_path(dialog(&app).add_filter("Addon", &["zip"]).blocking_pick_file())? else { return Ok(None) };
        let _lock = lock_install();
        let manifest = stage(&zip, &staging_dir(&app)?)?;
        let current = data_dir(&app)?.join("addons").join(&manifest.id);
        let replaces = current.is_dir().then(|| {
            let text = fs::read_to_string(current.join(MANIFEST)).unwrap_or_default();
            serde_json::from_str::<Value>(&text).ok().and_then(|v| v["version"].as_str().map(str::to_string)).unwrap_or_default()
        });
        Ok(Some(StagedAddon { id: manifest.id, name: manifest.name, version: manifest.version, permissions: manifest.permissions, replaces }))
    })
    .await
}

/// Installs the staged addon; it loads on the next start.
#[tauri::command]
pub async fn install_staged_addon(app: AppHandle) -> Result<(), StorageError> {
    blocking(move || {
        let _lock = lock_install();
        install(&staging_dir(&app)?, &data_dir(&app)?.join("addons"))
    })
    .await
}

#[tauri::command]
pub async fn discard_staged_addon(app: AppHandle) -> Result<(), StorageError> {
    blocking(move || {
        let _lock = lock_install();
        discard(&staging_dir(&app)?)
    })
    .await
}

fn installed_folder(data: &Path, id: &str) -> Result<PathBuf, StorageError> {
    crate::storage::check_id(id, "addon")?;
    let folder = data.join("addons").join(id);
    if folder.is_dir() { Ok(folder) } else { Err(StorageError::NotFound(format!("No addon \"{id}\" is installed."))) }
}

/// Enables or disables an installed addon; it takes effect on the next start.
#[tauri::command]
pub async fn set_addon_enabled(app: AppHandle, id: String, enabled: bool) -> Result<(), StorageError> {
    blocking(move || {
        let data = data_dir(&app)?;
        installed_folder(&data, &id)?;
        let _lock = lock_install();
        set_disabled(&data, &id, !enabled)
    })
    .await
}

/// Removes `addons/<id>`: moved aside in one step (the step that can fail on a locked file, so it goes first and
/// nothing else is touched if it does), then deleted. With `delete_data`, each of its secrets (`delete_secret`) and
/// `addon-data/<id>/` go too. Packs it installed stay.
fn uninstall(
    data: &Path,
    staging: &Path,
    id: &str,
    delete_data: bool,
    delete_secret: impl Fn(&str) -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    let folder = installed_folder(data, id)?;
    fs::create_dir_all(staging)?;
    let aside = staging.join(format!("removed-{id}"));
    let _ = fs::remove_dir_all(&aside);
    move_addon_folder(&folder, &aside)?;
    let _ = fs::remove_dir_all(&aside); // what's left goes with the staging folder
    set_disabled(data, id, false)?;
    if delete_data {
        for name in read_list(&secret_names_file(data, id))? {
            delete_secret(&name)?;
        }
        discard(&data.join("addon-data").join(id))?;
    }
    Ok(())
}

/// Uninstalls an addon; its pages and calls stop at once. `delete_data`: its saved data and its secrets go too.
#[tauri::command]
pub async fn uninstall_addon(app: AppHandle, id: String, delete_data: bool) -> Result<(), StorageError> {
    blocking(move || {
        let data = data_dir(&app)?;
        installed_folder(&data, &id)?;
        let _lock = lock_install();
        uninstall(&data, &staging_dir(&app)?, &id, delete_data, |name| match keychain_entry(&app, &id, name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(keychain_error(&e)),
        })?;
        search::changed(); // its entries leave global search
        Ok(())
    })
    .await
}

/// Addons are read once per run, so a change needs a restart.
#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("koldron-addon-runtime-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn campaign_visibility_reads_what_was_saved_and_nothing_from_a_damaged_file() {
        let data = temp_dir("campaign-visibility");
        assert!(campaign_visibility(&data, "lore").is_empty());
        let file = data.join("addon-data/lore/campaign.json");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, r#"{"c1":true,"c2":false}"#).unwrap();
        assert_eq!(campaign_visibility(&data, "lore"), BTreeMap::from([("c1".to_string(), true), ("c2".to_string(), false)]));
        fs::write(&file, "{ not json").unwrap();
        assert!(campaign_visibility(&data, "lore").is_empty());
    }

    #[test]
    fn links_are_plain_https_addresses_only() {
        assert_eq!(link_host("https://app.example.com/a/b?c=1&d=%20#e").as_deref(), Some("app.example.com"));
        let long = format!("https://a.com/{}", "x".repeat(MAX_LINK));
        for bad in ["http://a.com/", "https://a.com/x y", "https://a.com/\"&calc", "https://a.com/^x", "https://a.com/<x>", r"https://a.com/x\y", "https://user@a.com/", long.as_str()] {
            assert_eq!(link_host(bad), None, "{bad}");
        }
    }

    #[test]
    fn serves_files_inside_the_addon_folder_only() {
        let dir = temp_dir("serve");
        let folder = dir.join("addon");
        fs::create_dir_all(folder.join("ui")).unwrap();
        fs::write(folder.join("ui/page.html"), "<p>hi</p>").unwrap();
        fs::write(dir.join("secret.txt"), "no").unwrap();
        assert_eq!(read_inside(&folder, "ui/page.html").unwrap(), b"<p>hi</p>");
        for bad in ["../secret.txt", "ui/../../secret.txt", "", "ui", "missing.js", "/etc/hosts"] {
            assert_eq!(read_inside(&folder, bad), Err(StatusCode::NOT_FOUND), "{bad}");
        }
        assert_eq!(content_type("ui/page.HTML"), "text/html; charset=utf-8");
        assert_eq!(content_type("data"), "application/octet-stream");
    }

    #[test]
    fn a_folder_scans_every_file_and_reads_only_inside() {
        let dir = temp_dir("granted");
        let lib = dir.join("lib");
        fs::create_dir_all(lib.join("sub")).unwrap();
        for f in ["a.md", "sub/b.json", "notes.txt"] {
            fs::write(lib.join(f), "x").unwrap();
        }
        fs::write(dir.join("outside.txt"), "x").unwrap();
        let found: Vec<_> = scan(&lib).unwrap().into_iter().map(|f| f.path).collect();
        assert_eq!(found, ["a.md", "notes.txt", "sub/b.json"]);
        assert!(file_inside(&lib, "sub/b.json").is_some());
        for bad in ["../outside.txt", "sub", "", "/a.md", "missing.md"] {
            assert!(file_inside(&lib, bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn downloads_land_inside_the_folder_only_and_hash_their_bytes() {
        let dir = temp_dir("download");
        let lib = dir.join("lib");
        fs::create_dir_all(lib.join("taken")).unwrap();
        let target = download_target(&lib, "Pub/Book/book.pdf").unwrap();
        assert!(target.ends_with("book.pdf") && target.parent().unwrap().is_dir());
        // One download per partial file: a fresh one is another download still running, an old one is left over.
        let partial = lib.join("busy.pdf.download");
        drop(open_partial(&partial).unwrap());
        assert!(open_partial(&partial).is_err());
        fs::File::options().write(true).open(&partial).unwrap().set_modified(std::time::SystemTime::now() - PARTIAL_STALE * 2).unwrap();
        assert!(open_partial(&partial).is_ok());
        for bad in ["../out.pdf", "Pub/../../out.pdf", "", "/abs.pdf", "taken", "x.pdf.download"] {
            assert!(download_target(&lib, bad).is_err(), "{bad}");
        }
        fs::write(&target, "hello").unwrap();
        assert_eq!(md5_of(&target).unwrap(), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn search_entries_need_an_id_and_a_name_and_stay_within_limits() {
        let ok = SearchEntry { id: "a".into(), name: "Name".into(), tags: vec!["t".into()], summary: String::new(), text: String::new(), state: None };
        assert!(check_search_entries(std::slice::from_ref(&ok)).is_ok());
        for bad in [
            SearchEntry { id: String::new(), ..ok.clone() },
            SearchEntry { name: " ".into(), ..ok.clone() },
            SearchEntry { text: "x".repeat(20_001), ..ok.clone() },
            SearchEntry { state: Some("x".repeat(2_001)), ..ok.clone() },
        ] {
            assert!(check_search_entries(&[bad]).is_err());
        }
        // Counted in characters: 20,000 accented letters (40,000 bytes) fit.
        assert!(check_search_entries(&[SearchEntry { text: "é".repeat(20_000), ..ok.clone() }]).is_ok());
        assert!(check_search_entries(&vec![ok.clone(); MAX_SEARCH_ENTRIES + 1]).is_err());
        // Each within its limits, but too much together.
        let long = SearchEntry { text: "x".repeat(20_000), ..ok };
        assert!(check_search_entries(&vec![long; MAX_SEARCH_BYTES / 20_000 + 1]).is_err());
    }

    #[test]
    fn fetch_sends_plain_headers_only_and_hides_queries() {
        let ok = |name: &str, value: &str| check_headers(&[(name.to_string(), value.to_string())]).is_ok();
        assert!(ok("Authorization", "abc.def") && ok("X-Api_Key", "1 2"));
        for (name, value) in [("Host", "x"), ("cookie", "a=b"), ("Bad Name", "x"), ("", "x"), ("X", "a\r\nInjected: 1")] {
            assert!(!ok(name, value), "{name}");
        }
        let many: Vec<_> = (0..=MAX_HEADERS).map(|i| (format!("X-{i}"), "v".to_string())).collect();
        assert!(check_headers(&many).is_err());
        assert_eq!(shown_url("https://api.x.com/auth?key=secret#a"), "https://api.x.com/auth");
    }

    #[test]
    fn stored_file_names_are_plain_names_only() {
        let longest = "x".repeat(200);
        for good in ["a", "notes.json", "Cover_01-b.png", "a.b.c", "console", "com10", longest.as_str()] {
            assert!(stored_name_ok(good), "{good}");
        }
        let long = "x".repeat(201);
        for bad in ["", ".hidden", "..", "../x", "a/b", r"a\b", "c:x", "a b", "é", "a.", "x.tmp", "x.TMP", "CON", "nul.txt", "com1", "Lpt9.log", long.as_str()] {
            assert!(!stored_name_ok(bad), "{bad}");
        }
    }

    #[test]
    fn stored_files_list_sorted_without_temp_files_or_folders() {
        let dir = temp_dir("stored");
        fs::write(dir.join("b.bin"), "12").unwrap();
        fs::write(dir.join("a.json"), "1").unwrap();
        fs::write(dir.join("a.json.tmp"), "half").unwrap();
        fs::create_dir_all(dir.join("sub")).unwrap();
        let got: Vec<_> = list_stored(&dir).unwrap().into_iter().map(|f| (f.name, f.size)).collect();
        assert_eq!(got, [("a.json".to_string(), 1), ("b.bin".to_string(), 2)]);
        assert!(list_stored(&dir.join("missing")).unwrap().is_empty());
    }

    #[test]
    fn reveal_stays_inside_the_folder_and_drops_the_verbatim_prefix() {
        let dir = temp_dir("reveal");
        let lib = dir.join("lib");
        fs::create_dir_all(lib.join("sub")).unwrap();
        fs::write(lib.join("sub/a.txt"), "x").unwrap();
        fs::write(dir.join("outside.txt"), "x").unwrap();
        let shown = revealable(&lib, "sub/a.txt").unwrap();
        assert!(!shown.to_string_lossy().starts_with(r"\\?\"), "{}", shown.display());
        assert!(shown.ends_with("sub/a.txt") && shown.is_file(), "{}", shown.display());
        for bad in ["../outside.txt", "sub", "", "/sub/a.txt", "missing.txt"] {
            assert!(matches!(revealable(&lib, bad), Err(StorageError::NotFound(_))), "{bad}");
        }
    }

    #[test]
    fn a_library_tab_is_visible_unless_saved_hidden() {
        let data = temp_dir("visible");
        assert!(library_visible(&data, "tabbed"));
        fs::create_dir_all(data.join("addon-data/tabbed")).unwrap();
        fs::write(data.join("addon-data/tabbed/library.json"), r#"{ "visible": false }"#).unwrap();
        assert!(!library_visible(&data, "tabbed"));
        fs::write(data.join("addon-data/tabbed/library.json"), "not json").unwrap();
        assert!(library_visible(&data, "tabbed"));
    }

    #[test]
    fn a_picked_folder_comes_back_with_relative_paths_and_text_only_for_text() {
        let dir = temp_dir("pick");
        fs::create_dir_all(dir.join("data")).unwrap();
        fs::write(dir.join("pack.json"), "{}").unwrap();
        fs::write(dir.join("data/c.json"), "[]").unwrap();
        fs::write(dir.join("data/icon.png"), [0xff, 0xfe, 0x00]).unwrap();
        let got = read_folder(&dir).unwrap();
        let want = [("data/c.json", Some("[]")), ("data/icon.png", None), ("pack.json", Some("{}"))];
        assert_eq!(got.iter().map(|f| (f.path.as_str(), f.text.as_deref())).collect::<Vec<_>>(), want);
    }

    fn zip_of(files: &[(&str, &str)]) -> io::Cursor<Vec<u8>> {
        let mut writer = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
        for (name, text) in files {
            writer.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            io::Write::write_all(&mut writer, text.as_bytes()).unwrap();
        }
        writer.finish().unwrap()
    }

    fn manifest(version: &str) -> String {
        serde_json::json!({ "api": crate::addons::API, "id": "demo", "name": "Demo", "version": version, "license": "MIT",
            "permissions": ["secrets"], "library": { "label": "Demo", "entry": "ui/tab.html" } }).to_string()
    }

    #[test]
    fn unzips_only_inside_the_folder() {
        let dir = temp_dir("unzip-escape");
        let err = unzip(zip_of(&[("ok.txt", "x"), ("../evil.txt", "x")]), &dir.join("out"), MAX_BYTES).unwrap_err();
        assert!(matches!(err, StorageError::Invalid(_)), "{err:?}");
        assert!(!dir.join("evil.txt").exists());
    }

    #[test]
    fn unzips_no_more_than_the_limit() {
        let dir = temp_dir("unzip-limit");
        assert!(unzip(zip_of(&[("a.txt", "12345"), ("./", "")]), &dir.join("fits"), 5).is_ok());
        let err = unzip(zip_of(&[("a.txt", "123"), ("b.txt", "456")]), &dir.join("over"), 5).unwrap_err();
        assert!(matches!(err, StorageError::Invalid(_)), "{err:?}");
    }

    #[test]
    fn installs_a_zipped_folder_replacing_the_old_version() {
        let dir = temp_dir("install");
        let (staging, addons) = (dir.join("staging"), dir.join("addons"));
        // A zipped folder, as the system's "compress" makes it, with a macOS metadata folder beside it.
        let v1 = dir.join("v1.zip");
        fs::write(&v1, zip_of(&[("demo/addon.json", &manifest("1.0.0")), ("demo/ui/tab.html", "1"), ("__MACOSX/._x", "")]).into_inner()).unwrap();
        assert_eq!(stage(&v1, &staging).unwrap().version, "1.0.0");
        install(&staging, &addons).unwrap();
        assert_eq!(fs::read_to_string(addons.join("demo/ui/tab.html")).unwrap(), "1");
        assert!(!staging.exists());

        let v2 = dir.join("v2.zip");
        fs::write(&v2, zip_of(&[("addon.json", &manifest("2.0.0")), ("ui/tab.html", "2")]).into_inner()).unwrap();
        stage(&v2, &staging).unwrap();
        install(&staging, &addons).unwrap();
        assert_eq!(fs::read_to_string(addons.join("demo/ui/tab.html")).unwrap(), "2");

        // An invalid addon (its page is missing) is refused and leaves nothing staged.
        let bad = dir.join("bad.zip");
        fs::write(&bad, zip_of(&[("addon.json", &manifest("3.0.0"))]).into_inner()).unwrap();
        assert!(matches!(stage(&bad, &staging), Err(StorageError::Invalid(_))));
        assert!(!staging.join("unzipped").exists());
        assert!(install(&staging, &addons).is_err());
    }

    #[test]
    fn uninstalling_keeps_the_data_unless_asked_and_enables_it_again() {
        let dir = temp_dir("uninstall");
        let staging = dir.join("staging");
        for id in ["kept", "gone"] {
            fs::create_dir_all(dir.join("addons").join(id)).unwrap();
            fs::create_dir_all(dir.join("addon-data").join(id)).unwrap();
            fs::write(dir.join("addon-data").join(id).join("storage.json"), "{}").unwrap();
        }
        set_disabled(&dir, "kept", true).unwrap();
        set_disabled(&dir, "gone", true).unwrap();
        note_secret(&dir, "gone", "token", true).unwrap();
        let deleted = std::cell::RefCell::new(Vec::new());
        let delete = |name: &str| {
            deleted.borrow_mut().push(name.to_string());
            Ok(())
        };
        uninstall(&dir, &staging, "kept", false, delete).unwrap();
        uninstall(&dir, &staging, "gone", true, delete).unwrap();
        assert_eq!(*deleted.borrow(), ["token"]);
        assert!(!dir.join("addons/kept").exists() && !dir.join("addons/gone").exists());
        assert!(dir.join("addon-data/kept/storage.json").exists());
        assert!(!dir.join("addon-data/gone").exists());
        assert!(disabled_addons(&dir).is_empty());
        assert!(matches!(uninstall(&dir, &staging, "kept", false, delete), Err(StorageError::NotFound(_))));
        assert!(installed_folder(&dir, "../addon-data").is_err());
    }

    fn write_manifest(dir: &Path, id: &str, version: &str, api: u32) {
        fs::create_dir_all(dir.join(id)).unwrap();
        let m = serde_json::json!({ "api": api, "id": id, "name": id, "version": version, "license": "MIT" });
        fs::write(dir.join(id).join(MANIFEST), m.to_string()).unwrap();
    }

    #[test]
    fn rows_say_what_changes_on_the_next_start() {
        let dir = temp_dir("summaries");
        let api = crate::addons::API;
        let at_start = dir.join("at-start");
        write_manifest(&at_start, "updated", "1.0", api);
        write_manifest(&at_start, "removed", "1.0", api);
        let addons = dir.join("addons");
        write_manifest(&addons, "updated", "2.0", api);
        write_manifest(&addons, "new", "1.0", api);
        write_manifest(&addons, "off", "1.0", api);
        write_manifest(&addons, "broken", "1.0", api + 1);
        set_disabled(&dir, "off", true).unwrap();
        let rows = summaries_of(&load_dir(&addons).unwrap(), &load_dir(&at_start).unwrap().0, &dir);
        let row = |id: &str| rows.iter().find(|r| r.id == id).unwrap();
        let got = |id: &str| (row(id).enabled, row(id).loaded_version.as_deref(), row(id).removed);
        assert_eq!(got("updated"), (true, Some("1.0"), false));
        assert_eq!(row("updated").version, "2.0");
        assert_eq!(got("new"), (true, None, false));
        assert_eq!(got("off"), (false, None, false));
        assert_eq!(got("broken"), (true, None, false));
        assert!(!row("broken").errors.is_empty());
        assert_eq!(got("removed"), (true, Some("1.0"), true));
        assert_eq!(rows.len(), 5);
    }

    #[test]
    fn a_name_list_that_cant_be_read_is_never_saved_over() {
        let dir = temp_dir("lists");
        let list = dir.join("list.json");
        edit_list(&list, "a", true).unwrap();
        edit_list(&list, "b", true).unwrap();
        edit_list(&list, "a", false).unwrap();
        assert_eq!(read_list(&list).unwrap(), ["b"]);
        fs::write(&list, "not json").unwrap();
        assert!(matches!(edit_list(&list, "c", true), Err(StorageError::Corrupt(_))));
        assert_eq!(fs::read_to_string(&list).unwrap(), "not json");
        assert!(disabled_addons(&dir).is_empty());
    }
}
