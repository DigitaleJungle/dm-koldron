// The encounter list's collage: one flat-topped hexagon per monster, placed at random but touching, on a hex grid.

/** A hexagon's place in grid units: one hexagon is 1 wide and HEX_HEIGHT high (its top-left corner). */
export type Cell = { x: number; y: number };

export const HEX_HEIGHT = Math.sqrt(3) / 2;
/** The space between two hexagons, as a share of a hexagon's width. */
const GAP = 0.05;

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
 * hexagons is likelier, which keeps the comb round rather than a long chain. Positions start at 0, 0.
 */
export function hexcomb(count: number, seed: string): Cell[] {
  const next = random(seed);
  const placed: [number, number][] = count > 0 ? [[0, 0]] : [];
  const taken = new Set(["0,0"]);
  while (placed.length < count) {
    // Every free neighbour once per placed hexagon it touches.
    const spots = placed.flatMap(([q, r]) => NEIGHBOURS.map(([dq, dr]) => [q + dq, r + dr] as [number, number])).filter(([q, r]) => !taken.has(`${q},${r}`));
    const spot = spots[Math.floor(next() * spots.length)];
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
