//! The addon runtime: the addons in the data folder, the `koldron-addon` scheme that serves their pages, and the calls
//! their sandboxed frames make through the window's bridge. Every call names the addon and is checked here against the
//! permissions its manifest declares; the window is not trusted to have checked.

use crate::addons::{https_host, load_dir, Addon};
use crate::commands::about::license_in;
use crate::commands::packs::{capped, disabled, installed_dir, loaded, reload, summaries, PackSummary};
use crate::packs::{self, safe_path, Invalid, NewPack};
use crate::storage::{blocking, data_dir, write_atomic, StorageError};
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
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
    pub settings: Option<crate::addons::Panel>,
    /// The license file in its folder, if it has one.
    pub license_text: Option<String>,
    /// Empty for a loaded addon. An addon with errors isn't loaded.
    pub errors: Vec<String>,
}

fn summaries_of((addons, invalid): &Loaded, dir: &Path) -> Vec<AddonSummary> {
    let valid = addons.iter().map(|a| AddonSummary {
        id: a.manifest.id.clone(),
        name: a.manifest.name.clone(),
        version: a.manifest.version.clone(),
        license: a.manifest.license.clone(),
        settings: a.manifest.settings.clone(),
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
            settings: None,
            errors: capped(&i.errors),
        }
    });
    valid.chain(invalid).collect()
}

#[tauri::command]
pub async fn list_addons(app: AppHandle) -> Result<Vec<AddonSummary>, StorageError> {
    blocking(move || Ok(summaries_of(addons(&app)?, &data_dir(&app)?.join("addons")))).await
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

/// `net.fetch`: a GET over https to a host the addon's manifest names (`net:<host>`). The body comes back as bytes.
#[tauri::command]
pub async fn addon_fetch(app: AppHandle, addon: String, url: String) -> Result<ipc::Response, StorageError> {
    blocking(move || {
        let found = self::addon(&app, &addon)?;
        let host = https_host(&url).ok_or_else(|| StorageError::Invalid(format!("\"{url}\" isn't a plain https:// address.")))?;
        require(found, &format!("net:{host}"))?;
        static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
        let agent = AGENT.get_or_init(|| ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(120))).build().into());
        let failed = |e: ureq::Error| StorageError::Io(format!("{url}: {e}"));
        let mut response = agent.get(&url).call().map_err(failed)?;
        let body = response.body_mut().with_config().limit(MAX_BYTES).read_to_vec().map_err(failed)?;
        Ok(ipc::Response::new(body))
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
    let (mut files, mut total, mut todo) = (Vec::new(), 0u64, vec![root.to_path_buf()]);
    while let Some(dir) = todo.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                todo.push(entry.path());
            } else if kind.is_file() {
                total += entry.metadata()?.len();
                if files.len() == MAX_FILES || total > MAX_BYTES {
                    return Err(StorageError::Invalid(format!(
                        "The folder has more than {MAX_FILES} files or {} MB.",
                        MAX_BYTES / 1024 / 1024
                    )));
                }
                let relative = entry.path().strip_prefix(root).expect("found under root").to_owned();
                let path = relative.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
                files.push(PickedFile { path, text: String::from_utf8(fs::read(entry.path())?).ok() });
            } // symlinks are skipped: they could lead out of the folder the user picked
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
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
    if !safe_path(path) {
        return Err(StatusCode::NOT_FOUND);
    }
    let root = folder.canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    let file = root.join(path).canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    if !file.starts_with(&root) || !file.is_file() {
        return Err(StatusCode::NOT_FOUND);
    }
    fs::read(file).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
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
