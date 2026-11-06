//! Writes `packs/srd/images.json`: every image the SRD data names that the SRD API server actually has, with its
//! download size. The app only downloads images listed there, and adds up the sizes for its estimate.
//! Run again after updating the SRD data: `cargo run --example image_manifest` (in `src-tauri/`).

use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

const HOST: &str = "https://www.dnd5eapi.co";

fn main() {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs/srd");
    let manifest: Value = serde_json::from_str(&fs::read_to_string(pack.join("pack.json")).unwrap()).unwrap();
    let mut paths = BTreeSet::new();
    for file in manifest["files"].as_array().unwrap() {
        let data: Value = serde_json::from_str(&fs::read_to_string(pack.join(file["path"].as_str().unwrap())).unwrap()).unwrap();
        for entry in data.as_array().into_iter().flatten() {
            if let Some(image) = entry["image"].as_str() {
                paths.insert(image.to_string());
            }
        }
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).build().into();
    let mut images = Map::new();
    for path in &paths {
        // A missing image is a 403 from the server; a malformed path isn't a URL. Both are left out.
        let size = agent
            .head(&format!("{HOST}{path}"))
            .call()
            .ok()
            .and_then(|r| r.headers().get("content-length")?.to_str().ok()?.parse::<u64>().ok());
        match size {
            Some(size) => drop(images.insert(path.clone(), json!(size))),
            None => eprintln!("left out: {path}"),
        }
    }
    println!("{} of {} images", images.len(), paths.len());
    let out = json!({ "host": HOST, "images": images });
    fs::write(pack.join("images.json"), serde_json::to_string_pretty(&out).unwrap() + "\n").unwrap();
}
