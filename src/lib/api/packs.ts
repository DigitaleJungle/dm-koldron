import { invoke } from "@tauri-apps/api/core";

/** Mirrors `PackSummary` in src-tauri/src/commands/packs.rs. A pack with `errors` isn't loaded. */
export type PackSummary = {
  id: string;
  name: string;
  version: string;
  license: string;
  entries: number;
  enabled: boolean;
  /** Installed by an addon in the data folder; false for a bundled pack. */
  installed: boolean;
  errors: string[];
};

/** The installed packs, valid ones first. Rejects with a StorageError. */
export function listPacks(): Promise<PackSummary[]> {
  return invoke("list_packs");
}

/** Removes an installed pack and its kept images; a bundled pack with that id comes back. Rejects with a StorageError. */
export function removePack(id: string): Promise<void> {
  return invoke("remove_pack", { pack: id });
}
