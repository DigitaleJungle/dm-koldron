//! Update rules (Settings → Rules): the SRD pack from the newest `5e-database-vX.Y.Z` release of 5e-bits/5e-srd-api
//! (not its `5e-srd-api-v…` server releases). The release's 2024 English data files replace the SRD pack's; ours stay
//! (the rules converted from the PDF, the encounter budgets, the images list, the license). `packs::install` validates
//! it completely and swaps it in as the installed `srd` pack; removing that brings back the SRD the app ships with. On
//! any failure the current SRD stays.

use crate::commands::packs::{disabled, installed_dir, loaded, lock_pack_change, reload, summaries, PackSummary};
use crate::packs::{self, NewPack};
use crate::storage::{blocking, data_dir, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const SRD: &str = "srd";
const REPO: &str = "https://api.github.com/repos/5e-bits/5e-srd-api";
const TAG_PREFIX: &str = "5e-database-v";
/// Where the release's zip has the data the SRD pack uses.
const DATA_DIR: &str = "/packages/5e-database/src/2024/en/";
/// GitHub's zip of a release is about 10 MB; one data file about 2 MB.
const MAX_ZIP: u64 = 200 * 1024 * 1024;
const MAX_FILE: u64 = 50 * 1024 * 1024;
const MAX_RELEASES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct SrdCheck {
    installed: String,
    /// The newest release's version, when it's newer than the installed one.
    latest: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// "5e-database-v7.10.0" → [7, 10, 0]; None for any other tag.
fn version_of(tag: &str) -> Option<[u64; 3]> {
    let parts = tag.strip_prefix(TAG_PREFIX)?.split('.').map(|p| p.bytes().all(|b| b.is_ascii_digit()).then(|| p.parse().ok()).flatten());
    parts.collect::<Option<Vec<u64>>>()?.try_into().ok()
}

/// The newest 5e-database release (not a draft or pre-release), when it's newer than `installed`.
fn newer_release(releases: &[Release], installed: &str) -> Option<String> {
    let newest = releases.iter().filter(|r| !r.draft && !r.prerelease).filter_map(|r| version_of(&r.tag_name)).max()?;
    let current = version_of(&format!("{TAG_PREFIX}{installed}")).unwrap_or_default();
    (newest > current).then(|| newest.map(|n| n.to_string()).join("."))
}

/// `current` (the SRD pack's files) with each data file the release's zip has replaced, at `version`.
fn updated(current: NewPack, zip: &[u8], version: &str) -> Result<NewPack, StorageError> {
    let unreadable = |e: zip::result::ZipError| StorageError::Invalid(format!("The release's zip can't be read: {e}"));
    let file_name = |path: &str| path.rsplit('/').next().unwrap_or(path).to_string();
    let wanted: HashSet<String> = current.files.keys().map(|p| file_name(p)).collect();
    let mut upstream: BTreeMap<String, Value> = BTreeMap::new();
    let mut archive = zip::ZipArchive::new(Cursor::new(zip)).map_err(unreadable)?;
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(unreadable)?;
        let path = file.name().map_err(unreadable)?.into_owned();
        let Some((_, name)) = path.split_once(DATA_DIR) else { continue };
        if name.contains('/') || !wanted.contains(name) {
            continue;
        }
        let name = name.to_string();
        let mut bytes = Vec::new();
        file.take(MAX_FILE).read_to_end(&mut bytes)?;
        let value = serde_json::from_slice(&bytes).map_err(|e| StorageError::Invalid(format!("{name}: {e}")))?;
        upstream.insert(name, value);
    }
    if upstream.is_empty() {
        return Err(StorageError::Invalid("The release has none of the SRD's data files.".into()));
    }
    // shortcut: a data file new in the release (a new type) isn't added; add it when the pack schema knows its type.
    let mut files = current.files;
    for (path, value) in files.iter_mut() {
        if let Some(new) = upstream.remove(&file_name(path)) {
            *value = new;
        }
    }
    let mut manifest = current.manifest;
    manifest["version"] = version.into();
    Ok(NewPack { manifest, files, ..current })
}

fn get(url: &str, limit: u64) -> Result<Vec<u8>, StorageError> {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    let agent = AGENT.get_or_init(|| ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(300))).build().into());
    let failed = |e: ureq::Error| StorageError::Io(e.to_string());
    let mut response = agent.get(url).header("Accept", "application/vnd.github+json").call().map_err(failed)?;
    response.body_mut().with_config().limit(limit).read_to_vec().map_err(failed)
}

