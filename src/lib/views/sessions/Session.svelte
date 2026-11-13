<script lang="ts">
  // Sessions → one session: the header (party collage, number and title, date, time, players, party note, counts, Run
  // session) stays put; the scenes, the editor and Across session each scroll inside their own column. Every change
  // saves itself a second after the last one, and before leaving. A new session (id null) is created at once, then
  // opened at its own address.
  import { beforeNavigate, goto } from "$app/navigation";
  import { page } from "$app/state";
  import { tick } from "svelte";
  import HexCollage from "$lib/components/HexCollage.svelte";
  import { moveTo } from "$lib/components/sidebarOrder";
  import { pictureSrc } from "$lib/api/campaigns";
  import { catalogRows, type CatalogRow } from "$lib/api/catalog";
  import { listCharacters, type Character } from "$lib/api/characters";
  import { encounterBudgets, listEncounters, type Budget, type Encounter } from "$lib/api/encounters";
  import { deleteSession, getSession, listSessions, saveSession, type Session } from "$lib/api/sessions";
  import { errorMessage } from "$lib/api/storage";
  import { closeWindow, onCloseRequested } from "$lib/api/window";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { gotoReplacing, isReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { showHeaderAction, showHeaderBack, showHeaderLead } from "$lib/stores/header.svelte";
  import { strings } from "$lib/strings";
  import { encounterCard } from "$lib/views/encounters/card";
  import { encounterParty, rowsByKey } from "$lib/views/encounters/roster";
  import Checklist from "./Checklist.svelte";
  import SessionWorldEntry from "./SessionWorldEntry.svelte";
  import SceneEditor from "./SceneEditor.svelte";
  import SceneList from "./SceneList.svelte";
  import SessionAside from "./SessionAside.svelte";
  import { askRunAgain, checkCounts, linkedIn, linkTo, newScene, nextNumber, readyToRun, type Linkable, type Pane, type Section } from "./sessions";
  import { WorldDraft } from "./worldDraft.svelte";

  /** The saved session to open, or null for a new one. */
  let { id }: { id: string | null } = $props();

  const t = strings.sessions;
  const e = strings.encounters;
  const campaign = campaigns.current!.id;
  const base = `/campaign/${campaign}/sessions`;
  showHeaderBack(() => base);
  const encountersBase = `/campaign/${campaign}/encounters`;
  const AUTOSAVE_MS = 1000;

  const blank = (number: number): Session => ({
    id: "",
    number,
    title: t.newTitle,
    date: null,
    time: null,
    partyNote: "",
    previously: "",
    premise: "",
    summary: "",
    started: null,
    ended: null,
    pausedAt: null,
    pausedMs: 0,
    playedScenes: null,
    playedSceneIds: [],
    currentSceneId: null,
    log: [],
    logAddedUntil: null,
    scenes: [],
    checklist: [],
    updated: 0,
  });

  // ---- Loading -----------------------------------------------------------------------------------
  let loadError = $state<string | null>(null);
  let draft = $state<Session>(blank(1));
  /** What was last saved (or loaded), to tell whether there are changes. Null until it has loaded. */
  let baseline = $state<string | null>(null);
  const snapshot = () => JSON.stringify({ ...draft, updated: 0 });
  /** The campaign's locations and NPCs, which the scenes link; saved with the session, and first. */
  const world = new WorldDraft(campaign);
  const loaded = $derived(!loadError && !world.loadError && baseline !== null && world.world !== null);
  showHeaderLead(() => (loaded ? deleteControls : null));
  showHeaderAction(() => (loaded ? runControls : null));
  const npcCount = $derived(linkedIn(draft, "npc").filter((nid) => world.world?.npcs.some((n) => n.id === nid)).length);

  /** Left already: a new session created after that doesn't take you back to it. */
  let gone = false;
  $effect(() => () => (gone = true));

  // svelte-ignore state_referenced_locally
  if (id) {
    getSession(campaign, id).then(
      (opened) => {
        draft = opened;
        selected = opened.scenes[0]?.id ?? null;
        baseline = snapshot();
      },
      (err) => (loadError = t.openError(errorMessage(err))),
    );
  } else {
    listSessions(campaign)
      .then((l) => saveSession(campaign, blank(nextNumber(l.sessions))))
      .then(
        (created) => (gone ? undefined : gotoReplacing(`${base}/${created.id}`)),
        (err) => (loadError = t.openError(errorMessage(err))),
      );
  }

  // What the encounters and the party need; without it they show no rating or no one.
  let all = $state<Encounter[] | null>(null);
  let encounterError = $state<string | null>(null);
  let characters = $state<Character[] | null>(null);
  let monsters = $state<CatalogRow[]>([]);
  let budgets = $state<Budget[]>([]);
  listEncounters(campaign).then(
    (l) => (all = l.encounters),
    (err) => (encounterError = t.encounters.loadError(errorMessage(err))),
  );
  listCharacters(campaign).then((l) => (characters = l.characters), () => {});
  catalogRows("monsters", ["xp", "image"]).then((rows) => (monsters = rows), () => {});
  encounterBudgets().then((b) => (budgets = b), () => {});

  const byKey = $derived(rowsByKey(monsters));
  const characterById = $derived(new Map((characters ?? []).map((c) => [c.id, c])));
  const party = $derived(encounterParty("party", [], characters).flatMap((cid) => characterById.get(cid) ?? [])); // as in Prepare
  const ready = $derived(all ? readyToRun(all) : null);
  const fmt = new Intl.NumberFormat();
  /** As the Encounters screen shows them. */
  const cardById = $derived(new Map((all ?? []).map((x) => [x.id, encounterCard(x, byKey, characters, budgets)])));

  const encounterName = (eid: string) => cardById.get(eid)?.name ?? (all === null ? t.loading : t.encounters.missing);
  /** Just the difficulty, for the Encounters section. */
  function encounterBand(eid: string): string | null {
    const card = cardById.get(eid);
    if (!card) return null;
    return card.band ? e.meter.bands[card.band] : e.meter.noRating;
  }

  // ---- What the editor shows ---------------------------------------------------------------------
  /** The open scene, unless a full view (`pane`) is open instead. */
  let selected = $state<string | null>(null);
  let pane = $state<Pane | null>(null);
  /** The location or NPC open in the editor (`pane` "locations" or "npcs"). */
  let worldEditing = $state<string | null>(null);
  /** The scene editor's open tab (kept when another scene opens, or a location or NPC editor takes its place). */
  let sceneTab = $state("text");
  let section = $state<Section | null>(null);
  const index = $derived(pane ? -1 : draft.scenes.findIndex((s) => s.id === selected));
  const counts = $derived(checkCounts(draft));

  function openScene(sid: string) {
    selected = sid;
    pane = null;
  }

  function addScene() {
    const scene = newScene();
    draft.scenes.push(scene);
    openScene(scene.id);
  }

  /** Opens a location or NPC in the editor, with the cursor in its name. */
  async function editWorld(kind: "location" | "npc", wid: string) {
    [pane, worldEditing] = [kind === "location" ? "locations" : "npcs", wid];
    await tick();
    const field = document.getElementById(`${kind === "location" ? "loc" : "npc"}-name-${wid}`) as HTMLInputElement | null;
    field?.focus();
    field?.select();
  }

  /** Back to the scene, with the focus on the entry's pencil: its card's in the scene, else its tree's when open. */
  async function closeWorld() {
    const wid = worldEditing;
    [pane, worldEditing] = [null, null];
    await tick();
    (document.querySelector<HTMLElement>(`main [data-edit="${wid}"]`) ?? document.querySelector<HTMLElement>(`[data-edit="${wid}"]`))?.focus();
  }

  function removeScene() {
    const at = index;
    if (at < 0) return;
    draft.scenes.splice(at, 1);
    selected = draft.scenes[Math.max(0, at - 1)]?.id ?? null;
  }

  // ---- Drag and drop -----------------------------------------------------------------------------
  let dragged = $state<{ item: Linkable; name: string } | null>(null);
  let overEditor = $state(false);
  let lastDrop = $state("");

  function startDrag(item: Linkable, name: string) {
    dragged = { item, name };
    lastDrop = "";
  }

  function endDrag() {
    dragged = null;
    overEditor = false;
  }

  /** Adds what's dragged to the scene, opens it and says so. */
  function dropOn(sid: string) {
    const scene = draft.scenes.find((s) => s.id === sid);
    if (dragged && scene) {
      linkTo(scene, dragged.item);
      openScene(sid);
      lastDrop = t.scenes.added(dragged.name, scene.title || t.scenes.untitled);
    }
    endDrag();
  }

  // ---- Saving and leaving ------------------------------------------------------------------------
  let saving = $state(false);
  let saveError = $state<string | null>(null);
  const valid = $derived(Number.isInteger(draft.number) && draft.number >= 1);
  const sessionChanged = $derived(baseline !== null && snapshot() !== baseline);
  const changed = $derived(sessionChanged || world.changed);
  let leaving = false;

  // Renaming: the pencil shows the number and title fields; Enter or leaving them keeps the change, Escape puts back
  // what was there (as does a number that isn't 1 or more).
  let renaming = $state(false);
  let before = { number: 1, title: "" };
  let numberInput: HTMLInputElement | undefined = $state();
  let pencil: HTMLButtonElement | undefined = $state();

  async function startRename() {
    before = { number: draft.number, title: draft.title };
    renaming = true;
    await tick();
    numberInput?.select();
  }

  async function endRename(keep: boolean) {
    if (!renaming) return;
    if (!keep) draft.title = before.title;
    if (!keep || !valid) draft.number = before.number;
    renaming = false;
    await tick();
    pencil?.focus();
  }

  let inflight: Promise<boolean> | null = null;
  /** What last failed to save: not tried again until something changes. */
  let failed = $state<string | null>(null);

  /** Saves the changes (after a save that's under way); false when it couldn't (the error shows). The world goes
   *  first: a session saved before a location it links would lose the link. */
  async function save(): Promise<boolean> {
    while (inflight) await inflight;
    if (!changed) return true;
    if (world.changed && !(await world.save())) {
      saveError = world.saveError;
      return false;
    }
    if (!sessionChanged) {
      saveError = null;
      return true;
    }
    if (!valid) {
      saveError = t.saveError(t.number);
      return false;
    }
    const sent = snapshot();
    saving = true;
    inflight = saveSession(campaign, { ...draft, title: draft.title.trim() || t.newTitle }).then(
      (saved) => {
        draft.updated = saved.updated;
        baseline = sent; // typed on while it saved: those changes are still unsaved, and save next
        [saveError, failed] = [null, null];
        return true;
      },
      (err) => {
        [saveError, failed] = [t.saveError(errorMessage(err)), sent];
        return false;
      },
    );
    try {
      return await inflight;
    } finally {
      [inflight, saving] = [null, false];
    }
  }

  // A second after the last change; not while renaming (a number half typed may be another session's).
  $effect(() => {
    const due = (sessionChanged && snapshot() !== failed) || world.due;
    if (!due || saving || world.saving || renaming || !valid) return;
    const timer = setTimeout(() => void save(), AUTOSAVE_MS);
    return () => clearTimeout(timer);
  });

  let leaveDialog: HTMLDialogElement | undefined = $state();
  let resolveLeave: ((leave: boolean) => void) | null = null;

  /** Saves before leaving; when that fails, asks whether to leave anyway (true) or stay. */
  async function saveBeforeLeaving(): Promise<boolean> {
    if (await save()) return true;
    leaveDialog?.showModal();
    return new Promise((resolve) => (resolveLeave = resolve));
  }

  // Escape closes the dialog too, which counts as Stay.
  function answerLeave(leave: boolean) {
    const resolve = resolveLeave;
    resolveLeave = null;
    if (leaveDialog?.open) leaveDialog.close();
    resolve?.(leave);
  }

  // Leaving saves first (Back included).
  beforeNavigate((navigation) => {
    if (leaving || !changed || !navigation.to || navigation.type === "leave") return;
    const replace = isReplacing();
    navigation.cancel();
    void saveBeforeLeaving().then((ok) => {
      if (!ok) return;
      leaving = true;
      if (navigation.type === "popstate") history.go(navigation.delta ?? -1);
      else void (replace ? gotoReplacing : goto)(navigation.to!.url.href);
    });
  });

  // Closing the window is leaving too (SvelteKit's "leave" can't be held).
  $effect(() => {
    const unlisten = onCloseRequested(async (event) => {
      if (leaving || !changed) return;
      event.preventDefault();
      if (!(await saveBeforeLeaving())) return;
      leaving = true;
      await closeWindow();
    });
    return () => void unlisten.then((stop) => stop());
  });

  // ---- Delete ------------------------------------------------------------------------------------
  let confirming = $state(false);
  let deleting = $state(false);

  async function askDelete() {
    confirming = true;
    await tick();
    document.getElementById("session-keep")?.focus();
  }

  async function keep() {
    confirming = false;
    await tick();
    document.getElementById("session-delete")?.focus();
  }

  async function remove() {
    deleting = true;
    saveError = null;
    try {
      await deleteSession(campaign, draft.id);
    } catch (err) {
      saveError = t.deleteError(draft.title, errorMessage(err));
      deleting = false;
      return;
    }
    leaving = true; // nothing left to save
    await gotoReplacing(base);
  }
</script>

<!-- Next to the shell's Back: deleting the session. -->
{#snippet deleteControls()}
  <div class="bar">
    {#if confirming}
      <p class="confirm" role="alert">{t.confirmDelete(draft.title.trim() || t.newTitle)}</p>
      <button id="session-keep" class="btn" onclick={keep}>{t.cancel}</button>
      <button class="btn btn-danger" disabled={deleting} onclick={remove}>{t.delete}</button>
    {:else}
      <button id="session-delete" class="btn delete" onclick={askDelete}>
        <i class="ti ti-trash" aria-hidden="true"></i>
        {t.delete}
      </button>
    {/if}
  </div>
{/snippet}

<!-- Right of the shell's search bar: the save status and Run session. -->
{#snippet runControls()}
  <div class="bar">
    {#if saveError}
      <span class="error small" role="alert">{saveError}</span>
    {:else if saving || world.saving}
      <span class="muted small" role="status">{t.saving}</span>
    {/if}
    <!-- A Played session can be run again: it carries on where it ended. -->
    <a class="btn btn-primary run" href="{base}/{draft.id}/run" onclick={() => askRunAgain(draft.id)}>
      {t.run}
      <i class="ti ti-player-play" aria-hidden="true"></i>
    </a>
  </div>
{/snippet}

<svelte:head><title>{draft.title.trim() || t.newTitle}</title></svelte:head>

{#if loadError || world.loadError}
  <div class="status">
    <p class="error" role="alert">{loadError ?? world.loadError}</p>
  </div>
{:else if baseline === null || world.world === null}
  <p class="status muted" role="status">{t.loading}</p>
{:else}
  <div class="session">
    <header class="head">
      {#if party.length > 0}
        <div class="party" aria-label={t.party} role="img">
          <!-- The party's pictures as hexagons, the same collage as the Characters screen's party band. -->
          <HexCollage
            tiles={party.map((c) => ({ key: c.id, picture: c.picture ? pictureSrc(c.picture) : null, letter: c.name.charAt(0) }))}
            seed={campaign}
            hexWidth="var(--session-hex-width)"
            maxRows={2.5}
          />
        </div>
      {/if}
      <div class="who">
        {#if renaming}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="rename"
            onkeydown={(ev) => {
              if (ev.key === "Enter") endRename(true);
              else if (ev.key === "Escape") endRename(false);
            }}
            onfocusout={(ev) => {
              if (!ev.currentTarget.contains(ev.relatedTarget as Node | null)) endRename(true);
            }}
          >
            <span>{t.session}</span>
            <input bind:this={numberInput} class="inline num" type="number" min="1" step="1" bind:value={draft.number} aria-label={t.number} aria-invalid={!valid} />
            <input class="inline grow" bind:value={draft.title} maxlength="100" autocomplete="off" placeholder={t.newTitle} aria-label={t.titleField} />
          </div>
        {:else}
          <h1>
            {t.session} {draft.number} — {draft.title || t.newTitle}
            <button bind:this={pencil} class="icon-btn" aria-label={t.rename} onclick={startRename}>
              <i class="ti ti-pencil" aria-hidden="true"></i>
            </button>
          </h1>
        {/if}
        <div class="when">
          <label>
            <span class="sr-only">{t.date}</span>
            <input class="inline" type="date" bind:value={() => draft.date ?? "", (v) => (draft.date = v || null)} />
          </label>
          <label>
            <span class="sr-only">{t.time}</span>
            <input class="inline" type="time" bind:value={() => draft.time ?? "", (v) => (draft.time = v || null)} />
          </label>
          <span>{t.players(party.length)}</span>
        </div>
      </div>
      <dl class="counts">
        <div>
          <dt>{t.counts.scenes}</dt>
          <dd>{fmt.format(draft.scenes.length)}</dd>
        </div>
        <div>
          <dt>{t.counts.npcs}</dt>
          <dd>{fmt.format(npcCount)}</dd>
        </div>
        <div>
          <dt>{t.counts.checklist}</dt>
          <dd>{fmt.format(counts.done)}<span class="of">/{fmt.format(counts.total)}</span></dd>
        </div>
      </dl>
    </header>

    <div class="columns">
      <div class="col left">
        <div class="scenes" use:keepScroll={{ id: "session-scenes", url: page.url.href, ready: true }}>
          <SceneList session={draft} world={world.world} selected={pane ? null : selected} dragging={dragged !== null} onselect={openScene} onadd={addScene} ondrop={dropOn} onmove={(from, to) => (draft.scenes = moveTo(draft.scenes, from, to))} />
        </div>
        <div class="linkables" use:keepScroll={{ id: "session-linkables", url: page.url.href, ready: true }}>
          <SessionAside
            bind:session={draft}
            keys={["locations", "npcs", "encounters"]}
            heading={false}
            bind:world={() => world.world!, (v) => (world.world = v)}
            bind:open={section}
            {lastDrop}
            {ready}
            {encounterError}
            encountersHref={encountersBase}
            encounterMeta={encounterBand}
            onmanage={(p) => ([pane, worldEditing] = [p, null])}
            onedit={editWorld}
            ondragstart={startDrag}
            ondragend={endDrag}
          />
        </div>
      </div>
      <main
        class="col editor"
        class:target={dragged !== null && !pane && index >= 0}
        class:over={overEditor}
        aria-label={draft.title || t.newTitle}
        use:keepScroll={{ id: "session-editor", url: page.url.href, ready: true }}
        ondragover={(ev) => {
          if (!dragged || index < 0) return;
          ev.preventDefault();
          overEditor = true;
        }}
        ondragleave={(ev) => {
          if (!ev.currentTarget.contains(ev.relatedTarget as Node | null)) overEditor = false;
        }}
        ondrop={(ev) => {
          ev.preventDefault();
          if (index >= 0) dropOn(draft.scenes[index].id);
        }}
      >
        {#if pane}
          <div class="pane">
            {#if (pane === "locations" || pane === "npcs") && worldEditing}
              <SessionWorldEntry kind={pane === "locations" ? "location" : "npc"} id={worldEditing} onclose={closeWorld} bind:session={draft} bind:world={() => world.world!, (v) => (world.world = v)} />
            {:else}
              <h2>{t.sections.checklist}</h2>
              <Checklist bind:items={draft.checklist} placeholder={t.checklist.placeholder} />
            {/if}
          </div>
        {:else if index >= 0}
          <SceneEditor bind:current={sceneTab} bind:session={draft} bind:world={() => world.world!, (v) => (world.world = v)} {index} {encounterName} encounterCard={(eid) => cardById.get(eid)} encountersHref={encountersBase} encounterChoices={ready ?? []} onedit={editWorld} onremove={removeScene} />
        {:else}
          <p class="muted">{t.scenes.empty}</p>
        {/if}
      </main>
      <div class="col" use:keepScroll={{ id: "session-aside", url: page.url.href, ready: true }}>
        <SessionAside
          bind:session={draft}
          keys={["previously", "premise", "partyNote", "checklist"]}
          bind:world={() => world.world!, (v) => (world.world = v)}
          bind:open={section}
          {lastDrop}
          {ready}
          {encounterError}
          encountersHref={encountersBase}
          encounterMeta={encounterBand}
          onmanage={(p) => ([pane, worldEditing] = [p, null])}
          onedit={editWorld}
          ondragstart={startDrag}
          ondragend={endDrag}
        />
      </div>
    </div>
  </div>
{/if}

<dialog bind:this={leaveDialog} class="dialog leave" aria-labelledby="leave-title" aria-describedby="leave-reason" onclose={() => answerLeave(false)}>
  <h2 id="leave-title">{t.leave.title}</h2>
  <p id="leave-reason">{saveError}</p>
  <div class="actions">
    <button class="btn" onclick={() => answerLeave(true)}>{t.leave.leave}</button>
    <!-- svelte-ignore a11y_autofocus: the safe answer is the default -->
    <button class="btn btn-primary" autofocus onclick={() => answerLeave(false)}>{t.leave.stay}</button>
  </div>
</dialog>

<style>
  .status {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-5);
  }

  /* Nothing goes below the fold: the header stays, each column is the rest of the window's height and scrolls inside. */
  .session {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    height: 100%;
    padding: var(--space-4) var(--space-4) var(--space-5);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .delete {
    color: var(--color-danger);
  }

  .confirm {
    margin: 0;
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-5);
    padding: var(--space-4) var(--space-5);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  /* Two and a half hexagons high at most, like the party band; it grows sideways instead. */
  .party {
    flex: none;
    display: flex;
    height: calc(2.44 * var(--session-hex-width));
  }

  .who {
    flex: 1 1 var(--search-width);
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  h1 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    font: var(--text-h1);
    letter-spacing: var(--tracking-tight);
  }

  .rename {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font: var(--text-h2);
  }

  .rename .grow {
    flex: 1;
    min-width: 0;
  }

  .inline {
    min-height: var(--control-height);
    padding: 0 var(--space-2);
    border: var(--border-width) solid transparent;
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text);
    font: inherit;
  }

  .rename .num {
    width: calc(var(--control-height-xl) + var(--space-5));
  }

  .inline:hover,
  .inline:focus {
    border-color: var(--color-border-strong);
    background: var(--color-surface);
  }

  .inline[aria-invalid="true"] {
    border-color: var(--color-danger);
  }

  .when {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .when .inline {
    font-size: var(--text-sm);
  }

  .when > * + *::before {
    content: "·" / ""; /* decorative separator */
    margin-right: var(--space-2);
    font-weight: var(--weight-bold);
  }

  .counts {
    display: flex;
    gap: var(--space-6);
    margin: 0;
  }

  .counts div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .counts dt {
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .counts dd {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
  }

  .of {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .run {
    min-height: var(--control-height-lg);
    text-decoration: none;
  }

  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: var(--session-steps-width) minmax(0, 1fr) var(--session-aside-width);
    grid-template-rows: minmax(0, 1fr);
    gap: var(--space-4);
  }

  .col {
    min-width: 0;
    overflow-y: auto;
  }

  /* The scenes fill the column down to Locations, NPCs and Encounters, and scroll after that; an open section grows
     up over them (to most of the column, then it scrolls itself). */
  .left {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    overflow: hidden;
  }

  .scenes {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
  }

  .linkables {
    flex: none;
    max-height: 75%;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    padding: var(--space-5);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  /* While a card is dragged, the open scene is outlined (dashed, easing in and out), and tinted under the pointer. */
  .editor {
    outline: var(--focus-ring) dashed transparent;
    outline-offset: calc(-1 * var(--focus-ring));
    transition:
      outline-color var(--duration-hover) var(--ease-hover),
      background-color var(--duration-hover) var(--ease-hover);
  }

  .editor.target {
    outline-color: var(--color-accent);
  }

  .editor.over {
    background: var(--color-accent-soft);
  }

  :global([data-motion="reduce"]) .editor {
    transition: none;
  }

  /* What the full views (locations, NPCs, checklist) share; the scene editor is outside, so none of it reaches there. */
  .pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .pane :global(h2) {
    margin: 0;
    font: var(--text-h2);
  }

  .pane :global(.card) {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  .pane :global(.row) {
    display: flex;
    align-items: flex-end;
    gap: var(--space-2);
  }

  .pane :global(.grow) {
    flex: 1;
    min-width: 0;
  }

  .pane :global(.field) {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .pane :global(.pair) {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-2);
  }

  .pane :global(.small-label) {
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
  }

  .pane :global(.strong) {
    font-weight: var(--weight-bold);
  }

  .pane :global(textarea) {
    resize: vertical;
    font: inherit;
  }

  .pane :global(.btn.icon) {
    justify-content: center;
    width: var(--control-height);
    padding: 0;
  }

  .pane :global(.note) {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .pane :global(.add) {
    display: flex;
    gap: var(--space-2);
    padding-top: var(--space-4);
    border-top: var(--border-width) solid var(--color-border);
  }

  .pane :global(.add input) {
    flex: 1;
    min-width: 0;
  }

  .muted {
    color: var(--color-text-muted);
  }

  .leave .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .small {
    font-size: var(--text-sm);
  }

  .error {
    color: var(--color-danger);
  }

  /* A narrow window: one column that scrolls as a whole, as in Prepare. */
  @media (max-width: 900px) {
    .session {
      height: auto;
    }

    .columns {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: none;
    }

    .col,
    .scenes,
    .linkables {
      overflow: visible;
    }

    .linkables {
      max-height: none;
    }
  }
</style>
