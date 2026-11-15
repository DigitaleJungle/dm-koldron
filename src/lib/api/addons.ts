import { invoke } from "@tauri-apps/api/core";
import type { Character } from "./characters";
import type { PackSummary } from "./packs";

/** Mirrors `AddonSummary` in src-tauri/src/commands/addons.rs. An addon with `errors` isn't loaded. */
export type AddonSummary = {
  id: string;
  name: string;
  version: string;
  license: string;
  settings: { section: string; entry: string } | null;
  /** An item in the Characters screen's Add character menu, opening `entry` in a dialog. */
  characters: { label: string; entry: string } | null;
  /** A page that runs hidden as long as the app is open. */
  background: string | null;
  licenseText: string | null;
  errors: string[];
};

/** Mirrors `NewPack` in src-tauri/src/packs.rs: the manifest, and each data file by its path. */
export type NewPack = { manifest: unknown; files: Record<string, unknown>; images?: unknown; license?: string };

/** Mirrors `PickedFile`: a path inside the picked folder, and the text when the file is UTF-8 text. */
export type PickedFile = { path: string; text: string | null };

/** The addons in the data folder, valid ones first. Rejects with a StorageError. */
export function listAddons(): Promise<AddonSummary[]> {
  return invoke("list_addons");
}

/** Where an addon's page is served: `koldron-addon://localhost/…`, which Windows (WebView2) spells as http. */
export function addonUrl(addon: string, path: string): string {
  const base = navigator.userAgent.includes("Windows") ? "http://koldron-addon.localhost" : "koldron-addon://localhost";
  return `${base}/${addon}/${path}`;
}

// The calls below are made on an addon's behalf; Rust checks each against that addon's permissions.

export function addonPacks(addon: string): Promise<PackSummary[]> {
  return invoke("addon_packs", { addon });
}

export function addonInstallPack(addon: string, pack: NewPack): Promise<PackSummary> {
  return invoke("addon_install_pack", { addon, pack });
}

export function addonRemovePack(addon: string, pack: string): Promise<void> {
  return invoke("addon_remove_pack", { addon, pack });
}

export function addonFetch(addon: string, url: string): Promise<ArrayBuffer> {
  return invoke("addon_fetch", { addon, url });
}

/** Empty when the user cancelled. */
export function addonPickFile(addon: string, extensions: string[]): Promise<ArrayBuffer> {
  return invoke("addon_pick_file", { addon, extensions });
}

/** Null when the user cancelled. */
export function addonPickFolder(addon: string): Promise<PickedFile[] | null> {
  return invoke("addon_pick_folder", { addon });
}

export function addonCharacters(addon: string, campaign: string): Promise<Character[]> {
  return invoke("addon_characters", { addon, campaign });
}

export function addonSaveCharacter(addon: string, campaign: string, character: Character): Promise<Character> {
  return invoke("addon_save_character", { addon, campaign, character });
}

export function addonSetCharacterPicture(addon: string, campaign: string, id: string, bytes: Uint8Array): Promise<Character> {
  return invoke("addon_set_character_picture", { addon, campaign, id, bytes: Array.from(bytes) });
}

export function addonStorageGet(addon: string): Promise<unknown> {
  return invoke("addon_storage_get", { addon });
}

export function addonStorageSet(addon: string, value: unknown): Promise<void> {
  return invoke("addon_storage_set", { addon, value });
}
