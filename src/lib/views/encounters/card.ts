// What an encounter card shows (the Encounters screen, and a scene's links in session prep): the monsters strongest
// first (for the collage), how many, difficulty, XP and the party.
import type { CatalogRow } from "$lib/api/catalog";
import type { Character } from "$lib/api/characters";
import type { Budget, Encounter, RosterEntry } from "$lib/api/encounters";
import { strings } from "$lib/strings";
import { difficulty, monsterXp, type Band } from "./difficulty";
import { characterLevel, encounterParty, entryKey, rowKeys } from "./roster";

export type CardRow = RosterEntry & { key: string; image: string | null };
export type EncounterCardData = Encounter & { count: number; band: Band | null; xp: number; partyLine: string; rows: CardRow[] };

const t = strings.encounters;

function partyText(ids: string[], characterById: Map<string, Character>) {
  if (ids.length === 0) return t.list.noParty;
  const levels = ids.map((id) => characterLevel(characterById.get(id)?.classes));
  const known = levels.every((l) => l !== null) ? (levels as number[]) : null;
  return t.list.party(ids.length, known && Math.min(...known), known && Math.max(...known));
}

/** `byKey`: the monsters' catalog rows (with `xp`, and `image` for pictures); `characters`: null when not loaded. */
export function encounterCard(e: Encounter, byKey: Map<string, CatalogRow>, characters: Character[] | null, budgets: Budget[]): EncounterCardData {
  const characterById = new Map((characters ?? []).map((c) => [c.id, c]));
  const party = encounterParty(e.partyMode, e.party, characters); // as in Prepare
  const xps = e.monsters.map((m) => monsterXp(byKey.get(entryKey(m))?.fields.xp));
  const levels = party.map((id) => characterLevel(characterById.get(id)?.classes));
  const result = difficulty(levels, e.monsters.map((m, i) => ({ quantity: m.quantity, xp: xps[i] })), budgets);
  const keys = rowKeys(e.monsters); // a split group is its own hexagon
  const rows = e.monsters
    .map((m, i) => {
      const image = byKey.get(entryKey(m))?.fields.image;
      return { row: { ...m, key: keys[i], image: typeof image === "string" ? image : null }, xp: xps[i] ?? 0 };
    })
    .sort((a, b) => b.xp - a.xp) // the strongest goes in the middle, the rest outward
    .map(({ row }) => row);
  return {
    ...e,
    count: e.monsters.reduce((n, m) => n + m.quantity, 0),
    band: result.band,
    xp: result.total,
    partyLine: partyText(party, characterById),
    rows,
  };
}
