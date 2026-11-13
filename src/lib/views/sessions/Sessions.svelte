<script lang="ts">
  // Sessions overview (docs/Sessions-html): the campaign header; Up next (the lowest-numbered session not played), the
  // other unplayed ones (Later), Planned only when nothing is left to play, and the Played ones; beside them the
  // campaign's Locations and NPCs trees. Selecting an entry highlights the sessions it appears in. Edits to the trees
  // save themselves a second after the last one, and before leaving.
  import { beforeNavigate, goto } from "$app/navigation";
  import { page } from "$app/state";
  import { listCharacters, type Character } from "$lib/api/characters";
  import { getSession, listSessions, saveSession, type Session, type SessionList, type SessionRow } from "$lib/api/sessions";
  import type { Location, Npc } from "$lib/api/world";
  import { errorMessage } from "$lib/api/storage";
  import { closeWindow, onCloseRequested } from "$lib/api/window";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { gotoReplacing, isReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";
  import { characterLevel, encounterParty } from "$lib/views/encounters/roster";
  import { tick } from "svelte";
  import SummaryDialog from "./SummaryDialog.svelte";
  import WorldEntry from "./WorldEntry.svelte";
  import WorldTree from "./WorldTree.svelte";
  import { ancestors, averageLevel, descendants, overview, parseDate, parseTime, removeLocation, removeNpc } from "./sessions";
  import { WorldDraft } from "./worldDraft.svelte";

  const t = strings.sessions;
  const o = strings.sessions.overview;
  const campaign = campaigns.current!.id;
  const base = `/campaign/${campaign}/sessions`;
  const AUTOSAVE_MS = 1000;
  const fmt = new Intl.NumberFormat();
  const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });
  const longDate = new Intl.DateTimeFormat(undefined, { dateStyle: "full" });
  const timeFormat = new Intl.DateTimeFormat(undefined, { timeStyle: "short" });
  const hours = new Intl.NumberFormat(undefined, { style: "unit", unit: "hour", unitDisplay: "narrow" });
  const minutes = new Intl.NumberFormat(undefined, { style: "unit", unit: "minute", unitDisplay: "narrow" });
  const names = new Intl.ListFormat(undefined, { type: "conjunction" });

  // ---- Loading -----------------------------------------------------------------------------------
  let list = $state<SessionList | null>(null);
  let loadError = $state<string | null>(null);
  let characters = $state<Character[] | null>(null);
  listSessions(campaign).then(
    (l) => (list = l),
    (e) => (loadError = t.loadError(errorMessage(e))),
  );
  listCharacters(campaign).then((l) => (characters = l.characters), () => {}); // without it, no party line
  const world = new WorldDraft(campaign);

  const parts = $derived(overview(list?.sessions ?? []));

  // ---- A Played session's summary: its pencil opens it, with the session notes beside it ---------------------
  let summaryDialog: SummaryDialog | undefined = $state();
  let summaryOf = $state<Session | null>(null);
  let summaryError = $state<string | null>(null);

  async function editSummary(id: string) {
    [summaryOf, summaryError] = [null, null];
    try {
      summaryOf = await getSession(campaign, id);
    } catch (e) {
      summaryError = t.openError(errorMessage(e));
      return;
    }
    await tick();
    summaryDialog?.open(summaryOf.summary);
  }

  async function saveSummary(summary: string): Promise<boolean> {
    if (!summaryOf) return false;
    try {
      await saveSession(campaign, { ...summaryOf, summary: summary.trim() });
      list = await listSessions(campaign);
      return true;
    } catch (e) {
      summaryError = t.saveError(errorMessage(e));
      return false;
    }
  }
  const party = $derived(encounterParty("party", [], characters).flatMap((id) => characters?.find((c) => c.id === id) ?? []));
  const level = $derived(averageLevel(party.flatMap((c) => characterLevel(c.classes) ?? [])));

  // ---- Editing a location or NPC: in place of the sessions -----------------------------------------
  let editing = $state<{ kind: "location" | "npc"; id: string } | null>(null);
  /** A tree's entries; the NPCs' with their groups. */
  const treeOf = (kind: "location" | "npc"): { id: string; parent: string | null; name: string }[] =>
    !world.world ? [] : kind === "location" ? world.world.locations : [...world.world.npcGroups, ...world.world.npcs];
  const editEntries = $derived<(Location | Npc)[]>(!editing || !world.world ? [] : editing.kind === "location" ? world.world.locations : world.world.npcs);
  const editTree = $derived(editing ? treeOf(editing.kind) : []);
  const editEntry = $derived(editing ? editEntries.find((e) => e.id === editing!.id) : undefined);
  const editPath = $derived(editEntry ? ancestors(editTree, editEntry.id).map((a) => a.name || unnamedOf(editing!.kind)) : []);
  const unnamedOf = (kind: "location" | "npc") => (kind === "location" ? t.manage.unnamedLocation : t.manage.unnamedNpc);

  /** Opens the entry in the editor, with the cursor in its name. */
  async function edit(kind: "location" | "npc", id: string) {
    editing = { kind, id };
    await tick();
    const field = document.getElementById(`${kind === "location" ? "loc" : "npc"}-name-${id}`) as HTMLInputElement | null;
    field?.focus();
    field?.select();
  }

  /** Back to the sessions, with the focus on the entry's pencil (or its tree's search, when the pencil is hidden). */
  async function closeEditor() {
    const { kind, id } = editing!;
    editing = null;
    await tick();
    (document.querySelector<HTMLElement>(`[data-edit="${id}"]`) ?? document.getElementById(`${kind}-search`))?.focus();
  }

  function removeEdited(id: string) {
    (editing!.kind === "location" ? removeLocation : removeNpc)(world.world!, null, id);
    if (selected?.id === id) selected = null;
    editing = null;
  }

  // ---- Highlighting ------------------------------------------------------------------------------
  let selected = $state<{ kind: "location" | "npc"; id: string } | null>(null);
  const selectedEntry = $derived.by(() => {
    if (!selected || !world.world) return null;
    const entries = treeOf(selected.kind);
    const entry = entries.find((e) => e.id === selected!.id);
    return entry ? { name: entry.name || (selected.kind === "location" ? t.manage.unnamedLocation : t.manage.unnamedNpc), ids: descendants(entries, entry.id) } : null;
  });
  /** Appears here (true), dimmed (false), or nothing selected (null). */
  function hit(s: SessionRow): boolean | null {
    if (!selected || !selectedEntry) return null;
    return (selected.kind === "location" ? s.locationIds : s.npcIds).some((id) => selectedEntry.ids.includes(id));
  }

  // ---- Showing -----------------------------------------------------------------------------------
  const whenText = (date: string | null, time: string | null, format = dateFormat) => {
    const d = parseDate(date);
    const at = parseTime(time);
    return d ? [format.format(d), at && timeFormat.format(at)].filter(Boolean).join(" · ") : t.noDate;
  };
  function duration(ms: number): string {
    const total = Math.round(ms / 60_000);
    const [h, m] = [Math.floor(total / 60), total % 60];
    return [h && hours.format(h), (m || !h) && minutes.format(m)].filter(Boolean).join(" ");
  }

  type Fact = { text: string; icon: string; label: string };
  const counts = (s: SessionRow): Fact[] => [
    { text: fmt.format(s.scenes), icon: "ti-masks-theater", label: o.scenes(s.scenes) },
    { text: fmt.format(s.npcIds.length), icon: "ti-users", label: o.npcCount(s.npcIds.length) },
    { text: fmt.format(s.encounters), icon: "ti-swords", label: o.encounters(s.encounters) },
    { text: fmt.format(s.locationIds.length), icon: "ti-map-pin-2", label: o.locationCount(s.locationIds.length) },
  ];
  const playedFacts = (s: SessionRow): Fact[] =>
    [
      s.playedScenes !== null && { text: fmt.format(s.playedScenes), icon: "ti-masks-theater", label: o.playedScenes(s.playedScenes) },
      s.playedMs !== null && { text: duration(s.playedMs), icon: "ti-clock", label: o.timePlayed },
      parseDate(s.date) && { text: dateFormat.format(parseDate(s.date)!), icon: "ti-calendar", label: t.date },
    ].filter((f) => !!f);

  // ---- Saving the trees --------------------------------------------------------------------------
  $effect(() => {
    if (!world.due) return;
    const timer = setTimeout(() => void world.save(), AUTOSAVE_MS);
    return () => clearTimeout(timer);
  });

  let leaving = false;
  // Leaving saves first; when that fails, the error shows and you stay.
  beforeNavigate((navigation) => {
    if (leaving || !world.changed || !navigation.to || navigation.type === "leave") return;
    const replace = isReplacing();
    navigation.cancel();
    void world.save().then((ok) => {
      if (!ok) return;
      leaving = true;
      if (navigation.type === "popstate") history.go(navigation.delta ?? -1);
      else void (replace ? gotoReplacing : goto)(navigation.to!.url.href);
    });
  });
  $effect(() => {
    const unlisten = onCloseRequested(async (event) => {
      if (leaving || !world.changed) return;
      event.preventDefault();
      if (!(await world.save())) return;
      leaving = true;
      await closeWindow();
    });
    return () => void unlisten.then((stop) => stop());
  });
