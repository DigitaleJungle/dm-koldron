//! Pack images (Settings → Rules → Enable images). A pack's `images.json` lists the images its entries name that can
//! be downloaded; only those are ever fetched. They're kept (larger than 1024 px: shrunk to that) in `images/<pack id>/` in the app data folder:
//! all at once ("Download all now") or each the first time it's shown ("When browsing"). Images never ship in a pack.

use crate::commands::packs::loaded;
use crate::commands::settings;
use crate::packs::{Images, Pack};
use crate::storage::{blocking, data_dir, write_atomic_bytes, StorageError};
use serde::Serialize;
use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::AppHandle;

pub const DIR: &str = "images";
/// Larger images are shrunk to this on their longest side; smaller ones are kept as downloaded.
const MAX_SIDE: u32 = 1024;
const MAX_DOWNLOAD: u64 = 20 * 1024 * 1024;
const WORKERS: usize = 4;

/// What a pack offers: how many images, how much to download for all of them, and how many are kept already.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackImages {
    pub pack: String,
    pub count: usize,
    pub bytes: u64,
    pub kept: usize,
}

/// How many of the listed images are kept in `dir`.
fn kept(dir: &Path, images: &Images) -> usize {
    images.images.keys().filter(|image| file_for(dir, image).is_file()).count()
}

#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// Downloaded or already kept.
    pub done: usize,
    /// The server didn't send it, or it wasn't an image.
    pub failed: usize,
    pub total: usize,
    pub cancelled: bool,
}

/// The kept file for an image path: the path with anything but letters, digits, `.` and `-` turned into `_`, so it's
/// always one file name inside the pack's folder.
fn file_for(dir: &Path, image: &str) -> PathBuf {
    let name: String = image.trim_start_matches('/').chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect();
    dir.join(name)
}

/// Keeps the downloaded image, shrunk to `MAX_SIDE` when it's larger. An unreadable image is `Invalid`; a disk
/// problem is `Io`.
fn keep(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let decoded = image::load_from_memory(bytes).map_err(|e| StorageError::Invalid(format!("Not an image: {e}")))?;
    if decoded.width().max(decoded.height()) <= MAX_SIDE {
        return write_atomic_bytes(path, bytes);
    }
    let mut out = vec![];
    decoded
        .resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Lanczos3)
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| StorageError::Io(e.to_string()))?;
    write_atomic_bytes(path, &out)
}

