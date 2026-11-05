// Encounter difficulty by the 2024 rules: per-character XP budgets against total monster XP, with no multipliers.
// The budgets come from the packs' `encounter-budgets` table and each monster's XP from its own entry.
import type { Budget } from "$lib/api/encounters";

/** Bands from the budgets: Trivial up to half of Low, Easy up to Low, Medium up to Moderate, Hard up to High, then Deadly. */
export const BANDS = ["trivial", "easy", "medium", "hard", "deadly"] as const;
export type Band = (typeof BANDS)[number];

/** The upper edge of each band but Deadly: half of Low, Low, Moderate, High. */
export const bandEdges = (budgets: [number, number, number]) => [budgets[0] / 2, ...budgets];

export type Difficulty = {
  /** Summed [Low, Moderate, High] budgets of the characters with a known level. */
  budgets: [number, number, number];
  total: number;
  missingLevels: number;
  unknownXp: number;
  complete: boolean;
  /** Only when there's a party, monster XP and nothing missing. */
  band: Band | null;
};

/** A monster entry's XP, or null when it has none usable; unknown XP never becomes zero. */
export const monsterXp = (xp: unknown): number | null => (typeof xp === "number" && Number.isInteger(xp) && xp >= 0 ? xp : null);

/** `levels` per selected character (null when unknown), `monsters` per roster row (xp null when unknown). */
export function difficulty(levels: (number | null)[], monsters: { quantity: number; xp: number | null }[], table: Budget[]): Difficulty {
  const budgets: [number, number, number] = [0, 0, 0];
  let missingLevels = 0;
  for (const level of levels) {
    const row = table.find((b) => b.level === level);
    if (!row) missingLevels++;
    else [row.low, row.moderate, row.high].forEach((xp, i) => (budgets[i] += xp));
  }
  const total = monsters.reduce((sum, m) => sum + (m.xp ?? 0) * m.quantity, 0);
  const unknownXp = monsters.filter((m) => m.xp === null).length;
  const complete = table.length > 0 && missingLevels === 0 && unknownXp === 0;
  const edge = bandEdges(budgets).findIndex((xp) => total <= xp);
  const band: Band | null = !complete || levels.length === 0 || total === 0 ? null : BANDS[edge === -1 ? 4 : edge];
  return { budgets, total, missingLevels, unknownXp, complete, band };
}
