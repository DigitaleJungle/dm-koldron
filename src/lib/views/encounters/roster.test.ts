import { describe, expect, it } from "vitest";
import type { CatalogRow } from "$lib/api/catalog";
import type { Character } from "$lib/api/characters";
import { blankCharacter } from "$lib/views/characters/characters";
import { addToRoster, characterLevel, encounterParty, MAX_QUANTITY, rowsByKey } from "./roster";

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

  it("fights with the party as it is now in Party mode, else its own pick without deleted characters", () => {
    const pc = (id: string, onHome: boolean): Character => ({ ...blankCharacter(), id, name: id, onHome });
    const characters = [pc("ann", true), pc("bo", false), pc("cy", true)];
    expect(encounterParty("party", ["bo"], characters)).toEqual(["ann", "cy"]);
    expect(encounterParty("party", ["bo"], null)).toEqual([]); // not known yet
    expect(encounterParty("custom", ["bo", "gone"], characters)).toEqual(["bo"]);
    expect(encounterParty("custom", ["bo", "gone"], null)).toEqual(["bo", "gone"]);
  });

  it("never adds past the most of one monster", () => {
    const full = [{ ...addToRoster([], goblin("srd"))[0], quantity: MAX_QUANTITY }];
    expect(addToRoster(full, goblin("srd"))[0].quantity).toBe(MAX_QUANTITY);
  });

  it("finds a saved row through a bundled duplicate, and nothing for a pack that's off", () => {
    const also = { kind: "monsters", pack: "extra", source: "Extra", index: "goblin", name: "Goblin", chapterIndex: null, chapterName: null };
    const rows = rowsByKey([goblin("srd", [also])]);
    expect(rows.get("srd/goblin")?.pack).toBe("srd");
    expect(rows.get("extra/goblin")?.pack).toBe("srd");
    expect(rows.get("gone/goblin")).toBeUndefined();
  });

  it("sums class levels and flags missing ones", () => {
    const cls = (level: number) => ({ name: "Fighter", level });
    expect(characterLevel([cls(3), cls(2)])).toBe(5);
    expect(characterLevel([])).toBeNull();
    expect(characterLevel(undefined)).toBeNull();
    expect(characterLevel([cls(Number.NaN)])).toBeNull();
  });
});
