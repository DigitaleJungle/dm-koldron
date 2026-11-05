import { describe, expect, it } from "vitest";
import type { SearchHit } from "$lib/api/search";
import { groupByKind, hitAddress, kindLabel, placeLabel, searchAddress } from "./search";

const hit = (kind: string, name: string, chapter?: [string, string]): SearchHit => ({
  kind,
  pack: "srd",
  source: "SRD 5.2",
  index: name.toLowerCase(),
  name,
  chapterIndex: chapter?.[0] ?? null,
  chapterName: chapter?.[1] ?? null,
});

describe("search results", () => {
  it("open a rule section in its chapter and an entry in the Catalog", () => {
    const section = hit("rule-sections", "Blinded", ["rules-glossary", "Rules Glossary"]);
    expect(hitAddress(section)).toBe("/library?pack=srd&chapter=rules-glossary&section=blinded");
    expect(placeLabel(section)).toBe("Rules · Rules Glossary");
    expect(kindLabel("rule-sections")).toBe("Rules");
    const spell = hit("spells", "Fireball");
    expect(hitAddress(spell)).toBe("/library?tab=catalog&cat=spells&entry=srd%2Ffireball");
    expect(placeLabel(spell)).toBe("Catalog");
    expect(kindLabel("spells")).toBe("Spells");
    expect(searchAddress(" fire ball ")).toBe("/search?q=fire+ball");
  });

  it("group by kind in display order, keeping the best first within a kind", () => {
    const hits = [hit("spells", "B"), hit("monsters", "M"), hit("rule-sections", "R", ["c", "C"]), hit("spells", "A"), hit("plugin-thing", "P")];
    const groups = groupByKind(hits);
    expect(groups.map((g) => g.kind)).toEqual(["rule-sections", "spells", "monsters", "plugin-thing"]);
    expect(groups[1].hits.map((h) => h.name)).toEqual(["B", "A"]);
  });
});
