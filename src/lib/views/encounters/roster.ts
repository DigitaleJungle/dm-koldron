// The encounter being prepared: its roster rows, what the browser knows about each monster, and the party's levels.
import type { CatalogRow } from "$lib/api/catalog";
import type { Character, ClassLevel } from "$lib/api/characters";
import type { PartyMode, RosterEntry } from "$lib/api/encounters";

/** A pack entry's key: the same name in two packs stays two rows. */
export const entryKey = (e: { pack: string; index: string }) => `${e.pack}/${e.index}`;

/**
 * Each roster row's own key: its entry key, then "#2", "#3"… for later rows of the same monster (a split group).
 * shortcut: removing an earlier row of a monster shifts the later ones' keys, so their tiles animate as if swapped;
 * give rows a stored id if that ever matters.
 */
export function rowKeys(roster: { pack: string; index: string }[]): string[] {
  const seen = new Map<string, number>();
  return roster.map((r) => {
    const key = entryKey(r);
    const n = (seen.get(key) ?? 0) + 1;
    seen.set(key, n);
    return n === 1 ? key : `${key}#${n}`;
  });
}

/** Splits the row at `i` (×2 or more) into two groups beside each other, the odd one in the first: ×5 → ×3 + ×2. */
export function splitRow(roster: RosterEntry[], i: number): RosterEntry[] {
  const row = roster[i];
  if (!row || row.quantity < 2) return roster;
  const first = row.quantity - Math.floor(row.quantity / 2);
  const hp = row.hp ?? [];
  return [
    ...roster.slice(0, i),
    { ...row, quantity: first, hp: hp.slice(0, first) },
    { ...row, quantity: row.quantity - first, hp: hp.slice(first) },
    ...roster.slice(i + 1),
  ];
}

/** The row with `quantity` monsters; HP set for monsters no longer there goes. */
export const withQuantity = (row: RosterEntry, quantity: number): RosterEntry => ({ ...row, quantity, hp: (row.hp ?? []).slice(0, quantity) });

/** The row with the `n`th monster's HP set, or back to its stat block's (null). */
export function withHp(row: RosterEntry, n: number, hp: number | null): RosterEntry {
  const list = Array.from({ length: row.quantity }, (_, i) => (i === n ? hp : (row.hp?.[i] ?? null)));
  while (list.length && list[list.length - 1] === null) list.pop(); // untouched HP isn't stored
  return { ...row, hp: list };
}

/** What a monster's HP is rolled with: "20d10 + 40" (`hit_points_roll`, with its CON), else its hit dice; "" for none. */
export const hpDice = (m: Record<string, any> | undefined): string =>
  typeof m?.hit_points_roll === "string" ? m.hit_points_roll : typeof m?.hit_dice === "string" ? m.hit_dice : "";

/** The row as another pack's version of its monster: the same count, the stat block's HP. */
export const asVersion = (row: RosterEntry, v: CatalogRow): RosterEntry => ({ ...row, pack: v.pack, index: v.index, name: v.name, source: v.source, hp: [] });

/** The most of one monster in a roster; Rust refuses to save more. */
export const MAX_QUANTITY = 999;

/** Adds a monster, or one more of it (up to MAX_QUANTITY) when that pack entry is already there. */
export function addToRoster(roster: RosterEntry[], monster: CatalogRow): RosterEntry[] {
  const key = entryKey(monster);
  const found = roster.find((r) => entryKey(r) === key);
  return found
    ? roster.map((r) => (r === found ? withQuantity(r, Math.min(r.quantity + 1, MAX_QUANTITY)) : r))
    : [...roster, { pack: monster.pack, index: monster.index, name: monster.name, source: monster.source, quantity: 1 }];
}

/** A browser row's versions: its own entry, then the same monster in each other pack it bundles, with its own fields. */
export function versions(row: CatalogRow): CatalogRow[] {
  return [row, ...row.others.map((o) => ({ ...row, pack: o.pack, source: o.source, index: o.index, name: o.name, fields: o.fields, others: [] }))];
}

/**
 * Every version of the browser's rows by entry key (the same monster in another pack is its own version), so a saved
 * row finds its CR and XP. A row whose pack is off finds nothing: its XP shows as unknown.
 */
export function rowsByKey(rows: CatalogRow[]): Map<string, CatalogRow> {
  return new Map(rows.flatMap(versions).map((v) => [entryKey(v), v]));
}

/** Every version's key to all the versions of its monster (one for a monster in one pack). */
export function versionsByKey(rows: CatalogRow[]): Map<string, CatalogRow[]> {
  return new Map(rows.flatMap((r) => versions(r).map((v, _, all) => [entryKey(v), all] as const)));
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
