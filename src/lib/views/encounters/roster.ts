// The encounter being prepared: its roster rows, what the browser knows about each monster, and the party's levels.
import type { CatalogRow } from "$lib/api/catalog";
import type { Character, ClassLevel } from "$lib/api/characters";
import type { PartyMode, RosterEntry } from "$lib/api/encounters";

/** A pack entry's key: the same name in two packs stays two rows. */
export const entryKey = (e: { pack: string; index: string }) => `${e.pack}/${e.index}`;

/** The most of one monster in a roster; Rust refuses to save more. */
export const MAX_QUANTITY = 999;

/** Adds a monster, or one more of it (up to MAX_QUANTITY) when that pack entry is already there. */
export function addToRoster(roster: RosterEntry[], monster: CatalogRow): RosterEntry[] {
  const key = entryKey(monster);
  const found = roster.find((r) => entryKey(r) === key);
  return found
    ? roster.map((r) => (r === found ? { ...r, quantity: Math.min(r.quantity + 1, MAX_QUANTITY) } : r))
    : [...roster, { pack: monster.pack, index: monster.index, name: monster.name, source: monster.source, quantity: 1 }];
}

/**
 * The browser's rows by entry key, also under the keys of the packs a row bundles (the same monster in another pack),
 * so a saved row finds its CR and XP. A row whose pack is off finds nothing: its XP shows as unknown.
 */
export function rowsByKey(rows: CatalogRow[]): Map<string, CatalogRow> {
  const map = new Map<string, CatalogRow>();
  for (const row of rows) {
    map.set(entryKey(row), row);
    for (const other of row.others) map.set(entryKey(other), row);
  }
  return map;
}

/**
 * The ids of who fights: in Party mode the campaign's party as it is now (none until the characters are known), else
 * the encounter's own pick, leaving out deleted characters once the characters are known.
 */
export function encounterParty(mode: PartyMode, pick: string[], characters: Character[] | null): string[] {
  if (mode === "party") return (characters ?? []).filter((c) => c.onHome).map((c) => c.id);
  return characters ? pick.filter((id) => characters.some((c) => c.id === id)) : pick;
}

/** Total character level (the sum of class levels), or null when it's missing or invalid. */
export function characterLevel(classes: ClassLevel[] | undefined): number | null {
  const total = (classes ?? []).reduce((sum, c) => sum + c.level, 0);
  return Number.isInteger(total) && total >= 1 ? total : null;
}
