<script lang="ts">
  // One step: its title, the list style (a List step), the Ready toggle, and its items by kind.
  import { untrack } from "svelte";
  import Menu from "$lib/components/Menu.svelte";
  import RichText from "$lib/components/RichText.svelte";
  import NotesPanel from "$lib/views/encounters/NotesPanel.svelte";
  import type { Encounter } from "$lib/api/encounters";
  import type { Step } from "$lib/api/sessions";
  import { strings } from "$lib/strings";
  import type { Band } from "$lib/views/encounters/difficulty";
  import { blankItem, choices, LIST_STYLES } from "./sessions";

  let {
    step = $bindable(),
    encountersBase,
    encounters,
    encounterError,
    rate,
    busy,
    oncreate,
    setMap,
  }: {
    step: Step;
    encountersBase: string;
    /** The campaign's encounters; null until they have loaded. */
    encounters: Encounter[] | null;
    encounterError: string | null;
    rate: (encounter: Encounter) => { band: Band | null; xp: number };
    /** A new encounter is being made (or the session saved). */
    busy: boolean;
    oncreate: () => void;
    /** Sets a Map step's picture (saving the session first when the step isn't saved yet). */
    setMap: (path: string | null) => Promise<void>;
  } = $props();

  const t = strings.sessions.steps;
  let titleInput: HTMLInputElement | undefined = $state();
  // A new step has no title yet: start typing it.
  $effect(() => {
    step.id;
    if (!untrack(() => step.title)) titleInput?.focus();
  });
  const ti = strings.sessions.items;
  const e = strings.encounters;
  const fmt = new Intl.NumberFormat();
  const BAND_ICONS: Record<Band, string> = { trivial: "ti-feather", easy: "ti-shield", medium: "ti-sword", hard: "ti-flame", deadly: "ti-skull" };

  let draft = $state("");

  // Another step starts afresh.
  $effect(() => {
    void step.id;
    draft = "";
  });

  const STYLE_ICONS = { bulleted: "ti-list", numbered: "ti-list-numbers", check: "ti-list-check" } as const;
  const styleItems = $derived(
    LIST_STYLES.map((s) => ({ label: t.styles[s], current: step.listStyle === s, onselect: () => (step.listStyle = s) })),
  );

  function addItem(ev: SubmitEvent) {
    ev.preventDefault();
    const text = draft.trim();
    if (!text) return;
    step.items.push({ ...blankItem(), title: text });
    draft = "";
  }

  const current = $derived(step.kind === "encounter" && step.encounter ? encounters?.find((x) => x.id === step.encounter) : undefined);
  const rating = $derived(current ? rate(current) : null);
  const monsterLine = (x: Encounter) => x.monsters.map((m) => (m.quantity > 1 ? `${m.name} ×${fmt.format(m.quantity)}` : m.name)).join(" · ");
  const choiceItems = $derived(
    choices(encounters ?? []).map((x) => ({ label: x.name, onselect: () => (step.encounter = x.id) })),
  );
</script>

