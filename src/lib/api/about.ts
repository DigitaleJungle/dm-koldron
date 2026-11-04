import { invoke } from "@tauri-apps/api/core";

/** Mirrors `LicenseText` in src-tauri/src/commands/about.rs. `id` is "app" or a pack's folder name. */
export type LicenseText = { id: string; text: string };

/** The app's license, then each installed pack's. Rejects with a StorageError. */
export function licenseTexts(): Promise<LicenseText[]> {
  return invoke("license_texts");
}
