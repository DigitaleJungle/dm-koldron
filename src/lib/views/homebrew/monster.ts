// The monster form's worked-out values: proficiency levels, the average hit points of its hit dice, the usual XP and
// proficiency bonus of a challenge rating (from the packs, `homebrewMonsterRules`), and an attack's text.
import type { MonsterRules } from "$lib/api/homebrew";
import { strings } from "$lib/strings";
import { signed } from "$lib/views/library/entries/format";

/** How a saving throw or skill bonus is made: the proficiency bonus once, twice, or a value of its own. */
export type Level = "prof" | "expert" | "custom";

export const bonusFor = (level: Exclude<Level, "custom">, mod: number, pb: number) => mod + (level === "expert" ? 2 : 1) * pb;

/** The level a stored bonus has: proficient or expert when it's that sum, else its own value. */
export function detectLevel(value: number, mod: number, pb: number | undefined): Level {
  if (pb === undefined) return "custom";
  if (value === bonusFor("prof", mod, pb)) return "prof";
  if (value === bonusFor("expert", mod, pb)) return "expert";
  return "custom";
}

/** "5d8", "5d8 + 10": the count, the die, and what follows (kept when the count or die changes). */
export function parseHitDice(value: unknown): { count: number; die: number; rest: string } | null {
  const found = typeof value === "string" ? /^\s*(\d+)\s*d\s*(\d+)(.*)$/i.exec(value) : null;
  return found ? { count: Number(found[1]), die: Number(found[2]), rest: found[3] } : null;
}

/** The average roll of `count` dice of `die` sides, plus CON's modifier per die (at least 1). */
export const averageHp = (count: number, die: number, conMod: number) => Math.max(1, Math.floor((count * (die + 1)) / 2) + count * conMod);

/** " + 10", " − 5", or nothing. */
const plus = (n: number) => (n === 0 ? "" : n > 0 ? ` + ${n}` : ` − ${Math.abs(n)}`);

/** "5d8 + 10 = 32". */
export const hpFormula = (count: number, die: number, conMod: number) => `${count}d${die}${plus(count * conMod)} = ${averageHp(count, die, conMod)}`;

/** The XP and proficiency bonus the packs' monsters with this challenge rating usually have. */
export function usualFor(rules: MonsterRules | null, cr: unknown): { xp: number | null; pb: number | null } | undefined {
  return typeof cr === "number" ? rules?.challenges.find((c) => c.cr === cr) : undefined;
}

/** A text list ("Common, Elvish") as its items, and back. */
export const splitList = (value: unknown) => (typeof value === "string" ? value.split(",").map((s) => s.trim()).filter(Boolean) : []);
export const joinList = (items: string[]) => items.join(", ");

/** An attack as the attack builder edits it; it writes the item's text. `ability` is the monster field (`strength`). */
export type Attack = { kind: "melee" | "ranged" | "both"; ability: string; reach: string; dice: string; type: string; extra: string };

/** "4 (1d6 + 1)" from "1d6" with +1; dice it can't read stay as typed. */
export function damageText(dice: string, mod: number): string {
  const found = /^\s*(\d+)\s*d\s*(\d+)\s*$/i.exec(dice);
  if (!found) return dice.trim();
  const [n, d] = [Number(found[1]), Number(found[2])];
  return `${Math.max(1, Math.floor((n * (d + 1)) / 2) + mod)} (${n}d${d}${plus(mod)})`;
}

/** The text of an attack: "Melee Attack Roll: +3, reach 5 ft. Hit: 4 (1d6 + 1) Bludgeoning damage." */
export function attackText(a: Attack, mod: number, pb: number | undefined, typeName: string): string {
  const m = strings.homebrew.monster;
  const line = m.attackText(m.attackRoll[a.kind], signed(mod + (pb ?? 0)), a.reach.trim(), damageText(a.dice, mod), typeName);
  return a.extra.trim() ? `${line} ${a.extra.trim()}` : line;
}
