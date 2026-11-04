import { describe, expect, it } from "vitest";
import type { CatalogRow } from "$lib/api/catalog";
import { categoryById, crText, filterOptions, rarities, selectRows } from "./catalog";

const row = (name: string, fields: Record<string, any>, source = "SRD 5.2.1", kind = "spells"): CatalogRow => ({ kind, pack: "srd", source, index: name.toLowerCase(), name, fields });
const spells = categoryById("spells");
const rows = [
  row("Fireball", { level: 3, school: { name: "Evocation" }, classes: [{ name: "Wizard" }, { name: "Sorcerer" }] }),
  row("Fire Bolt", { level: 0, school: { name: "Evocation" }, classes: [{ name: "Wizard" }] }),
  row("Bless", { level: 1, school: { name: "Enchantment" }, concentration: true, classes: [{ name: "Cleric" }] }, "Homebrew"),
];
const query = { search: "", filters: {}, sort: "name", desc: false };
const names = (list: CatalogRow[]) => list.map((r) => r.name);

describe("selectRows", () => {
  it("needs every search word in the name", () => {
    expect(names(selectRows(rows, spells, { ...query, search: "fire b" }))).toEqual(["Fire Bolt", "Fireball"]);
    expect(names(selectRows(rows, spells, { ...query, search: "FIRE bolt" }))).toEqual(["Fire Bolt"]);
  });

  it("ORs options within a filter and ANDs filters", () => {
    expect(names(selectRows(rows, spells, { ...query, filters: { level: ["Cantrip", "1"] } }))).toEqual(["Bless", "Fire Bolt"]);
    expect(names(selectRows(rows, spells, { ...query, filters: { school: ["Evocation"], class: ["Sorcerer"] } }))).toEqual(["Fireball"]);
    expect(names(selectRows(rows, spells, { ...query, filters: { other: ["Concentration"] } }))).toEqual(["Bless"]);
  });

  it("sorts numbers as numbers, both ways, ties by name; and by source", () => {
    expect(names(selectRows(rows, spells, { ...query, sort: "level" }))).toEqual(["Fire Bolt", "Bless", "Fireball"]);
    expect(names(selectRows(rows, spells, { ...query, sort: "level", desc: true }))).toEqual(["Fireball", "Bless", "Fire Bolt"]);
    expect(names(selectRows(rows, spells, { ...query, sort: "school" }))).toEqual(["Bless", "Fire Bolt", "Fireball"]);
    expect(names(selectRows(rows, spells, { ...query, sort: "source" }))).toEqual(["Bless", "Fire Bolt", "Fireball"]);
  });
});

describe("filterOptions", () => {
  it("lists the values present, in the filter's order", () => {
    const level = spells.filters.find((f) => f.id === "level")!;
    expect(filterOptions(rows, level)).toEqual(["Cantrip", "1", "3"]);
    const cls = spells.filters.find((f) => f.id === "class")!;
    expect(filterOptions(rows, cls)).toEqual(["Cleric", "Sorcerer", "Wizard"]);
  });

  it("monster CR bands and sizes", () => {
    const monsters = categoryById("monsters");
    const m = [row("Rat", { challenge_rating: 0.125, size: "Tiny" }), row("Ogre", { challenge_rating: 2, size: "Large" }), row("Mage", { challenge_rating: 6, size: "Medium or small" })];
    expect(filterOptions(m, monsters.filters.find((f) => f.id === "cr")!)).toEqual(["0–½", "1–4", "5–10"]);
    expect(filterOptions(m, monsters.filters.find((f) => f.id === "size")!)).toEqual(["Tiny", "Small", "Medium", "Large"]);
    expect(names(selectRows(m, monsters, { ...query, sort: "cr", desc: true }))).toEqual(["Mage", "Ogre", "Rat"]);
  });
});

it("reads CR fractions and magic item rarities", () => {
  expect([crText(0.125), crText(0.5), crText(10), crText(undefined)]).toEqual(["1/8", "1/2", "10", "—"]);
  expect(rarities("Uncommon (+1), Rare (+2), or Very Rare (+3)")).toEqual(["Uncommon", "Rare", "Very Rare"]);
  expect(rarities("Rare (Silver or Brass), Very Rare (Bronze), or Legendary (Iron)")).toEqual(["Rare", "Very Rare", "Legendary"]);
  expect(rarities("Rarity Varies")).toEqual(["Varies"]);
  expect(rarities(undefined)).toEqual([]);
});

it("falls back to the first category, All", () => {
  expect(categoryById("nope").id).toBe("all");
});

it("All lists every type with its category, filterable by it", () => {
  const all = categoryById("all");
  const mixed = [row("Fireball", {}), row("Ogre", {}, "SRD 5.2.1", "monsters"), row("Blinded", {}, "SRD 5.2.1", "conditions")];
  expect(all.columns[0].value(mixed[1])).toBe("Monsters");
  expect(filterOptions(mixed, all.filters[0])).toEqual(["Spells", "Monsters", "Conditions"]);
  expect(names(selectRows(mixed, all, { ...query, filters: { category: ["Monsters", "Conditions"] } }))).toEqual(["Blinded", "Ogre"]);
});