/// The kept image for an entry's `image`, downloading it first when needed. None: the pack doesn't list it, or it
/// can't be downloaded now (offline); the entry then shows without one.
pub fn image(dir: &Path, images: &Images, image: &str, fetch: impl FnOnce(&str) -> Option<Vec<u8>>) -> Result<Option<PathBuf>, StorageError> {
    if !images.images.contains_key(image) {
        return Ok(None);
    }
    let path = file_for(dir, image);
    if path.is_file() {
        return Ok(Some(path));
    }
    let Some(bytes) = fetch(&format!("{}{image}", images.host)) else { return Ok(None) };
    match keep(&path, &bytes) {
        Ok(()) => Ok(Some(path)),
        Err(StorageError::Invalid(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Downloads every listed image not kept yet, `WORKERS` at a time, reporting after each. Stops early when `cancel` is
/// set. A disk error stops it and is returned; what was kept stays.
pub fn download_all(
    dir: &Path,
    images: &Images,
    fetch: impl Fn(&str) -> Option<Vec<u8>> + Sync,
    cancel: &AtomicBool,
    report: impl Fn(Progress) + Sync,
) -> Result<Progress, StorageError> {
    let paths: Vec<&String> = images.images.keys().collect();
    let (next, done, failed) = (AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0));
    let error = Mutex::new(None);
    let progress = || Progress { done: done.load(Ordering::SeqCst), failed: failed.load(Ordering::SeqCst), total: paths.len(), cancelled: false };
    report(progress());
    std::thread::scope(|scope| {
        for _ in 0..WORKERS {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= paths.len() || cancel.load(Ordering::SeqCst) {
                    break;
                }
                let path = file_for(dir, paths[i]);
                let ok = path.is_file()
                    || match fetch(&format!("{}{}", images.host, paths[i])).map(|bytes| keep(&path, &bytes)) {
                        Some(Ok(())) => true,
                        None | Some(Err(StorageError::Invalid(_))) => false,
                        Some(Err(e)) => {
                            error.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert(e);
                            cancel.store(true, Ordering::SeqCst);
                            break;
                        }
                    };
                (if ok { &done } else { &failed }).fetch_add(1, Ordering::SeqCst);
                report(progress());
            });
        }
    });
    if let Some(e) = error.into_inner().unwrap_or_else(|p| p.into_inner()) {
        return Err(e);
    }
    Ok(Progress { cancelled: cancel.load(Ordering::SeqCst), ..progress() })
}

/// Removes everything kept for a pack.
pub fn delete(dir: &Path) -> Result<(), StorageError> {
    match fs::remove_dir_all(dir) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

fn download(url: &str) -> Option<Vec<u8>> {
    // One client for all downloads, so connections to the server are reused.
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    let agent = AGENT.get_or_init(|| ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).build().into());
    // Offline, a timeout or a missing image all mean: no image this time.
    let mut response = agent.get(url).call().ok()?;
    response.body_mut().with_config().limit(MAX_DOWNLOAD).read_to_vec().ok()
}

/// The pack's listed images and its kept-images folder; an unknown pack or one without images is `NotFound`.
fn pack_images(app: &AppHandle, pack: &str) -> Result<(&'static Pack, PathBuf), StorageError> {
    let found = loaded(app)?.0.iter().find(|p| p.manifest.id == pack && p.images.is_some());
    let found = found.ok_or_else(|| StorageError::NotFound(format!("The pack \"{pack}\" has no images.")))?;
    // A loaded pack's id is lowercase letters, digits and dashes, so it's one folder name.
    Ok((found, data_dir(app)?.join(DIR).join(pack)))
}

/// The download running for each pack, so it can be cancelled; one per pack at a time.
static RUNNING: Mutex<Vec<(String, Arc<AtomicBool>)>> = Mutex::new(Vec::new());

fn running() -> std::sync::MutexGuard<'static, Vec<(String, Arc<AtomicBool>)>> {
    RUNNING.lock().unwrap_or_else(|p| p.into_inner())
}

#[tauri::command]
pub async fn list_pack_images(app: AppHandle) -> Result<Vec<PackImages>, StorageError> {
    let packs = &loaded(&app)?.0;
    let dir = data_dir(&app)?.join(DIR);
    blocking(move || {
        Ok(packs
            .iter()
            .filter_map(|p| {
                let images = p.images.as_ref()?;
                let kept = kept(&dir.join(&p.manifest.id), images);
                Some(PackImages { pack: p.manifest.id.clone(), count: images.images.len(), bytes: images.images.values().sum(), kept })
            })
            .collect())
    })
    .await
}

/// The kept image for an entry, downloading it when needed; None when the pack's images are off or it has none.
#[tauri::command]
pub async fn entry_image(app: AppHandle, pack: String, image: String) -> Result<Option<PathBuf>, StorageError> {
    let on = settings::load(&data_dir(&app)?.join(settings::FILE_NAME)).map(|s| s.image_packs.contains(&pack)).unwrap_or(false);
    if !on {
        return Ok(None);
    }
    let Ok((found, dir)) = pack_images(&app, &pack) else { return Ok(None) };
    blocking(move || self::image(&dir, found.images.as_ref().unwrap(), &image, download)).await
}

#[tauri::command]
pub async fn download_images(app: AppHandle, pack: String, on_progress: Channel<Progress>) -> Result<Progress, StorageError> {
    let (found, dir) = pack_images(&app, &pack)?;
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut running = running();
        if running.iter().any(|(id, _)| *id == pack) {
            return Err(StorageError::Invalid("The images are already downloading.".into()));
        }
        running.push((pack.clone(), cancel.clone()));
    }
    let result = blocking(move || {
        download_all(&dir, found.images.as_ref().unwrap(), download, &cancel, |p| {
            let _ = on_progress.send(p); // the window may be gone; the download carries on
        })
    })
    .await;
    running().retain(|(id, _)| *id != pack);
    result
}

#[tauri::command]
pub fn cancel_image_download(pack: String) {
    for (_, cancel) in running().iter().filter(|(id, _)| *id == pack) {
        cancel.store(true, Ordering::SeqCst);
    }
}

