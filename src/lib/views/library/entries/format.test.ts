import { describe, expect, it } from "vitest";
import { challenge, components, damages, duration, levelColumns, modifier, monsterHeader, proficiencies, senses, signed, speed, spellHeader } from "./format";

describe("spells", () => {
  it("header, components and duration", () => {
    expect(spellHeader({ level: 3, school: { name: "Evocation" }, classes: [{ name: "Sorcerer" }] })).toBe("Level 3 Evocation");
    expect(spellHeader({ level: 0, school: { name: "Evocation" }, classes: [] })).toBe("Evocation Cantrip");
    expect(components({ components: ["V", "S", "M"], material: "a ball of bat guano" })).toBe("V, S, M (a ball of bat guano)");
    expect(components({ components: ["V"], material: "ignored" })).toBe("V");
    expect(duration({ concentration: true, duration: "up to 1 hour" })).toBe("Concentration, up to 1 hour");
  });
});

describe("monsters", () => {
  it("modifiers with a real minus sign", () => {
    expect([modifier(17), modifier(10), modifier(9), modifier(1)].map(signed)).toEqual(["+3", "+0", "−1", "−5"]);
  });

  it("header, speed, saves and skills", () => {
    expect(monsterHeader({ size: "Large", type: "dragon", alignment: "chaotic evil" })).toBe("Large Dragon, Chaotic evil");
    expect(speed({ fly: "80 ft.", walk: "40 ft.", hover: true })).toBe("40 ft., Fly 80 ft. (hover)");
    const m = { proficiencies: [{ value: 6, proficiency: { name: "Saving Throw: DEX" } }, { value: 13, proficiency: { name: "Skill: Perception" } }] };
    expect(proficiencies(m, "Saving Throw")).toBe("DEX +6");
    expect(proficiencies(m, "Skill")).toBe("Perception +13");
  });

  it("immunities, senses and challenge", () => {
    expect(damages(["fire", "cold"])).toBe("Fire, Cold");
    expect(damages(undefined)).toBe("");
    expect(senses({ darkvision: "120 ft.", passive_perception: 23 })).toBe("Darkvision 120 ft.; Passive Perception 23");
    expect(challenge({ challenge_rating: 0.5, xp: 1800, proficiency_bonus: 2 })).toMatch(/^1\/2 \(XP 1.800|^1\/2 \(XP 1,800|^1\/2 \(XP 1800/);
  });
});

it("class table columns: only values some level has", () => {
  const levels = [
    { class_specific: { rage_count: 2, sneak: 0 }, spellcasting: { spell_slots_level_1: 2, spell_slots_level_9: 0 } },
    { class_specific: { rage_count: 3, sneak: 0, martial: { dice_count: 1, dice_value: 6 } }, spellcasting: { spell_slots_level_1: 3, spell_slots_level_9: 0 } },
  ];
  const cols = levelColumns(levels);
  expect(cols.map((c) => c.label)).toEqual(["Rage Count", "Martial", "Level 1 slots"]);
  expect(cols.map((c) => c.cell(levels[1]))).toEqual(["3", "1d6", "3"]);
  expect(cols[1].cell(levels[0])).toBe("—");
});
