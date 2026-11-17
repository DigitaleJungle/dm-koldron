import { describe, expect, it } from "vitest";
import { applyPending } from "./rulesPending";

describe("applyPending", () => {
  it("changes only the packs that were switched, and keeps packs that aren't loaded as saved", () => {
    const saved = { disabledPacks: ["gone", "old"], imagePacks: ["srd"] };
    const next = applyPending(saved, { old: true, home: false }, { srd: false, home: true });
    expect(next.disabledPacks.sort()).toEqual(["gone", "home"]);
    expect(next.imagePacks).toEqual(["home"]);
    expect(applyPending(saved, {}, {})).toEqual(saved);
  });
});
