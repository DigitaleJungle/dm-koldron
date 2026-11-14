import { describe, expect, it } from "vitest";
import { fillRandom, fromSaved, moveWidget, pickRandom, snapWidth } from "./widgets";

describe("home widgets", () => {
  it("reads a saved layout, keeping the known widgets once and adding the missing ones", () => {
    expect(fromSaved(null).order).toEqual(["party", "nextSession", "summary", "encounter"]);
    const l = fromSaved({ widgets: ["summary", "tools", "summary", "party"], widths: { summary: 3, party: 9 }, locked: true });
    expect(l.order).toEqual(["summary", "party", "nextSession", "encounter"]);
    expect([l.widths.summary, l.widths.party, l.locked]).toEqual([3, 2, true]); // 9 isn't a width: the default stays
    expect(fromSaved(null).locked).toBe(false);
    const r = fromSaved({ widgets: ["summary"], widths: {}, locked: false, removed: ["party", "tools", "party"] });
    expect([r.order, r.removed]).toEqual([["summary", "nextSession", "encounter"], ["party"]]); // removed stays off
  });

  it("moves a widget, clamped to the list", () => {
    expect(moveWidget(["party", "nextSession", "summary"], "party", 2)).toEqual(["nextSession", "summary", "party"]);
    expect(moveWidget(["party", "nextSession"], "nextSession", -5)).toEqual(["nextSession", "party"]);
  });

  it("snaps a dragged edge to whole columns, 1 to 3", () => {
    // Columns 100 wide with gaps of 10, the widget starting at 0.
    expect([snapWidth(40, 0, 100, 10), snapWidth(100, 0, 100, 10), snapWidth(170, 0, 100, 10), snapWidth(900, 0, 100, 10)]).toEqual([1, 1, 2, 3]);
    expect(snapWidth(-50, 0, 100, 10)).toBe(1);
  });

  it("picks at random, never one already shown", () => {
    expect(pickRandom([], [])).toBeNull();
    expect(pickRandom(["a"], ["a"])).toBeNull();
    expect(pickRandom(["a", "b"], ["a"], () => 0)).toBe("b");
    expect(pickRandom(["a", "b", "c"], [], () => 0.99)).toBe("c");
  });

  it("fills to a count with distinct picks, keeping the shown ones", () => {
    expect(fillRandom(["a", "b", "c"], ["b"], 3, () => 0)).toEqual(["b", "a", "c"]);
    expect(fillRandom(["a", "b", "c"], ["c", "a"], 1)).toEqual(["c"]);
    expect(fillRandom(["a", "b"], [], 3, () => 0)).toEqual(["a", "b"]); // fewer than asked: all of them
    expect(fillRandom(["a"], ["gone"], 1)).toEqual(["a"]); // a deleted one is dropped
  });
});
