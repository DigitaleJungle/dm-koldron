import { invoke } from "@tauri-apps/api/core";

/** Mirrors `PackSummary` in src-tauri/src/commands/packs.rs. A pack with `errors` isn't loaded. */
export type PackSummary = {
  id: string;
  name: string;
  version: string;
  license: string;
  entries: number;
  enabled: boolean;
  /** Installed in the data folder (by an addon or Update rules); false for a bundled pack. */
  installed: boolean;
  errors: string[];
};

/** The installed packs, valid ones first. Rejects with a StorageError. */
export function listPacks(): Promise<PackSummary[]> {
  return invoke("list_packs");
}

/** Mirrors `SrdCheck` in src-tauri/src/commands/rules_update.rs. `latest` only when newer than `installed`. */
export type SrdCheck = { installed: string; latest: string | null };

/** Update rules: asks GitHub for the newest 5e-database release. Rejects with a StorageError. */
export function checkSrdUpdate(): Promise<SrdCheck> {
  return invoke("check_srd_update");
}

/** Update rules: installs that release's data as the SRD pack; on failure the SRD stays. Rejects with a StorageError. */
export function updateSrd(version: string): Promise<PackSummary> {
  return invoke("update_srd", { version });
}

/** Removes an installed pack and its kept images; a bundled pack with that id comes back. Rejects with a StorageError. */
export function removePack(id: string): Promise<void> {
  return invoke("remove_pack", { pack: id });
}