/// The SRD pack in use: its version, and the folder that holds it (the installed one before the bundled one).
fn srd_in_use(app: &AppHandle) -> Result<(String, PathBuf), StorageError> {
    let loaded = loaded(app)?;
    let Some(pack) = loaded.0.iter().find(|p| p.manifest.id == SRD) else {
        return Err(StorageError::Invalid("There's no SRD pack.".into()));
    };
    let dir = if pack.installed {
        installed_dir(app)?
    } else {
        app.path().resource_dir().map_err(|e| StorageError::Io(e.to_string()))?.join("packs")
    };
    Ok((pack.manifest.version.clone(), dir))
}

/// Check for updates: the installed SRD version, and the newest release's when it's newer.
#[tauri::command]
pub async fn check_srd_update(app: AppHandle) -> Result<SrdCheck, StorageError> {
    blocking(move || {
        let (installed, _) = srd_in_use(&app)?;
        let releases: Vec<Release> =
            serde_json::from_slice(&get(&format!("{REPO}/releases?per_page=100"), MAX_RELEASES)?).map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(SrdCheck { latest: newer_release(&releases, &installed), installed })
    })
    .await
}

/// Update: downloads that release, puts its data in the SRD pack and installs it. The pack as it is now.
#[tauri::command]
pub async fn update_srd(app: AppHandle, version: String) -> Result<PackSummary, StorageError> {
    blocking(move || {
        let tag = format!("{TAG_PREFIX}{version}");
        if version_of(&tag).is_none() {
            return Err(StorageError::Invalid(format!("\"{version}\" isn't a 5e-database version.")));
        }
        let zip = get(&format!("{REPO}/zipball/{tag}"), MAX_ZIP)?;
        {
            let _guard = lock_pack_change();
            let (_, dir) = srd_in_use(&app)?;
            let new = updated(packs::read(&dir, SRD)?, &zip, &version)?;
            packs::install(&installed_dir(&app)?, &data_dir(&app)?.join("packs-staging-srd"), &new)?;
            reload();
        }
        let summary = summaries(&*loaded(&app)?, &disabled(&app)?).into_iter().find(|p| p.id == SRD);
        summary.ok_or_else(|| StorageError::Io("The SRD was updated but didn't load.".into()))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn release(tag: &str, prerelease: bool) -> Release {
        Release { tag_name: tag.into(), draft: false, prerelease }
    }

    #[test]
    fn only_a_newer_5e_database_release_counts() {
        let releases = [release("5e-srd-api-v9.0.0", false), release("5e-database-v7.9.1", false), release("5e-database-v8.0.0", true), release("5e-database-v7.10.0", false)];
        assert_eq!(newer_release(&releases, "7.9.1").as_deref(), Some("7.10.0"));
        assert_eq!(newer_release(&releases, "7.10.0"), None);
        assert_eq!(version_of("5e-database-v7.+1.0"), None);
        assert_eq!(version_of("5e-database-v7.1"), None);
    }

    fn zip_of(files: &[(&str, &str)]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (path, text) in files {
            zip.start_file(*path, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(text.as_bytes()).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn the_release_replaces_its_files_and_ours_stay() {
        let current = NewPack {
            manifest: serde_json::json!({ "id": "srd", "version": "7.9.1" }),
            files: [
                ("source/2024/en/5e-SRD-Spells.json".to_string(), serde_json::json!(["old"])),
                ("source/2024/en/5e-SRD-Rules.json".to_string(), serde_json::json!(["ours"])),
            ]
            .into(),
            images: Some(serde_json::json!({ "host": "h" })),
            license: Some("L".into()),
        };
        let zip = zip_of(&[
            ("5e-bits-5e-srd-api-abc/packages/5e-database/src/2024/en/5e-SRD-Spells.json", r#"["new"]"#),
            ("5e-bits-5e-srd-api-abc/packages/5e-database/src/2014/en/5e-SRD-Rules.json", r#"["2014"]"#),
            ("5e-bits-5e-srd-api-abc/packages/5e-database/src/2024/en/5e-SRD-New.json", "[]"),
        ]);
        let new = updated(current, &zip, "7.10.0").unwrap();
        assert_eq!(new.manifest["version"], "7.10.0");
        assert_eq!(new.files["source/2024/en/5e-SRD-Spells.json"], serde_json::json!(["new"]));
        assert_eq!(new.files["source/2024/en/5e-SRD-Rules.json"], serde_json::json!(["ours"]));
        assert_eq!(new.files.len(), 2);
        assert_eq!((new.images, new.license.as_deref()), (Some(serde_json::json!({ "host": "h" })), Some("L")));

        let empty = NewPack { manifest: serde_json::json!({}), files: BTreeMap::new(), images: None, license: None };
        assert!(matches!(updated(empty, &zip, "7.10.0"), Err(StorageError::Invalid(_))));
    }
}
