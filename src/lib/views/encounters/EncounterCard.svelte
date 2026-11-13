<script lang="ts">
  // An encounter as a card: a hexagon collage of the monsters (one per roster row, placed at random but the same each
  // visit), name, Start (Ready) or Draft, how many monsters, difficulty, XP and party. The name opens it in Prepare; with
  // `onremove`, a cross beside Start removes it (a scene's link); `compact` (a scene's links) leaves out Start and XP.
  import HexCollage from "$lib/components/HexCollage.svelte";
  import { monsterPicture } from "$lib/stores/monsterPictures";
  import { strings } from "$lib/strings";
  import { BAND_ICONS, type EncounterCardData } from "./card";
  import { entryKey } from "./roster";

  let {
    card: c,
    base,
    onremove,
    removeLabel = "",
    compact = false,
  }: {
    card: EncounterCardData;
    /** The campaign's Encounters address. */
    base: string;
    onremove?: () => void;
    removeLabel?: string;
    compact?: boolean;
  } = $props();

  const t = strings.encounters;
  const fmt = new Intl.NumberFormat();

  // Pictures, when its pack's images are on.
  let pictures = $state<Record<string, string | null>>({});
  $effect(() => {
    for (const row of c.rows) {
      const key = entryKey(row);
      if (row.image) monsterPicture(row.pack, row.image).then((src) => (pictures[key] = src));
    }
  });

</script>

<article class="card">
  <div class="art">
    <HexCollage
      tiles={c.rows.map((r) => ({ key: r.key, picture: pictures[entryKey(r)] ?? null, letter: r.name.charAt(0), count: r.quantity }))}
      seed={c.id}
      hexWidth="var(--encounter-hex-width)"
    />
  </div>
  <div class="body">
    <div class="title">
      <!-- The name's link covers the whole card; Run (and the cross) sit above it. -->
      <!-- In a scene's links, under its Encounters heading. -->
      <svelte:element this={compact ? "h4" : "h2"} class="name"><a class="open" href="{base}/{c.id}">{c.name}</a></svelte:element>
      {#if c.status === "ready"}
        {#if !compact}
          <a class="btn btn-primary run" href="{base}/{c.id}/run">
            {t.start}
            <i class="ti ti-player-play" aria-hidden="true"></i>
          </a>
        {/if}
      {:else}
        <span class="status">{t.list.draft}</span>
      {/if}
      {#if onremove}
        <button class="icon-btn remove" aria-label={removeLabel} onclick={onremove}>
          <i class="ti ti-x" aria-hidden="true"></i>
        </button>
      {/if}
    </div>
    <p class="muted">{c.count > 0 ? t.monsterCount(c.count) : t.list.noMonsters}</p>
    <div class="stats">
      <span class="band band-{c.band ?? 'none'}">
        {#if c.band}<i class="ti {BAND_ICONS[c.band]}" aria-hidden="true"></i>{/if}
        {c.band ? t.meter.bands[c.band] : t.meter.noRating}
      </span>
      {#if !compact}<span>{t.meter.xp(fmt.format(c.xp))}</span>{/if}
      <span class="party muted">{c.partyLine}</span>
    </div>
  </div>
</article>

<style>
  .card {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: var(--color-text);
    text-decoration: none;
  }

  .card:hover {
    border-color: var(--color-border-strong);
  }

  .card:hover .open {
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  /* The keyboard's focus ring goes round the whole card, which the link covers. */
  .card:has(.open:focus-visible) {
    outline: var(--focus-ring) solid var(--color-focus);
    outline-offset: 2px;
  }

  .open {
    color: inherit;
    text-decoration: none;
    outline: none;
  }

  .open::after {
    content: "";
    position: absolute;
    inset: 0;
  }

  .run,
  .remove {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
  }

  .run {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
    text-decoration: none;
  }

  .art {
    height: var(--encounter-art-height);
    padding: var(--space-4);
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
    border-bottom: var(--border-width) solid var(--color-border);
    background: var(--color-bg);
  }

  .body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4);
  }

  .title {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
  }

  .name {
    flex: 1;
    margin: 0;
    font: var(--text-body);
    font-weight: var(--weight-bold);
  }

  .status {
    flex-shrink: 0;
    padding: 2px var(--space-2);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .muted {
    margin: 0;
    color: var(--color-text-muted);
    font: var(--text-body-sm);
  }

  .stats {
    margin-top: auto;
    padding-top: var(--space-2);
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-4);
    border-top: var(--border-width) solid var(--color-border);
    font: var(--text-dice);
  }

  .band {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--band);
    font: var(--text-body-sm);
    font-weight: var(--weight-bold);
  }

  .band .ti {
    font-size: var(--icon-size-sm);
  }

  .party {
    margin-left: auto;
  }
</style>
