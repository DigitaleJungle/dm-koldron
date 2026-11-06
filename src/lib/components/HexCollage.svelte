<script lang="ts" module>
  /** One hexagon: its picture (cut to a hexagon), else its letter; `count` above 1 adds a ×count hexagon. */
  export type HexTile = { key: string; picture: string | null; letter: string; count?: number };
</script>

<script lang="ts">
  // Hexagons placed at random but touching on a hex grid (hexcomb.ts), the first in the middle; `seed` keeps the same
  // collage between visits. `hexWidth` is one hexagon's width on screen; the collage is never taller than its box
  // (which needs a set height) or wider, and shrinks to fit. With no tiles, one empty hexagon. Used by the encounter
  // cards (the monsters) and the Characters screen's party band.
  import { HEX_HEIGHT, combSize, hexPoints, hexcomb } from "./hexcomb";

  let {
    tiles,
    seed,
    hexWidth,
    maxRows = Infinity,
  }: {
    tiles: HexTile[];
    seed: string;
    hexWidth: string;
    /** At most this many hexagons high: the collage grows sideways instead. */
    maxRows?: number;
  } = $props();

  const id = $props.id();
  const cells = $derived(hexcomb(tiles.length, seed, maxRows));
  const size = $derived(combSize(cells));
</script>

<!-- One unit is one hexagon's width. -->
<svg
  class="collage"
  viewBox="-0.1 -0.1 {size.width + 0.2} {size.height + 0.2}"
  style:height="min(calc({size.height + 0.2} * {hexWidth}), 100%)"
  aria-hidden="true"
>
  <!-- The hexagon every picture is cut to, in the picture's own box. -->
  <clipPath id="{id}-hex" clipPathUnits="objectBoundingBox">
    <polygon points="0.25,0 0.75,0 1,0.5 0.75,1 0.25,1 0,0.5" />
  </clipPath>
  {#if tiles.length === 0}
    <polygon class="blank" points={hexPoints(0, 0)} />
  {/if}
  {#each tiles as tile, i (tile.key)}
    {@const { x, y } = cells[i]}
    {#if tile.picture}
      <image href={tile.picture} {x} {y} width="1" height={HEX_HEIGHT} preserveAspectRatio="xMidYMid slice" clip-path="url(#{id}-hex)" />
    {:else}
      <polygon class="letter-bg" points={hexPoints(x, y)} />
      <text class="letter" x={x + 0.5} y={y + HEX_HEIGHT / 2}>{tile.letter}</text>
    {/if}
    {#if (tile.count ?? 1) > 1}
      <!-- The ×count on a small hexagon at the lower right, as on the Prepare tiles. -->
      <polygon class="badge" points={hexPoints(x + 0.6, y + HEX_HEIGHT * 0.64, 0.44)} />
      <text class="badge-text" x={x + 0.82} y={y + HEX_HEIGHT * 0.86}>×{tile.count}</text>
    {/if}
  {/each}
</svg>

<style>
  /* The width follows the height (the viewBox's proportions). */
  .collage {
    flex: none;
    width: auto;
    max-width: 100%;
  }

  .blank {
    fill: var(--color-surface-sunken);
  }

  .letter-bg {
    fill: var(--color-border);
  }

  .letter,
  .badge-text {
    text-anchor: middle;
    dominant-baseline: central;
  }

  .letter {
    fill: var(--color-text-muted);
    font-family: var(--font-display);
    font-size: 0.4px;
    font-weight: var(--weight-display);
  }

  .badge {
    fill: var(--potion-glow);
  }

  .badge-text {
    fill: var(--charcoal);
    font-family: var(--font-mono);
    font-size: 0.19px;
  }
</style>
