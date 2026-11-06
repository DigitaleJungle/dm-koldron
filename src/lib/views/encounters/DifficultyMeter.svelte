<script lang="ts">
  // A vertical meter: the party's band edges as marks, the encounter's XP as the fill, the band named above it.
  // Missing data gives no rating ("–") and says what's missing where the XP total goes.
  import type { Snippet } from "svelte";
  import { strings } from "$lib/strings";
  import { BANDS, bandEdges, type Difficulty } from "./difficulty";

  let {
    result,
    partySize,
    monsterCount,
    hasBudgets,
    budgetError = null,
    children,
  }: {
    result: Difficulty;
    partySize: number;
    monsterCount: number;
    hasBudgets: boolean;
    budgetError?: string | null;
    /** Shown at the top of the panel, above the meter (the party picker). */
    children?: Snippet;
  } = $props();

  const t = strings.encounters.meter;
  const fmt = new Intl.NumberFormat();
  // Headroom above High, so a total just over it still shows below the top.
  const top = $derived(Math.max(result.budgets[2] * 1.2, 1));
  const pct = (xp: number) => `${Math.min(100, (xp / top) * 100)}%`;
  const showBudgets = $derived(partySize > 0 && result.budgets[2] > 0);
  const edges = $derived(bandEdges(result.budgets));
  // Each band's name sits in the middle of its stretch of the meter.
  const middles = $derived(BANDS.map((_, i) => ((edges[i - 1] ?? 0) + (edges[i] ?? top)) / 2));
  // Without a rating, the reason takes the XP total's place: the most important one only, so the meter never moves.
  const note = $derived.by((): { text: string; warn: boolean } | null => {
    if (budgetError) return { text: t.budgetFailed, warn: true };
    if (!hasBudgets) return { text: t.noBudgets, warn: true };
    if (partySize === 0) return { text: t.noParty, warn: false };
    if (result.missingLevels) return { text: t.missingLevels(result.missingLevels), warn: true };
    if (result.unknownXp) return { text: t.unknownXp(result.unknownXp), warn: true };
    if (monsterCount === 0) return { text: t.noMonsters, warn: false };
    return null;
  });

  // The meter is one image to screen readers, so its label says what the names and numbers on it show.
  const scaleLabel = $derived(
    BANDS.map((band, i) => (i < edges.length ? t.upTo(t.bands[band], fmt.format(Math.floor(edges[i]))) : t.above(t.bands[band]))).join(", "),
  );
</script>

<section class="meter" aria-label={t.title}>
  {@render children?.()}
  <!-- Always two lines: the band (or "–") and the XP total (or why there's no rating). -->
  <div class="reading" role="status">
    <span class="band band-{result.band ?? 'none'}">{result.band ? t.bands[result.band] : t.noRating}</span>
    <span class="xp" class:warn={note?.warn}>{note ? note.text : t.xp(fmt.format(result.total))}</span>
    {#if budgetError}<span class="sr-only">{budgetError}</span>{/if}
  </div>

  <div class="scale" role="img" aria-label={showBudgets ? scaleLabel : t.label}>
    <div class="track">
      {#if showBudgets}
        <div class="fill band-{result.band ?? 'none'}" style:height={pct(result.total)}></div>
        {#each edges as xp, i (i)}
          <div class="mark" style:bottom={pct(xp)}><span class="threshold">{t.xp(fmt.format(Math.floor(xp)))}</span></div>
        {/each}
        {#each BANDS as band, i (band)}
          <span class="band-label" class:current={band === result.band} style:bottom={pct(middles[i])}>{t.bands[band]}</span>
        {/each}
        {#if result.total > 0}<div class="marker" style:bottom={pct(result.total)}></div>{/if}
      {/if}
    </div>
  </div>

</section>

<style>
  /* A set width, so long notes wrap instead of widening the column it sits in. */
  .meter {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    width: var(--meter-width);
    padding: var(--space-3) var(--space-4);
    background: var(--color-surface);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .reading {
    display: flex;
    flex-direction: column;
  }

  .band {
    color: var(--band);
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
  }

  /* One line, whatever it says, so nothing below it moves. */
  .xp {
    overflow: hidden;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scale {
    display: flex;
    justify-content: flex-start;
    height: var(--meter-height);
    padding: var(--space-2) 0;
  }

  .track {
    position: relative;
    width: var(--space-6);
    height: 100%;
    background: var(--color-surface-sunken);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-sm);
  }

  .fill {
    position: absolute;
    inset: auto 0 0 0;
    border-radius: var(--radius-sm);
    background: var(--band);
  }

  .mark {
    position: absolute;
    left: -2px;
    right: -2px;
    border-top: 2px solid var(--color-text);
  }

  .band-label {
    position: absolute;
    left: calc(100% + var(--space-2));
    transform: translateY(50%);
    white-space: nowrap;
    font-size: var(--text-sm);
    color: var(--color-text-muted);
  }

  /* Level with its line, lined up with the band names (the mark sticks out 2px past the track). */
  .threshold {
    position: absolute;
    left: calc(100% + var(--space-2) - 2px);
    transform: translateY(-50%);
    white-space: nowrap;
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
    color: var(--color-text-muted);
  }

  .band-label.current {
    color: var(--color-text);
    font-weight: var(--weight-bold);
  }

  .marker {
    position: absolute;
    right: calc(100% + var(--space-1));
    transform: translateY(50%);
    width: 0;
    height: 0;
    border-block: var(--space-2) solid transparent;
    border-left: var(--space-2) solid var(--color-text);
  }

  .warn {
    color: var(--color-danger);
  }
</style>
