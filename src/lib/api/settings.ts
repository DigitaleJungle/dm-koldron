import { invoke } from "@tauri-apps/api/core";

/** Mirrors `AppSettings` in src-tauri/src/commands/settings.rs. */
export type Theme = "system" | "light" | "dark";
export type Animations = "system" | "on" | "off";
/** `disabledPacks`: pack ids switched off in Settings → Rules; `imagePacks`: pack ids with Enable images ticked;
 *  `libraryTabs`: the Library's tab ids in the order set in Settings → UI. */
export type AppSettings = { theme: Theme; animations: Animations; disabledPacks: string[]; imagePacks: string[]; libraryTabs: string[] };

// Both reject with a StorageError.
export function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}

/** Saves only the given fields and returns the full, saved settings. */
export function setSettings(changes: Partial<AppSettings>): Promise<AppSettings> {
  return invoke("set_settings", { settings: changes });
}
