<script lang="ts">
  // A character's card, on the Characters screen and in the encounter builder's party: the square picture with the
  // total level on a dark hexagon, and beside it from the top the name, class line and HP. The name's button covers
  // the whole card. With `selected` the card is a toggle (pressed: an accent border and a check), else it opens.
  // `corner` goes in the bottom-right corner, above the card's cover (the Characters screen's party button).
  // `compact`: smaller, for the encounter builder's party.
  import type { Snippet } from "svelte";
  import { pictureSrc } from "$lib/api/campaigns";
  import type { Character } from "$lib/api/characters";
  import { strings } from "$lib/strings";
  import { classLine, totalLevel } from "./characters";

  let {
    character,
    onclick,
    selected,
    disabled = false,
    heading = "h2",
    compact = false,
    corner,
  }: {
    character: Character;
    onclick: () => void;
    /** Set: the card selects (true: it's selected). Unset: the card opens. */
    selected?: boolean;
    disabled?: boolean;
    /** The name's heading level, to fit the page's outline. */
    heading?: "h2" | "h3" | "h4";
    compact?: boolean;
    corner?: Snippet;
  } = $props();

  const t = strings.characters;
  const formatNumber = new Intl.NumberFormat();
</script>

<article class="card" class:selected class:compact>
  <div class="head">
    <div class="portrait">
      {#if character.picture}
        <img class="picture" src={pictureSrc(character.picture)} alt="" />
      {:else}
        <span class="picture no-picture" aria-hidden="true"><i class="ti ti-user"></i></span>
      {/if}
      <span class="hex level"><span class="sr-only">{t.levelLabel}</span> {totalLevel(character)}</span>
    </div>
    <div class="info">
      <svelte:element this={heading} class="title">
        <button class="name" aria-pressed={selected} {disabled} {onclick}>{character.name}</button>
      </svelte:element>
      <p>{classLine(character)}</p>
      {#if character.maxHp !== null}
        <!-- Current HP left empty shows as "—": it isn't known, so it isn't assumed full. -->
        <p class="hp">
          <span class="current">{character.currentHp === null ? "—" : formatNumber.format(character.currentHp)}</span>
          / {formatNumber.format(character.maxHp)} <span class="label">{t.hp}</span>
        </p>
      {/if}
    </div>
  </div>
  {#if selected}<i class="ti ti-circle-check check" aria-hidden="true"></i>{/if}
  {@render corner?.()}
</article>

<style>
  .card {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .card:hover {
    border-color: var(--color-border-strong);
  }

  /* Selected for the encounter: the accent border and fill, and a check (so it isn't colour alone). */
  .card.selected {
    border-color: var(--color-accent);
    background: var(--color-accent-soft);
  }

  .check {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    color: var(--color-accent);
    font-size: var(--icon-size-lg);
  }

  /* The name, class line and HP beside the picture, from its top. */
  .head {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
  }

  .portrait {
    position: relative;
    flex: none;
    width: var(--character-portrait-size);
    height: var(--character-portrait-size);
  }

  /* Pictures are square (they're saved square), with the card's corners. */
  .picture {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    border-radius: var(--radius-md);
    object-fit: cover;
  }

  .hex {
    display: grid;
    place-items: center;
    clip-path: polygon(25% 0, 75% 0, 100% 50%, 75% 100%, 25% 100%, 0 50%);
  }

  .no-picture {
    background: var(--color-border);
    color: var(--color-text-muted);
    font-size: var(--icon-size-lg);
  }

  /* The total level on a dark hexagon at the portrait's lower right, like the ×count on an encounter's monster. */
  .hex.level {
    position: absolute;
    right: calc(-1 * var(--space-2));
    bottom: calc(-1 * var(--space-1));
    width: var(--character-level-width);
    height: var(--character-level-height);
    background: var(--iron);
    color: var(--bone);
    font: var(--text-dice);
  }

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .info p {
    margin: 0;
    overflow-wrap: anywhere;
  }

  /* "12 / 20 HP": the current HP large, in the party name's font. */
  .hp .current {
    font: var(--text-h2);
    font-variant-numeric: lining-nums tabular-nums; /* old-style digits (Georgia's default) would dip below "/ 20 HP" */
  }

  .title {
    margin: 0;
    overflow: hidden;
    font: var(--text-h2);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Always underlined, but in a transparent colour until the card is hovered: a colour can ease in and out (the
     global hover transition), a line can't. */
  .name {
    padding: 0;
    border: none;
    background: none;
    color: var(--color-text);
    font: inherit;
    text-align: left;
    text-decoration: underline transparent;
    text-decoration-thickness: 1px;
    text-underline-offset: 4px;
    cursor: pointer;
  }

  .name:disabled {
    cursor: default;
  }

  /* Covers the card, under the corner's button. */
  .name::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: var(--radius-md);
  }

  .name:focus-visible {
    outline: none;
  }

  .name:focus-visible::after {
    outline: var(--focus-ring) solid var(--color-focus);
    outline-offset: 2px;
  }

  .card:hover .name:not(:disabled) {
    text-decoration-color: currentColor;
  }

  /* Compact: less padding, a smaller picture and level, the name and HP a size down. */
  .compact {
    padding: var(--space-3);
  }

  .compact .head {
    gap: var(--space-3);
  }

  .compact .portrait {
    width: var(--character-portrait-size-compact);
    height: var(--character-portrait-size-compact);
  }

  .compact .hex.level {
    width: var(--character-level-width-compact);
    height: var(--character-level-height-compact);
    font-size: var(--text-xs);
  }

  .compact .title,
  .compact .hp .current {
    font-size: var(--text-lg);
    line-height: var(--leading-tight);
  }

  .compact .info {
    font-size: var(--text-sm);
    padding-right: var(--space-5); /* clear of the check */
  }
</style>