</script>

<svelte:head><title>{t.title}</title></svelte:head>

{#snippet facts(items: Fact[])}
  <ul class="facts" aria-label={o.facts}>
    {#each items as f, i (i)}
      <li>
        <span aria-hidden="true">{f.text}</span>
        <i class="ti {f.icon}" aria-hidden="true"></i>
        <span class="sr-only">{f.label}{f.icon === "ti-clock" || f.icon === "ti-calendar" ? `: ${f.text}` : ""}</span>
      </li>
    {/each}
  </ul>
{/snippet}

{#snippet status(s: SessionRow)}
  <span class="status {s.status}">{o.status[s.status]}</span>
{/snippet}

{#snippet appears(s: SessionRow)}
  {#if hit(s)}<span class="appears">{o.appearsHere}</span>{/if}
{/snippet}

<div class="overview">
  <header class="head">
    <div class="who">
      <span class="label">{o.campaign}</span>
      <h1>{campaigns.current?.name}</h1>
      {#if party.length}
        <span class="muted">{[names.format(party.map((c) => c.name)), level !== null && o.level(level)].filter(Boolean).join(" · ")}</span>
      {/if}
    </div>
    <dl class="counts">
      <div>
        <dt class="label">{o.played}</dt>
        <dd>{list ? fmt.format(parts.played.length) : ""}</dd>
      </div>
      <div>
        <dt class="label">{o.locations}</dt>
        <dd>{world.world ? fmt.format(world.world.locations.length) : ""}</dd>
      </div>
      <div>
        <dt class="label">{o.npcs}</dt>
        <dd>{world.world ? fmt.format(world.world.npcs.length) : ""}</dd>
      </div>
    </dl>
    <a class="btn btn-primary" href="{base}/new">
      <i class="ti ti-plus" aria-hidden="true"></i>
      {t.create}
    </a>
  </header>

  <div class="body">
    {#if editing && editEntry}
      <section class="editor" aria-label={t.world.edit(editEntry.name || unnamedOf(editing.kind))}>
        {#if editPath.length}<span class="muted">{editPath.join(" › ")}</span>{/if}
        <WorldEntry kind={editing.kind} bind:world={() => world.world!, (v) => (world.world = v)} id={editEntry.id} onremove={removeEdited}>
          {#snippet end()}
            <button class="icon-btn icon-btn-lg" aria-label={t.world.close} onclick={closeEditor}>
              <i class="ti ti-x" aria-hidden="true"></i>
            </button>
          {/snippet}
        </WorldEntry>
      </section>
    {:else}
    <main class="sessions" aria-label={o.sessions} use:keepScroll={{ id: "sessions", url: page.url.href, ready: !!list }}>
      {#if selectedEntry}
        <div class="bar">
          <span class="chip">
            {o.highlighting(selectedEntry.name)}
            <button class="clear" aria-label={o.clearHighlight} onclick={() => (selected = null)}>
              <i class="ti ti-x" aria-hidden="true"></i>
            </button>
          </span>
        </div>
      {/if}

      {#if loadError}
        <p class="message error" role="alert">{loadError}</p>
      {:else if list === null}
        <p class="message" role="status">{t.loading}</p>
      {:else}
        {#if list.unreadable.length > 0}
          <p class="message error" role="alert">{t.unreadable(list.unreadable.join(", "))}</p>
        {/if}

        {#if parts.next}
          {@const s = parts.next}
          <section class="card next" class:hit={hit(s)} class:dim={hit(s) === false} aria-labelledby="next-title">
            <div class="next-head">
              <div class="who">
                <span class="label kicker">{o.upNext(s.number)} {@render status(s)} {@render appears(s)}</span>
                <h3 id="next-title">{s.title}</h3>
                <span class="muted">{whenText(s.date, s.time, longDate)}</span>
              </div>
              <div class="actions">
                <a class="btn" href="{base}/{s.id}">{o.continuePrep}</a>
                <a class="btn" href="{base}/{s.id}/run">
                  <i class="ti ti-player-play" aria-hidden="true"></i>
                  {t.run}
                </a>
              </div>
            </div>
            {#if s.premise}<p class="summary">{s.premise}</p>{/if}
            {@render facts(counts(s))}
          </section>
        {:else}
          <section class="planned" aria-labelledby="planned-title">
            <div class="who">
              <span class="label">{o.planned}</span>
              <strong id="planned-title">{o.nothingPlanned}</strong>
            </div>
            <a class="btn" href="{base}/new">{t.create}</a>
          </section>
        {/if}

        {#if parts.later.length}
          <ul class="rows" aria-label={o.later}>
            {#each parts.later as s (s.id)}
              <li class="row" class:hit={hit(s)} class:dim={hit(s) === false}>
                <strong class="grow">{fmt.format(s.number)} · {s.title}</strong>
                {@render appears(s)}
                <span class="muted small">{whenText(s.date, s.time)}</span>
                {@render status(s)}
                <a class="btn" href="{base}/{s.id}" aria-label={o.openSession(s.number, s.title)}>{o.open}</a>
              </li>
            {/each}
          </ul>
        {/if}

        {#if parts.played.length}
          <h2 class="label section">{o.played}</h2>
          {#each parts.played as s (s.id)}
            <article class="card played" class:hit={hit(s)} class:dim={hit(s) === false} aria-labelledby="played-{s.id}">
              <span class="hex" aria-hidden="true">{fmt.format(s.number)}</span>
              <div class="grow">
                <div class="line">
                  <strong id="played-{s.id}">{s.title}</strong>
                  {@render appears(s)}
                </div>
                {#if s.summary}<p class="summary small">{s.summary}</p>{/if}
                {@render facts(playedFacts(s))}
              </div>
              <!-- Shown while the card is hovered or has focus. -->
              <button class="icon-btn edit-summary" aria-label={o.editSummary(s.number, s.title)} onclick={() => editSummary(s.id)}>
                <i class="ti ti-pencil" aria-hidden="true"></i>
              </button>
              <a class="btn" href="{base}/{s.id}" aria-label={o.openSession(s.number, s.title)}>{o.open}</a>
            </article>
          {/each}
          {#if summaryError && !summaryOf}<p class="message error" role="alert">{summaryError}</p>{/if}
          <SummaryDialog bind:this={summaryDialog} notes={summaryOf?.log ?? []} saveLabel={strings.sessions.live.saveSummary} error={summaryError} onsave={saveSummary} />
        {/if}
      {/if}
    </main>
    {/if}

    {#if world.loadError}
      <p class="message error trees" role="alert">{world.loadError}</p>
    {:else if world.world === null}
      <p class="message trees" role="status">{t.loading}</p>
    {:else}
      <div class="trees">
        {#if world.saveError}<p class="message error" role="alert">{world.saveError}</p>{/if}
        {#each ["location", "npc"] as const as kind (kind)}
          <WorldTree
            {kind}
            bind:world={() => world.world!, (v) => (world.world = v)}
            selected={selected?.kind === kind ? selected.id : null}
            onselect={(id) => (selected = id ? { kind, id } : null)}
            onedit={(id) => edit(kind, id)}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  /* Nothing goes below the fold: the header stays, each column is the rest of the window's height and scrolls inside. */
  .overview {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5);
  }

  /* Links that look like buttons. */
  a.btn {
    text-decoration: none;
  }

  .label {
    margin: 0;
    color: var(--color-text-muted);
  }

  .muted {
    color: var(--color-text-muted);
    font: var(--text-body-sm);
  }

  .small {
    font-size: var(--text-sm);
  }

  .grow {
    flex: 1 1 auto;
    min-width: 0;
  }

  .who {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }

  /* Header: name and party, the counts, New session. */
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-6);
    padding: var(--space-5) var(--space-6);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .head .who {
    flex: 1 1 var(--search-width);
  }

  h1 {
    margin: 0;
    font: var(--text-h1);
  }

  .counts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-6);
    margin: 0;
  }

  .counts div {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .counts dd {
    margin: 0;
    font: var(--text-dice);
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
  }

  /* Sessions half the width, Locations and NPCs a quarter each; they stack in a narrow window. */
  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    grid-template-rows: minmax(0, 1fr);
    gap: var(--space-4);
  }

  /* The padding keeps a highlighted card's outline inside the scrolling column. */
  .sessions {
    grid-column: span 2;
    min-width: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--border-width);
  }

  /* The location or NPC being edited, in the sessions' place. */
  .editor {
    grid-column: span 2;
    min-width: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .trees {
    grid-column: span 2;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-template-rows: minmax(0, 1fr);
    align-items: start;
    gap: var(--space-4);
  }

  .trees > :global(*) {
    max-height: 100%;
  }

  .trees > .message {
    grid-column: 1 / -1;
  }

  /* A narrow window: one column that scrolls as a whole, as on the session page. */
  @media (max-width: 900px) {
    .overview {
      height: auto;
      padding-bottom: var(--corner-clearance);
    }

    .body {
      grid-template-rows: none;
    }

    .sessions,
    .editor,
    .trees {
      grid-column: 1 / -1;
      overflow: visible;
    }

    .trees {
      grid-template-rows: none;
    }

    .trees > :global(*) {
      max-height: none;
    }
  }

  @media (max-width: 600px) {
    .trees {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-1);
  }

  .section {
    padding: var(--space-2) var(--space-1) 0;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding-left: var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--color-accent-soft);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
  }

  .clear {
    display: grid;
    place-items: center;
    width: var(--control-height-sm);
    height: var(--control-height-sm);
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  /* A session: outlined when the selected entry appears in it, dimmed when it doesn't. */
  .card,
  .row {
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .hit {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 var(--border-width) var(--color-accent);
  }

  .dim {
    opacity: 0.55;
  }

  .next {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-5);
  }

  .next-head {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: var(--space-4);
  }

  .next-head .who {
    flex: 1 1 var(--search-width);
  }

  .kicker {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-accent);
  }

  h3 {
    margin: 0;
    font: var(--text-h2);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .summary {
    margin: 0;
    white-space: pre-line;
  }

  .facts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    margin: 0;
    padding: 0;
    list-style: none;
    color: var(--color-text-muted);
    font: var(--text-dice);
  }

  .facts li {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .facts li + li::before {
    content: "·";
    margin: 0 var(--space-2);
  }

  .status,
  .appears {
    padding: 0 var(--space-2);
    border-radius: var(--radius-sm);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    white-space: nowrap;
  }

  .status {
    border: var(--border-width) solid var(--color-border-strong);
    color: var(--color-text-muted);
  }

  .status.ready,
  .status.playing {
    border-color: var(--color-accent);
    color: var(--color-accent);
  }

  .appears {
    background: var(--color-accent-soft);
    color: var(--color-text);
  }

  .planned {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-5);
    border: var(--border-width) dashed var(--color-border-strong);
    border-radius: var(--radius-md);
  }

  .planned .who {
    flex: 1;
  }

  /* Apart from Up next, without a heading. */
  .rows {
    list-style: none;
    margin: var(--space-4) 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
  }

  .played {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
  }

  .played .grow {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .played > .btn,
  .edit-summary {
    align-self: center;
  }

  .edit-summary {
    opacity: 0;
  }

  .played:hover .edit-summary,
  .played:focus-within .edit-summary {
    opacity: 1;
  }

  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-2);
  }

  .hex {
    display: grid;
    place-items: center;
    flex: none;
    width: var(--roster-badge-width);
    height: var(--roster-badge-height);
    background: var(--color-ready-fill);
    color: var(--color-ready-text);
    font: var(--text-dice);
    font-weight: var(--weight-bold);
    clip-path: polygon(25% 0, 75% 0, 100% 50%, 75% 100%, 25% 100%, 0 50%);
  }

  .message {
    margin: 0;
    color: var(--color-text-muted);
  }

  .message.error {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-size: var(--text-sm);
  }
</style>
