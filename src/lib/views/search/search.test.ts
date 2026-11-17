import { describe, expect, it } from "vitest";
import type { SearchHit } from "$lib/api/search";
import { groupByKind, hitAddress, hitMeta, kindLabel, placeLabel, searchAddress } from "./search";

const hit = (kind: string, name: string, chapter?: [string, string]): SearchHit => ({
  kind,
  pack: "srd",
  source: "SRD 5.2",
  index: name.toLowerCase(),
  name,
  chapterIndex: chapter?.[0] ?? null,
  chapterName: chapter?.[1] ?? null,
  campaign: null,
  detail: null,
  state: null,
});

describe("search results", () => {
  it("open a rule section in its chapter, an entry in the Catalog and a character in its campaign", () => {
    const section = hit("rule-sections", "Blinded", ["rules-glossary", "Rules Glossary"]);
    expect(hitAddress(section)).toBe("/library?pack=srd&chapter=rules-glossary&section=blinded");
    expect(placeLabel(section)).toBe("Rules · Rules Glossary");
    expect(kindLabel("rule-sections")).toBe("Rules");
    const spell = hit("spells", "Fireball");
    expect(hitAddress(spell)).toBe("/library?tab=catalog&cat=spells&entry=srd%2Ffireball");
    expect(hitMeta(spell)).toBe("Spells · SRD 5.2 · Catalog");
    const mira: SearchHit = { ...hit("characters", "Mira"), pack: "", source: "Strahd", index: "a", campaign: "c1" };
    expect(hitAddress(mira)).toBe("/campaign/c1/characters?edit=a");
    expect(hitMeta(mira)).toBe("Characters · Strahd");
    expect(hitMeta(mira, false)).toBe("Strahd · Characters");
    expect(hitMeta(spell, false)).toBe("SRD 5.2 · Catalog");
    expect(searchAddress(" fire ball ")).toBe("/search?q=fire+ball");
    expect(searchAddress("fire", "c1")).toBe("/search?q=fire&campaign=c1");
  });

  it("open an addon's result in its tab of the app-wide Library, at its place there, from inside a campaign too", () => {
    const found: SearchHit = { ...hit("addon:shelf", "Tomb"), pack: "shelf", source: "Shelf", state: "page=3", detail: "swamp" };
    expect(hitAddress(found, "/campaign/c1/library")).toBe("/library?tab=shelf&state=page%3D3");
  });

  it("group by kind in display order, keeping the best first within a kind", () => {
    const hits = [hit("spells", "B"), hit("monsters", "M"), hit("rule-sections", "R", ["c", "C"]), hit("spells", "A"), hit("plugin-thing", "P"), hit("characters", "Mira")];
    const groups = groupByKind(hits);
    expect(groups.map((g) => g.kind)).toEqual(["characters", "rule-sections", "spells", "monsters", "plugin-thing"]);
    expect(groups[2].hits.map((h) => h.name)).toEqual(["B", "A"]);
  });
});
