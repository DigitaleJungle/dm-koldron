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
