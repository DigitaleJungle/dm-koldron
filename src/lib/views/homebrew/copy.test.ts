import { describe, expect, it } from "vitest";
import type { CatalogEntry } from "$lib/api/catalog";
import { copyDraft } from "./copy";
import { entryType } from "./entryTypes";

const page = (entry: Record<string, unknown>, related: CatalogEntry["related"] = []): CatalogEntry =>
  ({ pack: "srd", source: "SRD", html: "", parts: {}, also: [], related, entry }) as unknown as CatalogEntry;
const rel = (kind: string, entry: Record<string, unknown>) => ({ kind, entry, html: "", source: "SRD", parts: [] });

describe("copy as homebrew", () => {
  it("keeps the fields but not the index, address or picture", () => {
    const draft = copyDraft(entryType("monsters")!, page({ index: "goblin", url: "/api/2024/monsters/goblin", image: "/x.png", name: "Goblin", hit_points: 7 }));
    expect(draft).toEqual({ index: "", name: "Goblin", hit_points: 7 });
  });

  it("brings a species' traits and a class's levels and features along as texts", () => {
    const owl = copyDraft(
      entryType("species")!,
      page({ index: "owl", name: "Owl", traits: [{ index: "flight" }] }, [rel("traits", { index: "flight", name: "Flight", description: "You fly." })]),
    );
    expect(owl.traits).toEqual([{ name: "Flight", desc: "You fly." }]);

    const tinker = copyDraft(
      entryType("classes")!,
      page({ index: "tinker", name: "Tinker", hit_die: 8 }, [
        rel("levels", { index: "tinker-1", level: 1, prof_bonus: 2, features: [{ index: "spark" }], spellcasting: { cantrips_known: 2 } }),
        rel("features", { index: "spark", name: "Spark", description: "One." }),
      ]),
    );
    expect(tinker.levels).toEqual([{ level: 1, prof_bonus: 2, features: [{ name: "Spark", desc: "One." }], spellcasting: { cantrips_known: 2 } }]);
  });
});
