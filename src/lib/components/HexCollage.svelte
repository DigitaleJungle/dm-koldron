<script lang="ts" module>
  /** One hexagon: its picture (cut to a hexagon), else its letter; `count` above 1 adds a ×count hexagon. */
  export type HexTile = { key: string; picture: string | null; letter: string; count?: number };
</script>

<script lang="ts">
  // Hexagons placed at random but touching on a hex grid (hexcomb.ts), the first in the middle; `seed` keeps the same
  // collage between visits. `hexWidth` is one hexagon's width on screen; the collage is never taller than its box
  // (which needs a set height) or wider, and shrinks to fit. With no tiles, one empty hexagon. Used by the encounter
  // cards (the monsters), the Characters screen's party band and Home's Party. With `onremove`, each hexagon gets a
  // small cross near its top right corner (`removeLabel` names it) that calls it with the tile's key. With `add`, the
  // spot the next hexagon would take is kept free and holds `add` (rendered in that hexagon's box): hidden until it's
  // hovered or has focus, and shown at once while there are no tiles.
  import type { Snippet } from "svelte";
  import { HEX_HEIGHT, combSize, hexPoints, hexcomb } from "./hexcomb";

  let {
    tiles,
    seed,
    hexWidth,
    maxRows = Infinity,
    onremove,
    removeLabel,
    disabled = false,
    add,
  }: {
    tiles: HexTile[];
    seed: string;
    hexWidth: string;
    /** At most this many hexagons high: the collage grows sideways instead. */
    maxRows?: number;
    onremove?: (key: string) => void;
    removeLabel?: (tile: HexTile) => string;
    /** The crosses can't be used (a change is under way). */
    disabled?: boolean;
    /** What the free spot for the next hexagon holds (e.g. an add button). */
    add?: Snippet;
  } = $props();

  const id = $props.id();
  // hexcomb places them one by one from the seed, so asking for one more leaves the others where they are.
  const cells = $derived(hexcomb(tiles.length + (add ? 1 : 0), seed, maxRows));
  const size = $derived(combSize(cells));
  const height = $derived(`min(calc(${size.height + 0.2} * ${hexWidth}), 100%)`);
  /** Where a point of the drawing (in hexagon widths) is, in % of the collage's box. */
  const at = (x: number, y: number) => ({ left: ((x + 0.1) / (size.width + 0.2)) * 100, top: ((y + 0.1) / (size.height + 0.2)) * 100 });
</script>

{#if onremove || add}
  <!-- The collage with its crosses laid over it, the same size as the collage alone. -->
  <div
    class="removable"
    style:width="min(calc({size.width + 0.2} * {hexWidth}), 100%)"
    style:aspect-ratio="{size.width + 0.2} / {size.height + 0.2}"
  >
    {@render drawing("100%")}
    {#each tiles as tile, i (tile.key)}
      {@const corner = at(cells[i].x, cells[i].y)}
      <!-- The hexagon's box: hovering it shows its cross, near its top right corner. -->
      <div
        class="tile"
        style:left="{corner.left}%"
        style:top="{corner.top}%"
        style:width="{100 / (size.width + 0.2)}%"
        style:height="{(HEX_HEIGHT * 100) / (size.height + 0.2)}%"
      >
        {#if onremove}
          <button class="remove" aria-label={removeLabel?.(tile) ?? tile.letter} {disabled} onclick={() => onremove(tile.key)}>
            <i class="ti ti-x" aria-hidden="true"></i>
          </button>
        {/if}
      </div>
    {/each}
    {#if add}
      {@const corner = at(cells[tiles.length].x, cells[tiles.length].y)}
      <div
        class="tile spot"
        class:shown={tiles.length === 0}
        style:left="{corner.left}%"
        style:top="{corner.top}%"
        style:width="{100 / (size.width + 0.2)}%"
        style:height="{(HEX_HEIGHT * 100) / (size.height + 0.2)}%"
      >
        {@render add()}
      </div>
    {/if}
  </div>
{:else}
  {@render drawing(height)}
{/if}

{#snippet drawing(h: string)}
<!-- One unit is one hexagon's width. -->
<svg
  class="collage"
  viewBox="-0.1 -0.1 {size.width + 0.2} {size.height + 0.2}"
  style:height={h}
  aria-hidden="true"
>
  <!-- The hexagon every picture is cut to, in the picture's own box. -->
  <clipPath id="{id}-hex" clipPathUnits="objectBoundingBox">
    <polygon points="0.25,0 0.75,0 1,0.5 0.75,1 0.25,1 0,0.5" />
  </clipPath>
  {#if tiles.length === 0 && !add}
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
{/snippet}

<style>
  /* The width follows the height (the viewBox's proportions). */
  .collage {
    flex: none;
    width: auto;
    max-width: 100%;
  }

  /* Sized from its width in the collage's proportions, so the crosses stay on their hexagons when it shrinks. */
  .removable {
    position: relative;
    flex: none;
    display: flex;
    align-self: center;
  }

  .removable .collage {
    width: 100%;
  }

  .tile {
    position: absolute;
  }

  /* A small round cross near the hexagon's top right corner, shown while the hexagon is hovered (or the cross has the
     keyboard's focus). */
  .remove {
    position: absolute;
    top: 12%;
    left: 80%;
    display: grid;
    place-items: center;
    width: var(--icon-size);
    height: var(--icon-size);
    padding: 0;
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: 50%;
    background: var(--color-surface);
    color: var(--color-text);
    font-size: var(--text-xs);
    opacity: 0;
    transform: translate(-50%, -50%);
    transition: opacity var(--duration-hover) var(--ease-hover);
    cursor: pointer;
  }

  .tile:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  /* The free spot: what it holds shows while it's hovered or has focus (always while there are no tiles). */
  .spot {
    display: flex;
    opacity: 0;
    transition: opacity var(--duration-hover) var(--ease-hover);
  }

  .spot:hover,
  .spot:focus-within,
  .spot.shown {
    opacity: 1;
  }

  :global([data-motion="reduce"]) .spot {
    transition: none;
  }

  :global([data-motion="reduce"]) .remove {
    transition: none;
  }

  .remove:hover:not(:disabled) {
    border-color: var(--color-danger);
    color: var(--color-danger);
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
