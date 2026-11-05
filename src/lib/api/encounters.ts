import { invoke } from "@tauri-apps/api/core";

/** One roster row: a pack's monster and how many; name and source keep it readable when its pack is off. */
export type RosterEntry = { pack: string; index: string; name: string; source: string; quantity: number };

/** Mirrors `Encounter` in src-tauri/src/commands/encounters.rs. `party`: character ids. `savedAt`: ms. */
export type Encounter = { id: string; name: string; monsters: RosterEntry[]; party: string[]; savedAt: number };

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

/** The budget table of the first switched-on pack that has one; empty when none has. */
export function encounterBudgets(): Promise<Budget[]> {
  return invoke("encounter_budgets");
}