<section class="panel" aria-label={step.title || t.kinds[step.kind].label}>
  <div class="head">
    <div class="titles">
      <input bind:this={titleInput} class="title-input" bind:value={step.title} maxlength="100" autocomplete="off" placeholder={t.name} aria-label={t.name} />
    </div>
    {#if step.kind === "list"}
      <Menu id="list-style" label={t.listStyle} items={styleItems} triggerClass="btn">
        {#snippet trigger()}
          <i class="ti {STYLE_ICONS[step.listStyle]}" aria-hidden="true"></i>
          {t.styles[step.listStyle]}
        {/snippet}
      </Menu>
    {/if}
    <button class="btn" aria-pressed={step.ready} onclick={() => (step.ready = !step.ready)}>
      <i class="ti ti-check" aria-hidden="true"></i>
      {step.ready ? t.ready : t.markReady}
    </button>
  </div>

  {#if step.kind === "map"}
    <!-- The encounter map, as is; its pins are the step's (changed in place). -->
    <div class="map-step">
      <!-- Keyed: another Map step starts with no open pin, zoom or error of this one. -->
      {#key step.id}
        <NotesPanel topic="map" notes={{ description: "", pins: step.pins }} map={step.map} locked={false} {setMap} />
      {/key}
    </div>
  {:else if step.kind === "note"}
    <!-- Keyed: the field fills itself once, so another step needs a fresh one. -->
    {#key step.id}
      <RichText
        label={ti.note}
        bind:value={() => step.items[0]?.body ?? "", (v) => (step.items = v ? [{ ...blankItem(), body: v }] : [])}
      />
    {/key}
  {:else if step.kind === "encounter"}
    {#if encounterError}
      <p class="error" role="alert">{encounterError}</p>
    {:else if encounters === null}
      <p class="muted" role="status">{strings.sessions.loading}</p>
    {:else if current && rating}
      <div class="card">
        <div class="enc">
          <strong class="name">{current.name}</strong>
          <span class="tag">{current.status === "ready" ? e.ready : e.list.draft}</span>
        </div>
        <div class="stats">
          <span class="band band-{rating.band ?? 'none'}">
            {#if rating.band}<i class="ti {BAND_ICONS[rating.band]}" aria-hidden="true"></i>{/if}
            {rating.band ? e.meter.bands[rating.band] : e.meter.noRating}
          </span>
          <span class="mono muted">{ti.xp(fmt.format(rating.xp))}</span>
          <span>{current.monsters.length > 0 ? monsterLine(current) : e.list.noMonsters}</span>
        </div>
        <div class="buttons">
          <a class="btn" href="{encountersBase}/{current.id}">{ti.edit}</a>
          {#if current.status === "ready"}
            <a class="btn btn-ready" href="{encountersBase}/{current.id}/run">
              <i class="ti ti-player-play" aria-hidden="true"></i>
              {ti.run}
            </a>
          {/if}
          <button class="btn" onclick={() => (step.encounter = null)}>{ti.unlink}</button>
        </div>
      </div>
    {:else}
      <div class="buttons">
        <button class="btn" disabled={busy} onclick={oncreate}>
          <i class="ti ti-plus" aria-hidden="true"></i>
          {ti.newEncounter}
        </button>
        <Menu id="select-encounter" label={ti.selectEncounter} items={choiceItems.length ? choiceItems : [{ label: ti.noneToSelect, disabled: true }]} align="start" triggerClass="btn">
          {#snippet trigger()}
            <i class="ti ti-list-search" aria-hidden="true"></i>
            {ti.selectEncounter}
          {/snippet}
        </Menu>
      </div>
    {/if}
  {:else}
    <div class="items">
      {#each step.items as item, i (i)}
        <div class="card item" class:done={step.listStyle === "check" && item.done}>
          {#if step.listStyle === "bulleted"}
            <span class="marker" aria-hidden="true">•</span>
          {:else if step.listStyle === "numbered"}
            <span class="marker mono" aria-hidden="true">{fmt.format(i + 1)}.</span>
          {:else}
            <button class="tick" aria-pressed={item.done} aria-label={ti.done(item.title)} onclick={() => (item.done = !item.done)}>
              {#if item.done}<i class="ti ti-check" aria-hidden="true"></i>{/if}
            </button>
          {/if}
          <div class="fields">
            <input class="inline title" bind:value={item.title} maxlength="100" autocomplete="off" aria-label={ti.itemTitle} />
            <textarea class="inline body" bind:value={item.body} rows="2" aria-label={ti.itemBody}></textarea>
          </div>
          <button class="icon-btn" aria-label={ti.remove(item.title)} onclick={() => step.items.splice(i, 1)}>
            <i class="ti ti-trash" aria-hidden="true"></i>
          </button>
        </div>
      {:else}
        <p class="empty">{t.empty}</p>
      {/each}
    </div>
    <form class="add" onsubmit={addItem}>
      <input class="text-field" bind:value={draft} maxlength="100" autocomplete="off" placeholder={step.listStyle === "check" ? ti.tickPlaceholder : ti.namePlaceholder} aria-label={step.listStyle === "check" ? ti.tickPlaceholder : ti.namePlaceholder} />
      <button class="btn" type="submit" disabled={!draft.trim()}>
        <i class="ti ti-plus" aria-hidden="true"></i>
        {ti.add}
      </button>
    </form>
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  /* A Map step fills its column, as the map fills Prepare's details card. */
  .panel:has(.map-step) {
    box-sizing: border-box;
    min-height: 100%;
  }

  .map-step {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-height: var(--map-empty-height);
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: var(--space-3);
  }

  .titles {
    flex: 1 1 var(--session-steps-width);
    min-width: 0;
  }

  .title-input {
    width: 100%;
    min-height: var(--control-height);
    padding: 0 var(--space-2);
    border: var(--border-width) solid transparent;
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text);
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
  }

  .title-input:hover,
  .title-input:focus {
    border-color: var(--color-border-strong);
    background: var(--color-surface);
  }

  .muted {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .marker {
    flex: none;
    min-width: var(--control-height-sm);
    min-height: var(--control-height);
    display: grid;
    place-items: center;
    color: var(--color-text-muted);
    font-weight: var(--weight-bold);
  }

  .error {
    margin: 0;
    color: var(--color-danger);
  }

  .items {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .card {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-3);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .card:has(.enc) {
    flex-direction: column;
  }

  .enc,
  .stats,
  .buttons {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }

  .name {
    font-size: var(--text-md);
  }

  .tag {
    padding: 2px var(--space-2);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .band {
    color: var(--band);
    font-weight: var(--weight-bold);
  }

  .mono {
    font: var(--text-dice);
  }

  .btn {
    text-decoration: none;
  }

  .btn-ready {
    border-color: var(--color-accent);
  }

  .fields {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .inline {
    width: 100%;
    padding: var(--space-1) var(--space-2);
    border: var(--border-width) solid transparent;
    border-radius: var(--radius-sm);
    background: none;
    color: inherit;
    font: inherit;
  }

  .inline:hover,
  .inline:focus {
    border-color: var(--color-border-strong);
    background: var(--color-surface);
  }

  .title {
    font-weight: var(--weight-bold);
  }

  .body {
    resize: vertical;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .item.done {
    color: var(--color-text-muted);
  }

  .item.done .title {
    text-decoration: line-through;
  }

  .tick {
    display: grid;
    place-items: center;
    flex: none;
    width: var(--control-height-sm);
    height: var(--control-height-sm);
    margin-top: var(--space-1);
    padding: 0;
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    cursor: pointer;
  }

  .tick[aria-pressed="true"] {
    border-color: var(--color-accent);
    background: var(--color-accent-soft);
  }

  .empty {
    margin: 0;
    padding: var(--space-5);
    border: var(--border-width) dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    color: var(--color-text-muted);
    text-align: center;
  }

  .add {
    display: flex;
    gap: var(--space-2);
    padding-top: var(--space-3);
    border-top: var(--border-width) solid var(--color-border);
  }

  .add input {
    flex: 1;
    min-width: 0;
  }
</style>
