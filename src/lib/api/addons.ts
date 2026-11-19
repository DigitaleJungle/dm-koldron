import { invoke } from "@tauri-apps/api/core";
import type { Character } from "./characters";
import type { PackSummary } from "./packs";

/** Mirrors `AddonSummary` in src-tauri/src/commands/addons.rs. An addon with `errors` isn't loaded. */
export type AddonSummary = {
  id: string;
  name: string;
  version: string;
  license: string;
  /** Its pages in Settings sections, at most one per section; `label`: its tab's name in Integrations. */
  settings: { section: string; entry: string; label: string | null }[];
  /** An item in the Characters screen's Add character menu, opening `entry` in a dialog; `owns`: the fields of the
   * characters it adds that it keeps up to date (the app can't change those while the addon is loaded). */
  characters: { label: string; entry: string; owns: string[] } | null;
  /** A page that runs hidden as long as the app is open. */
  background: string | null;
  /** A page on the Start screen, beside the campaigns; it sets its own height (0 hides it). */
  start: string | null;
  /** A page shown in the search preview pane for the addon's results (`{ id, state }` in its context). */
  preview: string | null;
  /** A tab in the app-wide Library labelled `label`, showing the page `entry`; `sidebar`: the header can hide its side
   * column (`sidebarHidden` in the page's context). */
  library: { label: string; entry: string; sidebar: boolean } | null;
  /** Whether that tab is shown (the addon's `library.setVisible`). */
  libraryVisible: boolean;
  /** A widget on a campaign's Home labelled `label`, showing `entry` (`{ campaign, width }` in its context). */
  home: { label: string; entry: string } | null;
  licenseText: string | null;
  errors: string[];
  /** Whether the user has it enabled; a disabled one isn't loaded on the next start. */
  enabled: boolean;
  /** The version running this session; null when it isn't (disabled, invalid, or installed since start). */
  loadedVersion: string | null;
  /** Uninstalled since start: its pages and calls stop at once, the rest goes on the restart. */
  removed: boolean;
};

/** The page an addon shows in a Settings section, if it has one there. */
export const panelIn = (addon: AddonSummary, section: string) => addon.settings.find((p) => p.section === section)?.entry;

/** Mirrors `NewPack` in src-tauri/src/packs.rs: the manifest, and each data file by its path. */
export type NewPack = { manifest: unknown; files: Record<string, unknown>; images?: unknown; license?: string };

/** Mirrors `PickedFile`: a path inside the picked folder, and the text when the file is UTF-8 text. */
export type PickedFile = { path: string; text: string | null };

/** The addons in the data folder as it is now, valid ones first, then the ones uninstalled since start. Rejects with
 * a StorageError. */
export function listAddons(): Promise<AddonSummary[]> {
  return invoke("list_addons");
}

/** Mirrors `StagedAddon`: a picked .zip, unzipped and checked, waiting to be installed. `replaces`: the version of
 * the installed addon with this id ("" when unreadable), or null when there's none. */
export type StagedAddon = { id: string; name: string; version: string; permissions: string[]; replaces: string | null };

/** The user picks an addon's .zip; null when cancelled. Rejects with a StorageError when it isn't a valid addon. */
export function stageAddon(): Promise<StagedAddon | null> {
  return invoke("stage_addon");
}

/** Installs the staged addon (replacing one with its id, its saved data kept); it loads after a restart. */
export function installStagedAddon(): Promise<void> {
  return invoke("install_staged_addon");
}

export function discardStagedAddon(): Promise<void> {
  return invoke("discard_staged_addon");
}

/** Takes effect on the next start. */
export function setAddonEnabled(id: string, enabled: boolean): Promise<void> {
  return invoke("set_addon_enabled", { id, enabled });
}

/** Takes effect on the next start. `deleteData`: its saved data and secrets go too; packs it installed stay. */
export function uninstallAddon(id: string, deleteData: boolean): Promise<void> {
  return invoke("uninstall_addon", { id, deleteData });
}

