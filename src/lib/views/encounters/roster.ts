// The encounter being prepared: its roster rows, what the browser knows about each monster, and the party's levels.
import type { CatalogRow } from "$lib/api/catalog";
import type { ClassLevel } from "$lib/api/characters";
import type { RosterEntry } from "$lib/api/encounters";

/** A pack entry's key: the same name in two packs stays two rows. */
export const entryKey = (e: { pack: string; index: string }) => `${e.pack}/${e.index}`;

/** Adds a monster, or one more of it when that pack entry is already there. */
export function addToRoster(roster: RosterEntry[], monster: CatalogRow): RosterEntry[] {
  const key = entryKey(monster);
  const found = roster.find((r) => entryKey(r) === key);
  return found
    ? roster.map((r) => (r === found ? { ...r, quantity: r.quantity + 1 } : r))
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

/** A typed quantity, or null when it isn't a whole number from 1 to 999. */
export function parseQuantity(text: string): number | null {
  const n = Number(text.trim());
  return text.trim() !== "" && Number.isInteger(n) && n >= 1 && n <= 999 ? n : null;
}

/** Total character level (the sum of class levels), or null when it's missing or invalid. */
export function characterLevel(classes: ClassLevel[] | undefined): number | null {
  const total = (classes ?? []).reduce((sum, c) => sum + c.level, 0);
  return Number.isInteger(total) && total >= 1 ? total : null;
}
