import { describe, expect, it } from "vitest";
import { moveTo, ordered } from "./sidebarOrder";

const items = ["home", "library", "characters", "sessions"].map((id) => ({ id }));
const ids = (list: { id: string }[]) => list.map((x) => x.id);

describe("sidebar order", () => {
  it("follows the saved order, unknown and new items after it in their default order", () => {
    expect(ids(ordered(items, []))).toEqual(["home", "library", "characters", "sessions"]);
    expect(ids(ordered(items, ["sessions", "gone", "home"]))).toEqual(["sessions", "home", "library", "characters"]);
  });

  it("moves an entry, ignoring moves out of range", () => {
    expect(moveTo(["a", "b", "c"], 0, 2)).toEqual(["b", "c", "a"]);
    expect(moveTo(["a", "b", "c"], 2, 0)).toEqual(["c", "a", "b"]);
    const same = ["a", "b"];
    expect(moveTo(same, 0, -1)).toBe(same);
    expect(moveTo(same, 1, 2)).toBe(same);
  });
});
