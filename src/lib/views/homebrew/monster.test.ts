import { describe, expect, it } from "vitest";
import { attackText, averageHp, damageText, detectLevel, hpFormula, parseHitDice, splitList, usualFor } from "./monster";

describe("monster form values", () => {
  it("tells proficient, expert and custom bonuses apart", () => {
    expect(detectLevel(5, 3, 2)).toBe("prof");
    expect(detectLevel(7, 3, 2)).toBe("expert");
    expect(detectLevel(4, 3, 2)).toBe("custom");
    expect(detectLevel(5, 3, undefined)).toBe("custom");
  });

  it("reads hit dice and averages them with CON", () => {
    expect(parseHitDice("5d8")).toEqual({ count: 5, die: 8, rest: "" });
    expect(parseHitDice("20d10 + 40")).toEqual({ count: 20, die: 10, rest: " + 40" });
    expect(parseHitDice("lots")).toBeNull();
    expect(averageHp(5, 8, 2)).toBe(32);
    expect(averageHp(1, 4, -5)).toBe(1);
    expect(hpFormula(5, 8, 2)).toBe("5d8 + 10 = 32");
    expect(hpFormula(2, 6, -1)).toBe("2d6 − 2 = 5");
  });

  it("looks up the usual XP and PB of a challenge rating", () => {
    const rules = { challenges: [{ cr: 0.25, xp: 50, pb: 2 }], proficiencyAbilities: {} };
    expect(usualFor(rules, 0.25)).toEqual({ cr: 0.25, xp: 50, pb: 2 });
    expect(usualFor(rules, 18)).toBeUndefined();
    expect(usualFor(rules, undefined)).toBeUndefined();
    expect(usualFor(null, 0.25)).toBeUndefined();
  });

  it("writes an attack's text", () => {
    expect(damageText("1d6", 1)).toBe("4 (1d6 + 1)");
    expect(damageText("2d6", -1)).toBe("6 (2d6 − 1)");
    expect(damageText("7", 2)).toBe("7");
    const attack = { kind: "melee" as const, ability: "strength", reach: "reach 5 ft.", dice: "1d6", type: "bludgeoning", extra: "" };
    expect(attackText(attack, 1, 2, "Bludgeoning")).toBe("Melee Attack Roll: +3, reach 5 ft. Hit: 4 (1d6 + 1) Bludgeoning damage.");
    expect(attackText({ ...attack, kind: "ranged", extra: "The target is pushed." }, -1, 2, "Piercing")).toBe(
      "Ranged Attack Roll: +1, reach 5 ft. Hit: 2 (1d6 − 1) Piercing damage. The target is pushed.",
    );
  });

  it("splits a text list", () => {
    expect(splitList("Common, Deep Speech; telepathy 120 ft.")).toEqual(["Common", "Deep Speech; telepathy 120 ft."]);
    expect(splitList(undefined)).toEqual([]);
  });
});
