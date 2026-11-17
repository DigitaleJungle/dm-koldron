//! The addon runtime: the addons in the data folder, the `koldron-addon` scheme that serves their pages, and the calls
//! their sandboxed frames make through the window's bridge. Every call names the addon and is checked here against the
//! permissions its manifest declares; the window is not trusted to have checked.

use crate::addons::{https_host, load_dir, Addon};
use crate::commands::about::license_in;
use crate::commands::campaigns::{active_dir, dirs, now_ms};
use crate::commands::characters::{self, Character, Editor};
use crate::commands::search;
use crate::commands::packs::{capped, disabled, installed_dir, loaded, reload, summaries, PackSummary};
use crate::packs::{self, safe_path, Invalid, NewPack};
use crate::storage::{blocking, data_dir, write_atomic, write_atomic_bytes, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

// shortcut: addons are read once per run; there's no install UI, so a new addon needs a restart. Reload here when one comes.
static ADDONS: OnceLock<Loaded> = OnceLock::new();

/// Blocking: call it inside `blocking`.
fn addons(app: &AppHandle) -> Result<&'static Loaded, StorageError> {
    if let Some(loaded) = ADDONS.get() {
        return Ok(loaded);
    }
    let loaded = load_dir(&data_dir(app)?.join("addons"))?;
    Ok(ADDONS.get_or_init(|| loaded))
}

/// Each loaded addon's id with the character fields it owns (a character linked to an addon that isn't loaded is an
/// ordinary one). Blocking.
pub fn owned_fields(app: &AppHandle) -> Result<Vec<(String, Vec<String>)>, StorageError> {
    Ok(addons(app)?.0.iter().map(|a| (a.manifest.id.clone(), owns(a).to_vec())).collect())
}

fn owns(addon: &Addon) -> &[String] {
    addon.manifest.characters.as_ref().map_or(&[], |c| c.owns.as_slice())
}

fn addon(app: &AppHandle, id: &str) -> Result<&'static Addon, StorageError> {
    addons(app)?.0.iter().find(|a| a.manifest.id == id).ok_or_else(|| StorageError::NotFound(format!("No addon \"{id}\" is loaded.")))
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
    pub library: Option<crate::addons::Tab>,
    /// Whether the `library` tab is shown (`library.setVisible`); true for an addon without one.
    pub library_visible: bool,
    /// The license file in its folder, if it has one.
    pub license_text: Option<String>,
    /// Empty for a loaded addon. An addon with errors isn't loaded.
    pub errors: Vec<String>,
}

/// `data`: the data folder.
fn summaries_of((addons, invalid): &Loaded, data: &Path) -> Vec<AddonSummary> {
    let dir = data.join("addons");
    let valid = addons.iter().map(|a| AddonSummary {
        id: a.manifest.id.clone(),
        name: a.manifest.name.clone(),
        version: a.manifest.version.clone(),
        license: a.manifest.license.clone(),
        settings: a.manifest.settings.clone(),
        characters: a.manifest.characters.clone(),
        background: a.manifest.background.clone(),
        library: a.manifest.library.clone(),
        library_visible: library_visible(data, &a.manifest.id),
        license_text: license_in(&a.folder),
        errors: Vec::new(),
    });
    let invalid = invalid.iter().map(|i| {
        AddonSummary {
            name: i.name.clone().unwrap_or_else(|| i.folder.clone()),
            license_text: license_in(&dir.join(&i.folder)),
            id: i.folder.clone(),
            version: String::new(),
            license: String::new(),
            settings: Vec::new(),
            characters: None,
            background: None,
            library: None,
            library_visible: true,
            errors: capped(&i.errors),
        }
    });
    valid.chain(invalid).collect()
}

#[tauri::command]
pub async fn list_addons(app: AppHandle) -> Result<Vec<AddonSummary>, StorageError> {
    blocking(move || Ok(summaries_of(addons(&app)?, &data_dir(&app)?))).await
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
    keyring::Entry::new(&app.config().identifier, &format!("addon:{addon}:{name}")).map_err(|e| keychain_error(&e))
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
        secret(&app, &addon, &name)?.set_password(&value).map_err(|e| keychain_error(&e))
    })
    .await
}

/// `secrets.delete`: also when there's none.
#[tauri::command]
pub async fn addon_secret_delete(app: AppHandle, addon: String, name: String) -> Result<(), StorageError> {
    blocking(move || match secret(&app, &addon, &name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(keychain_error(&e)),
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

/// Streams `response` into `target` through a file beside it, which only replaces `target` once it is complete and,
/// when `md5` is given, matches it. Returns the file's MD5.
fn save_download(mut response: ureq::http::Response<ureq::Body>, target: &Path, md5: Option<&str>) -> Result<String, StorageError> {
    use md5::{Digest, Md5};
    let mut partial = target.as_os_str().to_owned();
    partial.push(PARTIAL);
    let partial = PathBuf::from(partial);
    let result = (|| {
        let mut out = fs::File::create(&partial)?;
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
        let got = format!("{:x}", hasher.finalize());
        if md5.is_some_and(|want| !want.eq_ignore_ascii_case(&got)) {
            return Err(StorageError::Invalid("The downloaded file doesn't match its checksum.".into()));
        }
        fs::rename(&partial, target)?;
        Ok(got)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&partial); // only a leftover partial file otherwise
    }
    result
}

/// `folders.download`: downloads an https file (from a `net:<host>` the addon names) to `path` in the folder
/// (`folders:write`), making its folders; a file already there is replaced only by a complete, matching download.
#[tauri::command]
pub async fn addon_download_file(app: AppHandle, addon: String, folder: String, path: String, url: String, md5: Option<String>) -> Result<String, StorageError> {
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
        save_download(response, &target, md5.as_deref())
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
        for bad in ["../out.pdf", "Pub/../../out.pdf", "", "/abs.pdf", "taken", "x.pdf.download"] {
            assert!(download_target(&lib, bad).is_err(), "{bad}");
        }
        fs::write(&target, "hello").unwrap();
        assert_eq!(md5_of(&target).unwrap(), "5d41402abc4b2a76b9719d911017c592");
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
}