export function restartApp(): Promise<void> {
  return invoke("restart_app");
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

/** `method`: GET (the default) or POST, without a body; `headers` as name/value pairs. */
export function addonFetch(addon: string, url: string, method?: string, headers?: [string, string][]): Promise<ArrayBuffer> {
  return invoke("addon_fetch", { addon, url, method, headers });
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

/** Mirrors `FolderSummary`: a folder the user gave the addon; its path stays in Rust. */
export type AddonFolder = { id: string; name: string };
/** Mirrors `ScannedFile`. */
export type ScannedFile = { path: string; size: number; modified: number };

export function addonFolders(addon: string): Promise<AddonFolder[]> {
  return invoke("addon_folders", { addon });
}

/** Null when the user cancelled. */
export function addonAddFolder(addon: string): Promise<AddonFolder | null> {
  return invoke("addon_add_folder", { addon });
}

export function addonRemoveFolder(addon: string, folder: string): Promise<void> {
  return invoke("addon_remove_folder", { addon, folder });
}

export function addonScanFolder(addon: string, folder: string): Promise<ScannedFile[]> {
  return invoke("addon_scan_folder", { addon, folder });
}

export function addonReadFile(addon: string, folder: string, path: string, offset: number, length: number): Promise<ArrayBuffer> {
  return invoke("addon_read_file", { addon, folder, path, offset, length });
}

/** False when the user cancelled. */
export function addonSaveFile(addon: string, name: string, bytes: Uint8Array): Promise<boolean> {
  return invoke("addon_save_file", { addon, name, bytes: Array.from(bytes) });
}

/** Mirrors `StoredFile`: one of the addon's own files (`storage.putFile`). */
export type StoredFile = { name: string; size: number; modified: number };

/** The bytes go as the raw request body, not JSON (they're saved while reading); the name passed the bridge's
 *  checks in Rust, and both header values are plain ASCII. */
export function addonPutFile(addon: string, name: string, bytes: Uint8Array): Promise<void> {
  return invoke("addon_put_file", bytes, { headers: { addon, name } });
}

/** The file after a leading 1 byte, or empty when there's no such file (so an empty file still differs). */
export function addonGetFile(addon: string, name: string): Promise<ArrayBuffer> {
  return invoke("addon_get_file", { addon, name });
}

export function addonDeleteFile(addon: string, name: string): Promise<void> {
  return invoke("addon_delete_file", { addon, name });
}

export function addonListFiles(addon: string): Promise<StoredFile[]> {
  return invoke("addon_list_files", { addon });
}

export function addonRevealFile(addon: string, folder: string, path: string): Promise<void> {
  return invoke("addon_reveal_file", { addon, folder, path });
}

/** The file's MD5 checksum, lowercase hex. */
export function addonHashFile(addon: string, folder: string, path: string): Promise<string> {
  return invoke("addon_hash_file", { addon, folder, path });
}

/** Downloads `url` to `path` in the folder; resolves to the file's MD5. */
export function addonDownloadFile(addon: string, folder: string, path: string, url: string): Promise<string> {
  return invoke("addon_download_file", { addon, folder, path, url });
}

export function addonSetSearch(addon: string, entries: unknown[]): Promise<void> {
  return invoke("addon_set_search", { addon, entries });
}

export function addonSecretGet(addon: string, name: string): Promise<string | null> {
  return invoke("addon_secret_get", { addon, name });
}

export function addonSecretSet(addon: string, name: string, value: string): Promise<void> {
  return invoke("addon_secret_set", { addon, name, value });
}

export function addonSecretDelete(addon: string, name: string): Promise<void> {
  return invoke("addon_secret_delete", { addon, name });
}

export function addonSetLibraryVisible(addon: string, visible: boolean): Promise<void> {
  return invoke("addon_set_library_visible", { addon, visible });
}

/** Mirrors `CampaignSummary`: an active campaign as an addon sees it. */
export type AddonCampaign = { id: string; name: string };

export function addonCampaigns(addon: string): Promise<AddonCampaign[]> {
  return invoke("addon_campaigns", { addon });
}

export function addonOpenExternal(addon: string, url: string): Promise<void> {
  return invoke("addon_open_external", { addon, url });
}
