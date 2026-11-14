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

export type PartyMode = "party" | "custom";

/** Mirrors `Encounter` in src-tauri/src/commands/encounters.rs. `party`: character ids. `savedAt`: ms.
 *  `map`: the map picture beside the file, set by Rust only (ignored when saving). */
export type Encounter = {
  id: string;
  name: string;
  monsters: RosterEntry[];
  party: string[];
  status: EncounterStatus;
  /** "party": whoever is in the campaign's party now; "custom": `party`. Older files read as "custom". */
  partyMode: PartyMode;
  notes: Notes;
  map?: Picture | null;
  savedAt: number;
};

/** Most recently saved first; `unreadable`: files that couldn't be read, left untouched. */
export type EncounterList = { encounters: Encounter[]; unreadable: string[] };

/** XP per character of one level for the three 2024 difficulties, from the packs' `encounter-budgets`. */
export type Budget = { level: number; low: number; moderate: number; high: number };

/** A condition on a combatant, from a pack's conditions. */
export type Condition = { index: string; name: string };

/** One line of the encounter log: the round, whose turn it was, and what happened. */
export type LogEntry = { round: number; actor: string; text: string };

export type CombatantKind = "monster" | "character" | "custom";

/** Mirrors `Combatant` in src-tauri/src/commands/encounters.rs. A monster has `pack` and `index`, a character
 *  `character` (its id); `group`: monsters sharing one initiative roll. HP are whole numbers ≥ 0; `maxHp` 0: unknown. */
export type Combatant = {
  id: string;
  kind: CombatantKind;
  pack: string;
  index: string;
  character: string;
  group: string;
  name: string;
  initiative: number | null;
  initiativeBonus: number;
  hp: number;
  maxHp: number;
  tempHp: number;
  ac: number | null;
  concentrating: boolean;
  conditions: Condition[];
  /** A downed character's death saving throws, 0 to 3 each. */
  deathSuccesses: number;
  deathFailures: number;
};

/** Mirrors `Run`: a running encounter, saved beside it as it changes. `round` 0: rolling initiative, before Start.
 *  `turn`: index into `combatants`, which are kept in initiative order. `log`: newest first. `savedAt`: ms, set by Rust. */
export type Run = { round: number; turn: number; combatants: Combatant[]; log: LogEntry[]; savedAt: number };

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

/** Deletes a saved encounter with its map and run, and unlinks it from the sessions' scenes. */
export function deleteEncounter(campaign: string, id: string): Promise<void> {
  return invoke("delete_encounter", { campaign, id });
}

/** Makes the picture file at `path` (from `choosePictureFile`) a saved encounter's map, or removes it (`null`). */
export function setEncounterMap(campaign: string, id: string, path: string | null): Promise<Picture | null> {
  return invoke("set_encounter_map", { campaign, id, path });
}

/** The encounter's run, or null when it isn't running. */
export function getRun(campaign: string, encounter: string): Promise<Run | null> {
  return invoke("get_run", { campaign, encounter });
}

/** Saves the run (atomically, after checking it). Returns it as saved. */
export function saveRun(campaign: string, encounter: string, run: Run): Promise<Run> {
  return invoke("save_run", { campaign, encounter, run });
}

/** Ends the run: removes its file. */
export function endRun(campaign: string, encounter: string): Promise<void> {
  return invoke("end_run", { campaign, encounter });
}

/** The budget table of the first switched-on pack that has one; empty when none has. */
export function encounterBudgets(): Promise<Budget[]> {
  return invoke("encounter_budgets");
}