/// Cancels a running download, then removes the pack's kept images.
#[tauri::command]
pub async fn delete_images(app: AppHandle, pack: String) -> Result<(), StorageError> {
    cancel_image_download(pack.clone());
    let (_, dir) = pack_images(&app, &pack)?;
    // shortcut: an image a cancelled download was writing just then can land after this; harmless, it's re-deleted next time.
    blocking(move || delete(&dir)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;
    use std::collections::BTreeMap;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("koldron-images-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut out = vec![];
        image::RgbaImage::new(width, height).write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    }

    fn listed(paths: &[&str]) -> Images {
        Images { host: "https://img.example".into(), images: paths.iter().map(|p| (p.to_string(), 100)).collect::<BTreeMap<_, _>>() }
    }

    #[test]
    fn an_image_is_downloaded_once_shrunk_and_only_when_listed() {
        let dir = temp_dir("one");
        let images = listed(&["/api/images/monsters/aboleth.png"]);
        let path = image(&dir, &images, "/api/images/monsters/aboleth.png", |url| {
            assert_eq!(url, "https://img.example/api/images/monsters/aboleth.png");
            Some(png(2048, 1024))
        })
        .unwrap()
        .unwrap();
        assert_eq!(path, dir.join("api_images_monsters_aboleth.png"));
        assert_eq!(image::open(&path).unwrap().dimensions(), (MAX_SIDE, MAX_SIDE / 2), "shrunk, same proportions");
        // A small image is kept as it came, never enlarged.
        let small = png(600, 300);
        let kept = image(&dir, &listed(&["/small.png"]), "/small.png", |_| Some(small.clone())).unwrap().unwrap();
        assert_eq!(fs::read(kept).unwrap(), small);
        // Kept: shown offline without downloading again.
        assert_eq!(image(&dir, &images, "/api/images/monsters/aboleth.png", |_| panic!("again")).unwrap(), Some(path));
        // Not listed, offline, or not an image: no picture, no error.
        assert_eq!(image(&dir, &images, "/api/images/monsters/../../settings.json", |_| panic!("not listed")).unwrap(), None);
        let other = listed(&["/a.png"]);
        assert_eq!(image(&dir, &other, "/a.png", |_| None).unwrap(), None);
        assert_eq!(image(&dir, &other, "/a.png", |_| Some(b"nope".to_vec())).unwrap(), None);
    }

    #[test]
    fn a_listed_path_always_maps_to_one_file_in_the_folder() {
        let dir = Path::new("x");
        assert_eq!(file_for(dir, "/api +2/images/a (b).png"), dir.join("api__2_images_a__b_.png"));
        assert_eq!(file_for(dir, "/../..").parent(), Some(dir));
    }

    #[test]
    fn download_all_skips_kept_images_counts_failures_and_can_be_cancelled() {
        let dir = temp_dir("all");
        let images = listed(&["/a.png", "/b.png", "/missing.png", "/text.png"]);
        let fetch = |url: &str| match url {
            u if u.ends_with("/missing.png") => None,
            u if u.ends_with("/text.png") => Some(b"nope".to_vec()),
            _ => Some(png(10, 10)),
        };
        let reports = Mutex::new(vec![]);
        let result = download_all(&dir, &images, fetch, &AtomicBool::new(false), |p| reports.lock().unwrap().push(p)).unwrap();
        assert_eq!(result, Progress { done: 2, failed: 2, total: 4, cancelled: false });
        assert_eq!(reports.lock().unwrap().len(), 5, "once at the start and after each image");
        assert!(dir.join("a.png").is_file() && dir.join("b.png").is_file());

        assert_eq!(kept(&dir, &images), 2);
        // A second run downloads nothing it already has.
        let again = download_all(&dir, &listed(&["/a.png", "/b.png"]), |_| panic!("again"), &AtomicBool::new(false), |_| {}).unwrap();
        assert_eq!(again.done, 2);

        let cancelled = download_all(&temp_dir("cancel"), &images, fetch, &AtomicBool::new(true), |_| {}).unwrap();
        assert_eq!((cancelled.done, cancelled.cancelled), (0, true));

        delete(&dir).unwrap();
        assert!(!dir.exists());
        delete(&dir).unwrap(); // nothing to delete is fine
    }
}
