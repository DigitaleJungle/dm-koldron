import { describe, expect, it } from "vitest";
import { HEX_HEIGHT, combSize, hexcomb } from "./hexcomb";

describe("hexcomb", () => {
  it("places one hexagon per monster, each touching another, none overlapping", () => {
    for (const count of [1, 2, 7, 20]) {
      const cells = hexcomb(count, `seed-${count}`);
      expect(cells).toHaveLength(count);
      const distances = (i: number) => cells.filter((_, j) => j !== i).map((o) => Math.hypot(o.x - cells[i].x, o.y - cells[i].y));
      cells.forEach((_, i) => {
        expect(Math.min(...distances(i))).toBeGreaterThan(0.8); // never on top of another
        if (count > 1) expect(Math.min(...distances(i))).toBeLessThan(0.95); // a neighbour sits about 0.88 away
      });
    }
  });

  it("never grows taller than maxRows hexagons, however many there are", () => {
    for (const count of [3, 8, 30]) {
      const height = combSize(hexcomb(count, `rows-${count}`, 2)).height;
      expect(height).toBeLessThanOrEqual(2 * HEX_HEIGHT + 0.05 + 1e-9); // two rows and the gap between them
    }
    expect(hexcomb(30, "rows", 2)).toHaveLength(30);
    expect(hexcomb(5, "rows", 1)).toEqual(hexcomb(5, "rows", 2)); // 1 can't be done: it counts as 2
  });

  it("allows half rows: 2.5 hexagons high", () => {
    const heights = ["a", "b", "c", "d"].map((seed) => combSize(hexcomb(30, seed, 2.5)).height);
    // Two and a half rows: 1.5 steps of a row and its gap, plus one hexagon.
    heights.forEach((h) => expect(h).toBeLessThanOrEqual(1.5 * (HEX_HEIGHT + 0.05) + HEX_HEIGHT + 1e-9));
    expect(Math.max(...heights)).toBeGreaterThan(2 * HEX_HEIGHT + 0.05 + 1e-9); // and does use the extra half row
  });

  it("prefers spots that touch more hexagons, so the comb is compact", () => {
    // Touching pairs among 7 hexagons: a chain has 6, the tightest flower 12. Averaged over many seeds.
    const pairs = (seed: string) => {
      const cells = hexcomb(7, seed);
      return cells.flatMap((a, i) => cells.slice(i + 1).filter((b) => Math.hypot(a.x - b.x, a.y - b.y) < 0.95)).length;
    };
    const seeds = Array.from({ length: 200 }, (_, i) => `compact-${i}`);
    expect(seeds.reduce((sum, seed) => sum + pairs(seed), 0) / seeds.length).toBeGreaterThan(10.2); // the old, plain weighting gave 9.4; this gives 10.7
  });

  it("keeps the same comb for the same seed and starts at 0, 0", () => {
    expect(hexcomb(9, "abc")).toEqual(hexcomb(9, "abc"));
    const cells = hexcomb(9, "abc");
    expect(Math.min(...cells.map((c) => c.x))).toBe(0);
    expect(Math.min(...cells.map((c) => c.y))).toBe(0);
  });

  it("measures the comb", () => {
    expect(hexcomb(0, "x")).toEqual([]);
    expect(combSize([])).toEqual({ width: 1, height: HEX_HEIGHT });
    expect(combSize(hexcomb(1, "x"))).toEqual({ width: 1, height: HEX_HEIGHT });
  });
});
