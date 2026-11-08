// The encounter list's collage: one flat-topped hexagon per monster, placed at random but touching, on a hex grid.

/** A hexagon's place in grid units: one hexagon is 1 wide and HEX_HEIGHT high (its top-left corner). */
export type Cell = { x: number; y: number };

export const HEX_HEIGHT = Math.sqrt(3) / 2;
/** The space between two hexagons, as a share of a hexagon's width. */
const GAP = 0.05;

/** How much likelier a spot is for each extra placed hexagon it touches (1 would ignore touching). */
const COMPACT = 4;

// Axial neighbours of a flat-topped hexagon.
const NEIGHBOURS = [[0, -1], [1, -1], [1, 0], [0, 1], [-1, 1], [-1, 0]];

/** A small seeded random generator (mulberry32), so an encounter keeps its collage between visits. */
function random(seed: string): () => number {
  let a = [...seed].reduce((h, c) => Math.imul(h ^ c.charCodeAt(0), 2654435761), 1779033703) >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = Math.imul(a ^ (a >>> 15), a | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * `count` cells: the first in the middle, each next one touching those before it. A free spot next to more placed
 * hexagons is much likelier (COMPACT times per extra one), which keeps the comb round rather than a long chain.
 * Positions start at 0, 0. With `maxRows` (in halves too: columns sit half a hexagon apart), a spot that would make
 * the comb taller than that many hexagons (at least 2) isn't picked, so it grows sideways
 * instead (there's always such a spot: the rightmost hexagon has a free neighbour half a row up and one half a row
 * down; with 1 there would be none, since every neighbour is at least half a row up or down).
 */
export function hexcomb(count: number, seed: string, maxRows = Infinity): Cell[] {
  maxRows = Math.max(2, maxRows);
  const next = random(seed);
  const placed: [number, number][] = count > 0 ? [[0, 0]] : [];
  const taken = new Set(["0,0"]);
  // A cell's height on the grid, in hexagons (a column sits half a hexagon lower than the one before it).
  const row = ([q, r]: [number, number]) => r + q / 2;
  while (placed.length < count) {
    const rows = placed.map(row);
    const [top, bottom] = [Math.min(...rows), Math.max(...rows)];
    const fits = (spot: [number, number]) => Math.max(bottom, row(spot)) - Math.min(top, row(spot)) <= maxRows - 1;
    // Every free neighbour that fits, with how many placed hexagons it touches.
    const touches = new Map<string, { spot: [number, number]; n: number }>();
    for (const [q, r] of placed) {
      for (const [dq, dr] of NEIGHBOURS) {
        const spot: [number, number] = [q + dq, r + dr];
        const key = `${spot[0]},${spot[1]}`;
        if (taken.has(key) || !fits(spot)) continue;
        const found = touches.get(key);
        if (found) found.n++;
        else touches.set(key, { spot, n: 1 });
      }
    }
    // Each extra hexagon a spot touches makes it COMPACT times likelier, so the comb fills in instead of trailing off.
    const weighted = [...touches.values()].map(({ spot, n }) => ({ spot, weight: COMPACT ** (n - 1) }));
    let pick = next() * weighted.reduce((sum, w) => sum + w.weight, 0);
    const spot = (weighted.find((w) => (pick -= w.weight) < 0) ?? weighted[weighted.length - 1]).spot;
    placed.push(spot);
    taken.add(`${spot[0]},${spot[1]}`);
  }
  const cells = placed.map(([q, r]) => ({ x: q * (0.75 + GAP), y: (r + q / 2) * (HEX_HEIGHT + GAP) }));
  const minX = Math.min(...cells.map((c) => c.x));
  const minY = Math.min(...cells.map((c) => c.y));
  return cells.map((c) => ({ x: c.x - minX, y: c.y - minY }));
}

/** A hexagon's corners for an SVG polygon, `size` hexagons wide, its top-left corner at x, y. */
export const hexPoints = (x: number, y: number, size = 1) =>
  [[0.25, 0], [0.75, 0], [1, 0.5], [0.75, 1], [0.25, 1], [0, 0.5]].map(([px, py]) => `${x + px * size},${y + py * size * HEX_HEIGHT}`).join(" ");

/** The comb's width and height in grid units. */
export const combSize = (cells: Cell[]) => ({
  width: Math.max(1, ...cells.map((c) => c.x + 1)),
  height: Math.max(HEX_HEIGHT, ...cells.map((c) => c.y + HEX_HEIGHT)),
});
