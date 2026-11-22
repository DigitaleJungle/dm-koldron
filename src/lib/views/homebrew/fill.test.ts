import { describe, expect, it } from "vitest";
import { fill } from "./fill";

describe("filling an open form", () => {
  it("fills what's empty and keeps what was typed", () => {
    const draft: Record<string, unknown> = { index: "owl", name: "My Owlbear", hit_points: 60, speed: {} };
    fill(draft, { index: "x", name: "Owlbear", hit_points: 59, armor_class: [{ value: 13 }], speed: { walk: "40 ft." } });
    expect(draft).toEqual({ index: "owl", name: "My Owlbear", hit_points: 60, armor_class: [{ value: 13 }], speed: { walk: "40 ft." } });
  });

  it("grows a text the new one continues, and adds list items it doesn't have", () => {
    const draft: Record<string, unknown> = {
      description: "A bright streak",
      actions: [{ name: "Bite", desc: "Melee Attack Roll: +7" }],
      proficiencies: [{ value: 3, proficiency: { index: "skill-perception" } }],
      desc: ["Wondrous Item", "This bag has"],
    };
    fill(draft, {
      description: "A bright streak flashes.",
      actions: [
        { name: "bite", desc: "Melee Attack Roll: +7, reach 5 ft." },
        { name: "Claws", desc: "Melee Attack Roll: +7" },
      ],
      proficiencies: [{ value: 5, proficiency: { index: "skill-perception" } }, { value: 2, proficiency: { index: "skill-stealth" } }],
      desc: ["Wondrous Item", "This bag has room.", "It weighs 15 pounds."],
    });
    expect(draft).toEqual({
      description: "A bright streak flashes.",
      actions: [
        { name: "Bite", desc: "Melee Attack Roll: +7, reach 5 ft." },
        { name: "Claws", desc: "Melee Attack Roll: +7" },
      ],
      proficiencies: [{ value: 3, proficiency: { index: "skill-perception" } }, { value: 2, proficiency: { index: "skill-stealth" } }],
      desc: ["Wondrous Item", "This bag has room.", "It weighs 15 pounds."],
    });
  });

  it("knows a saved section by its name, so it isn't added again", () => {
    const draft: Record<string, unknown> = { sections: [{ id: "s1", index: "main-hall", name: "Main Hall", desc: "A hall" }] };
    fill(draft, { sections: [{ name: "Main Hall", desc: "A hall with pillars." }, { name: "Crypt", desc: "Cold." }] });
    expect(draft.sections).toEqual([
      { id: "s1", index: "main-hall", name: "Main Hall", desc: "A hall with pillars." },
      { name: "Crypt", desc: "Cold." },
    ]);
  });

  it("keeps a text that was changed, rather than adding the new one to it", () => {
    const draft: Record<string, unknown> = { description: "My own words." };
    fill(draft, { description: "A bright streak flashes." });
    expect(draft.description).toBe("My own words.");
  });
});
