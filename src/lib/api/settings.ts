import { invoke } from "@tauri-apps/api/core";

/** Mirrors `AppSettings` in src-tauri/src/commands/settings.rs. */
export type Theme = "system" | "light" | "dark";
export type Animations = "system" | "on" | "off";
/** `disabledPacks`: pack ids switched off in Settings → Rules. */
export type AppSettings = { theme: Theme; animations: Animations; disabledPacks: string[] };

// Both reject with a StorageError.
export function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}

/** Saves only the given fields and returns the full, saved settings. */
export function setSettings(changes: Partial<AppSettings>): Promise<AppSettings> {
  return invoke("set_settings", { settings: changes });
}
