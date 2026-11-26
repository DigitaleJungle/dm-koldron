// A running encounter: who fights, in which order, whose turn it is, what happens to their hit points, conditions and
// death saves, and the encounter log. Pure functions over `Run` (saved by Rust as the encounter's run file after every
// change).
import type { Character } from "$lib/api/characters";
import type { Combatant, RosterEntry, Run } from "$lib/api/encounters";
import { entryKey, rowKeys } from "./roster";

/**
 * The roster's monsters one by one, each with its group (its row's key: a split group is its own). A monster with
 * more than one in all is numbered on across its groups: Goblin 1–3, then Goblin 4–5. `hp`: the HP the DM set for that
 * monster in Prepare, or null for its stat block's.
 */
export function rosterMonsters(rows: RosterEntry[]): { pack: string; index: string; name: string; group: string; hp: number | null }[] {
  const keys = rowKeys(rows);
  const totals = new Map<string, number>();
  for (const row of rows) totals.set(entryKey(row), (totals.get(entryKey(row)) ?? 0) + row.quantity);
  const numbered = new Map<string, number>();
  return rows.flatMap((row, i) =>
    Array.from({ length: row.quantity }, (_, j) => {
      const n = (numbered.get(entryKey(row)) ?? 0) + 1;
      numbered.set(entryKey(row), n);
      const name = (totals.get(entryKey(row)) ?? 1) > 1 ? `${row.name} ${n}` : row.name;
      return { pack: row.pack, index: row.index, name, group: keys[i], hp: row.hp?.[j] ?? null };
    }),
  );
}

/** Highest initiative first; a tie goes to the higher bonus, then the name. Without initiative, last. */
export function inOrder(combatants: Combatant[]): Combatant[] {
  return [...combatants].sort(
    (a, b) =>
      (b.initiative ?? -Infinity) - (a.initiative ?? -Infinity) ||
      b.initiativeBonus - a.initiativeBonus ||
      a.name.localeCompare(b.name, undefined, { numeric: true }),
  );
}

/** Down: no hit points left (a combatant without a known maximum is never down). */
export const isDown = (c: Combatant) => c.maxHp > 0 && c.hp <= 0;

/** A downed character rolls death saves; anyone else at 0 HP is defeated. */
export const isDying = (c: Combatant) => c.kind === "character" && isDown(c);

/** A defeated monster's (or custom combatant's) turn is skipped; a downed character still gets a turn (death saves). */
const skipped = (c: Combatant) => isDown(c) && !isDying(c);

/** The next (`step` 1) or previous (−1) turn, skipping the defeated; the round changes when the order wraps. */
export function stepTurn(run: Run, step: 1 | -1): Run {
  const n = run.combatants.length;
  if (n === 0) return run;
  let { turn, round } = run;
  for (let k = 0; k < n; k++) {
    const next = (turn + step + n) % n;
    if (step === 1 && next <= turn) round++;
    if (step === -1 && next >= turn) round--;
    turn = next;
    if (!skipped(run.combatants[turn])) break;
  }
  // Nothing before round 1's first turn.
  return round < 1 ? run : { ...run, turn, round };
}

/** Puts a combatant in its place in the order, keeping the turn on whoever has it now. */
export function addCombatant(run: Run, combatant: Combatant): Run {
  const current = run.combatants[run.turn];
  const combatants = inOrder([...run.combatants, combatant]);
  return { ...run, combatants, turn: current ? combatants.indexOf(current) : 0 };
}

/** Changes one combatant (by id). */
export function update(run: Run, id: string, change: (c: Combatant) => Combatant): Run {
  return { ...run, combatants: run.combatants.map((c) => (c.id === id ? change(c) : c)) };
}

// ---- Hit points ----------------------------------------------------------------------------------

/**
 * The fight follows the fields of a character that its addon owns (keeps up to date): HP, max and temp HP and death
 * saves, each only when owned. `ownedOf` gives a character's owned fields. Null when nothing changed.
 */
