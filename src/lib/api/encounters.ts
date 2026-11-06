import { invoke } from "@tauri-apps/api/core";
import type { Picture } from "$lib/api/campaigns";

/** One roster row: a pack's monster and how many; name and source keep it readable when its pack is off. */
export type RosterEntry = { pack: string; index: string; name: string; source: string; quantity: number };

/** Draft while being prepared; Ready is locked in Prepare and can be run. */
export type EncounterStatus = "draft" | "ready";

/** A pin on the map: x and y from 0 to 1 across and down the picture. */
export type Pin = { x: number; y: number; title: string; text: string };

/** The DM's notes on an encounter: the Notes text (stored as `description`) and the map's pins. */
export type Notes = { description: string; pins: Pin[] };

/** Mirrors `Encounter` in src-tauri/src/commands/encounters.rs. `party`: character ids. `savedAt`: ms.
 *  `map`: the map picture beside the file, set by Rust only (ignored when saving). */
export type Encounter = {
  id: string;
  name: string;
  monsters: RosterEntry[];
  party: string[];
  status: EncounterStatus;
  notes: Notes;
  map?: Picture | null;
  savedAt: number;
};

/** Most recently saved first; `unreadable`: files that couldn't be read, left untouched. */
export type EncounterList = { encounters: Encounter[]; unreadable: string[] };

/** XP per character of one level for the three 2024 difficulties, from the packs' `encounter-budgets`. */
export type Budget = { level: number; low: number; moderate: number; high: number };

// All of these reject with a StorageError.
export function listEncounters(campaign: string): Promise<EncounterList> {
  return invoke("list_encounters", { campaign });
}

export function getEncounter(campaign: string, id: string): Promise<Encounter> {
  return invoke("get_encounter", { campaign, id });
}

/** Creates the encounter when its `id` is empty, else replaces it. Returns it as saved. */
export function saveEncounter(campaign: string, encounter: Encounter): Promise<Encounter> {
  return invoke("save_encounter", { campaign, encounter });
}

/** Makes the picture file at `path` (from `choosePictureFile`) a saved encounter's map, or removes it (`null`). */
export function setEncounterMap(campaign: string, id: string, path: string | null): Promise<Picture | null> {
  return invoke("set_encounter_map", { campaign, id, path });
}

/** The budget table of the first switched-on pack that has one; empty when none has. */
export function encounterBudgets(): Promise<Budget[]> {
  return invoke("encounter_budgets");
}
