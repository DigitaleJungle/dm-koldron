import { describe, expect, it } from "vitest";
import type { Budget } from "$lib/api/encounters";
import { difficulty, monsterXp } from "./difficulty";

const table: Budget[] = [
  { level: 1, low: 50, moderate: 75, high: 100 },
  { level: 3, low: 150, moderate: 225, high: 400 },
];
const goblins = (n: number) => [{ quantity: n, xp: 50 }];

describe("monsterXp", () => {
  it("takes whole XP of zero or more and leaves anything else unknown", () => {
    expect([monsterXp(0), monsterXp(450), monsterXp(-5), monsterXp(1.5), monsterXp("50"), monsterXp(undefined)]).toEqual([0, 450, null, null, null, null]);
  });
});

describe("difficulty", () => {
  const party = [1, 1, 1, 1];
  it("reads the acceptance examples with no multiplier", () => {
    expect(difficulty(party, goblins(3), table)).toMatchObject({ budgets: [200, 300, 400], total: 150, band: "easy" });
    expect(difficulty(party, goblins(6), table)).toMatchObject({ total: 300, band: "medium" });
    expect(difficulty(party, goblins(8), table)).toMatchObject({ total: 400, band: "hard" });
    expect(difficulty(party, goblins(9), table)).toMatchObject({ total: 450, band: "deadly" });
  });
  it("puts exact budgets in the lower band", () => {
    expect(difficulty(party, [{ quantity: 1, xp: 200 }], table).band).toBe("easy");
    expect(difficulty(party, [{ quantity: 1, xp: 201 }], table).band).toBe("medium");
    // Trivial ends at half of Low.
    expect(difficulty(party, [{ quantity: 1, xp: 100 }], table).band).toBe("trivial");
    expect(difficulty(party, [{ quantity: 1, xp: 101 }], table).band).toBe("easy");
  });
  it("sums a mixed-level party and recalculates when a character leaves", () => {
    expect(difficulty([1, 3], [], table).budgets).toEqual([200, 300, 500]);
    expect(difficulty([1, 3], goblins(5), table).band).toBe("medium");
    expect(difficulty([1], goblins(5), table).band).toBe("deadly");
  });
  it("gives no rating while something is missing", () => {
    expect(difficulty([1, null], goblins(1), table)).toMatchObject({ missingLevels: 1, complete: false, band: null });
    expect(difficulty([1, 2], goblins(1), table)).toMatchObject({ missingLevels: 1, band: null });
    expect(difficulty([1], [{ quantity: 2, xp: null }, ...goblins(1)], table)).toMatchObject({ total: 50, unknownXp: 1, band: null });
    expect(difficulty([1], goblins(1), [])).toMatchObject({ complete: false, band: null });
  });
  it("gives no rating without a party or monsters", () => {
    expect(difficulty([], goblins(1), table).band).toBeNull();
    expect(difficulty(party, [], table)).toMatchObject({ total: 0, band: null });
  });
});