export function followLinked(run: Run, characters: Character[], ownedOf: (c: Character) => string[]): Run | null {
  let changed = false;
  const combatants = run.combatants.map((c) => {
    const ch = c.kind === "character" ? characters.find((x) => x.id === c.character) : undefined;
    const owned = ch ? ownedOf(ch) : [];
    if (!ch || owned.length === 0) return c;
    const synced: Partial<Combatant> = {};
    if (owned.includes("currentHp")) synced.hp = Math.max(0, ch.currentHp ?? ch.maxHp ?? 0);
    if (owned.includes("maxHp")) synced.maxHp = Math.max(0, ch.maxHp ?? 0);
    if (owned.includes("tempHp")) synced.tempHp = Math.max(0, ch.tempHp ?? 0);
    if (owned.includes("deathSaves")) [synced.deathSuccesses, synced.deathFailures] = [ch.deathSaves?.successes ?? 0, ch.deathSaves?.failures ?? 0];
    if ((Object.keys(synced) as (keyof Combatant)[]).every((k) => c[k] === synced[k])) return c;
    changed = true;
    return { ...c, ...synced };
  });
  return changed ? { ...run, combatants } : null;
}

/** The most of each kind of death save. */
export const DEATH_SAVES = 3;

/**
 * Damage takes temporary hit points first; hit points stop at 0, which ends concentration and starts the death saves
 * afresh. Damage to a character already at 0 counts as a failed death save.
 */
export function damage(c: Combatant, amount: number): Combatant {
  const absorbed = Math.min(c.tempHp, amount);
  const hp = Math.max(0, c.hp - (amount - absorbed));
  const next = { ...c, tempHp: c.tempHp - absorbed, hp, concentrating: c.concentrating && hp > 0 };
  if (hp > 0 || c.kind !== "character" || amount === 0) return next;
  return c.hp > 0 ? { ...next, deathSuccesses: 0, deathFailures: 0 } : { ...next, deathFailures: Math.min(DEATH_SAVES, c.deathFailures + 1) };
}

/** Healing stops at the maximum (when it's known); back above 0, the death saves are cleared. */
export function heal(c: Combatant, amount: number): Combatant {
  const hp = c.maxHp > 0 ? Math.min(c.maxHp, c.hp + amount) : c.hp + amount;
  return hp > 0 && c.hp <= 0 ? { ...c, hp, deathSuccesses: 0, deathFailures: 0 } : { ...c, hp };
}

/** Temporary hit points don't add up: the higher amount stays. */
export const giveTempHp = (c: Combatant, amount: number): Combatant => ({ ...c, tempHp: Math.max(c.tempHp, amount) });

/** Clicking the n-th death save pip: sets the count to n, or to n − 1 when it already is n (to take one back). */
export function setDeathSave(c: Combatant, kind: "deathSuccesses" | "deathFailures", n: number): Combatant {
  return { ...c, [kind]: c[kind] === n ? n - 1 : n };
}

export type Health = "dead" | "stable" | "down" | "defeated" | "bloodied" | null;

/** What the HP line says: dead or stable (3 death saves), down or defeated (0 HP), bloodied (half HP or less). */
export function health(c: Combatant): Health {
  if (isDying(c)) return c.deathFailures >= DEATH_SAVES ? "dead" : c.deathSuccesses >= DEATH_SAVES ? "stable" : "down";
  if (isDown(c)) return "defeated";
  return c.maxHp > 0 && c.hp <= c.maxHp / 2 ? "bloodied" : null;
}

// ---- Conditions ----------------------------------------------------------------------------------

/** Toggles a condition on a combatant: off when it has it, else on. */
export function toggleCondition(c: Combatant, condition: { index: string; name: string }): Combatant {
  const has = c.conditions.some((x) => x.index === condition.index);
  return { ...c, conditions: has ? c.conditions.filter((x) => x.index !== condition.index) : [...c.conditions, condition] };
}

// ---- The log -------------------------------------------------------------------------------------

/** The most entries the log keeps (Rust refuses more); the oldest go first. */
export const MAX_LOG = 1000;
/** The longest entry. */
export const MAX_LOG_TEXT = 500;

/** Adds a log entry, stamped with the round and whose turn it is. */
export function addLog(run: Run, text: string): Run {
  const entry = { round: Math.max(run.round, 1), actor: run.round > 0 ? (run.combatants[run.turn]?.name ?? "") : "", text: text.slice(0, MAX_LOG_TEXT) };
  return { ...run, log: [entry, ...run.log].slice(0, MAX_LOG) };
}

/** The largest HP, AC, initiative or amount typed in a run: far above any real one, and safe for Rust's numbers. */
export const MAX_NUMBER = 99999;

/** A d20 initiative roll in dice notation, e.g. `1d20+2`, `1d20-1`. */
export const initiativeNotation = (bonus: number) => (bonus < 0 ? `1d20${bonus}` : `1d20+${bonus}`);
