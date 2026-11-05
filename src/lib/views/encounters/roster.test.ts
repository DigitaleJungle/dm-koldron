import { describe, expect, it } from "vitest";
import type { CatalogRow } from "$lib/api/catalog";
import { addToRoster, characterLevel, parseQuantity, rowsByKey } from "./roster";

const goblin = (pack: string, others: CatalogRow["others"] = []): CatalogRow => ({
  kind: "monsters",
  pack,
  source: pack.toUpperCase(),
  index: "goblin",
  name: "Goblin",
  fields: { xp: 50 },
  others,
});

describe("roster", () => {
  it("raises the quantity for the same entry and keeps packs apart", () => {
    let roster = addToRoster([], goblin("srd"));
    roster = addToRoster(roster, goblin("srd"));
    roster = addToRoster(roster, goblin("homebrew"));
    expect(roster.map((r) => [r.pack, r.source, r.quantity])).toEqual([["srd", "SRD", 2], ["homebrew", "HOMEBREW", 1]]);
  });

  it("finds a saved row through a bundled duplicate, and nothing for a pack that's off", () => {
    const also = { kind: "monsters", pack: "extra", source: "Extra", index: "goblin", name: "Goblin", chapterIndex: null, chapterName: null };
    const rows = rowsByKey([goblin("srd", [also])]);
    expect(rows.get("srd/goblin")?.pack).toBe("srd");
    expect(rows.get("extra/goblin")?.pack).toBe("srd");
    expect(rows.get("gone/goblin")).toBeUndefined();
  });

  it("accepts only whole quantities from 1 to 999", () => {
    expect(["3", " 12 ", "999", "1000", "0", "-1", "1.5", "", "x"].map(parseQuantity)).toEqual([3, 12, 999, null, null, null, null, null, null]);
  });

  it("sums class levels and flags missing ones", () => {
    const cls = (level: number) => ({ name: "Fighter", level });
    expect(characterLevel([cls(3), cls(2)])).toBe(5);
    expect(characterLevel([])).toBeNull();
    expect(characterLevel(undefined)).toBeNull();
    expect(characterLevel([cls(Number.NaN)])).toBeNull();
  });
});
