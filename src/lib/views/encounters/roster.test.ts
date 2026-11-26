import { describe, expect, it } from "vitest";
import type { CatalogRow } from "$lib/api/catalog";
import type { Character } from "$lib/api/characters";
import { blankCharacter } from "$lib/views/characters/characters";
import { addToRoster, asVersion, characterLevel, encounterParty, MAX_QUANTITY, rowKeys, rowsByKey, splitRow, versions, versionsByKey, withHp, withQuantity } from "./roster";

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

  it("finds every version of a bundled monster with its own XP, and nothing for a pack that's off", () => {
    const also = { kind: "monsters", pack: "extra", source: "Extra", index: "goblin", name: "Goblin", chapterIndex: null, chapterName: null, fields: { xp: 100 } };
    const rows = rowsByKey([goblin("srd", [also])]);
    expect(rows.get("srd/goblin")?.fields.xp).toBe(50);
    expect([rows.get("extra/goblin")?.pack, rows.get("extra/goblin")?.source, rows.get("extra/goblin")?.fields.xp]).toEqual(["extra", "Extra", 100]);
    expect(rows.get("gone/goblin")).toBeUndefined();
  });

  it("lists a row's own entry first, then each bundled version, which adds as its own roster row", () => {
    const also = { kind: "monsters", pack: "extra", source: "Extra", index: "goblin", name: "Goblin", chapterIndex: null, chapterName: null, fields: { xp: 100 } };
    const [own, other] = versions(goblin("srd", [also]));
    expect([own.pack, other.pack, other.others]).toEqual(["srd", "extra", []]);
    expect(versions(goblin("srd"))).toHaveLength(1);
    expect(addToRoster(addToRoster([], own), other).map((r) => r.pack)).toEqual(["srd", "extra"]);
  });

  it("sums class levels and flags missing ones", () => {
    const cls = (level: number) => ({ name: "Fighter", level });
    expect(characterLevel([cls(3), cls(2)])).toBe(5);
    expect(characterLevel([])).toBeNull();
    expect(characterLevel(undefined)).toBeNull();
    expect(characterLevel([cls(Number.NaN)])).toBeNull();
  });
});

describe("each monster's HP", () => {
  const row = { pack: "srd", index: "goblin", name: "Goblin", source: "SRD", quantity: 3 };

  it("sets one monster's HP and stores nothing past the last one set", () => {
    expect(withHp(row, 1, 9).hp).toEqual([null, 9]);
    expect(withHp(withHp(row, 1, 9), 1, null).hp).toEqual([]);
  });

  it("keeps the HP with its monsters when splitting or dropping some", () => {
    const set = { ...row, quantity: 5, hp: [1, 2, 3, null, 5] };
    expect(splitRow([set], 0).map((r) => r.hp)).toEqual([[1, 2, 3], [null, 5]]);
    expect(withQuantity(set, 2).hp).toEqual([1, 2]);
  });

  it("switches to another pack's version with its own HP", () => {
    const xmm = { ...goblin("xmm"), name: "Goblin Warrior" };
    expect(asVersion({ ...row, hp: [4] }, xmm)).toMatchObject({ pack: "xmm", name: "Goblin Warrior", quantity: 3, hp: [] });
  });
});

describe("versions by key", () => {
  it("finds every version of a monster from any of them", () => {
    const also = { kind: "monsters", pack: "xmm", source: "XMM", index: "goblin", name: "Goblin", chapterIndex: null, chapterName: null, fields: {} };
    const map = versionsByKey([goblin("srd", [also])]);
    expect(map.get("xmm/goblin")?.map((v) => v.pack)).toEqual(["srd", "xmm"]);
    expect(map.get("srd/goblin")).toBe(map.get("xmm/goblin"));
  });
});

describe("split groups", () => {
  const row = (quantity: number, index = "goblin") => ({ pack: "srd", index, name: index, source: "SRD", quantity });

  it("splits in half beside the row, the odd one in the first, and never a single monster", () => {
    expect(splitRow([row(1, "orc"), row(5), row(2, "wolf")], 1).map((r) => [r.index, r.quantity])).toEqual([
      ["orc", 1],
      ["goblin", 3],
      ["goblin", 2],
      ["wolf", 2],
    ]);
    const single = [row(1)];
    expect(splitRow(single, 0)).toBe(single);
  });

  it("gives every row its own key, and adding the same monster raises the first group", () => {
    const roster = splitRow([row(4)], 0);
    expect(rowKeys([...roster, row(1, "orc")])).toEqual(["srd/goblin", "srd/goblin#2", "srd/orc"]);
    expect(addToRoster(roster, { ...goblin("srd"), index: "goblin" }).map((r) => r.quantity)).toEqual([3, 2]);
  });
});
