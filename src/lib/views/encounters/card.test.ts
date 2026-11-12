import { describe, expect, it } from "vitest";
import type { CatalogRow } from "$lib/api/catalog";
import type { Encounter } from "$lib/api/encounters";
import { encounterCard } from "./card";
import { rowsByKey } from "./roster";

const row = (index: string, xp: number, image?: unknown): CatalogRow => ({
  kind: "monsters",
  pack: "srd",
  source: "SRD",
  index,
  name: index,
  fields: { xp, image },
  others: [],
});

const encounter: Encounter = {
  id: "e1",
  name: "Ambush",
  monsters: [
    { pack: "srd", index: "goblin", name: "goblin", source: "SRD", quantity: 3 },
    { pack: "srd", index: "ogre", name: "ogre", source: "SRD", quantity: 1 },
  ],
  party: [],
  status: "ready",
  partyMode: "custom",
  notes: { description: "", pins: [] },
  savedAt: 0,
};

describe("encounter card", () => {
  it("puts the strongest monster first, counts them all and keeps only real image paths", () => {
    const card = encounterCard(encounter, rowsByKey([row("goblin", 50, 42), row("ogre", 450, "ogre.webp")]), [], []);
    expect(card.rows.map((r) => [r.index, r.image])).toEqual([
      ["ogre", "ogre.webp"],
      ["goblin", null],
    ]);
    expect(card.count).toBe(4);
  });
});
