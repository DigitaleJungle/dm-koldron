<script lang="ts">
  // Sessions → Run session: the live view at the table. The campaign header holds the live line, the time played and
  // the counts (beside Back), and Pause and End session (right); the party band stays put under it. The run sheet (with
  // the current scene's encounters below it), the current scene (tabs: Info with its checklist, Location, NPCs) and
  // Across session each scroll inside. Opening it starts the session (once; a Played one runs again only from Session
  // prep's Run session); every change is saved at once, so a crash loses nothing. End session opens the Summary popup.
  import { beforeNavigate } from "$app/navigation";
  import { tick } from "svelte";
  import { page } from "$app/state";
  import { pictureSrc } from "$lib/api/campaigns";
  import { catalogRows, type CatalogRow } from "$lib/api/catalog";
  import { listCharacters, type Character } from "$lib/api/characters";
  import { goLive } from "$lib/stores/addons.svelte";
  import { encounterBudgets, listEncounters, type Budget, type Encounter } from "$lib/api/encounters";
  import { getSession, saveSession, type Session } from "$lib/api/sessions";
  import { errorMessage } from "$lib/api/storage";
  import { getWorld, type Npc, type World } from "$lib/api/world";
  import { cleanHtml, isEmpty } from "$lib/components/richtext";
  import "$lib/styles/prose.css";
  import { campaigns, libraryHome } from "$lib/stores/campaigns.svelte";
  import { showHeaderAction, showHeaderBack, showHeaderLead } from "$lib/stores/header.svelte";
  import { canGoBack, gotoReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";
  import { BAND_ICONS, encounterCard } from "$lib/views/encounters/card";
  import SideTabs from "$lib/components/SideTabs.svelte";
  import NotesPanel from "$lib/views/encounters/NotesPanel.svelte";
  import { encounterParty, rowsByKey } from "$lib/views/encounters/roster";
  import Checklist from "./Checklist.svelte";
  import SummaryDialog from "./SummaryDialog.svelte";
  import { checkCounts, clock, currentScene, endSession, markPlayed, npcSpecs, pause, resume, startOrResume, summaryDraft, takeRunAgain, timePlayed } from "./sessions";

  let { id }: { id: string } = $props();

  const t = strings.sessions.live;
  const campaign = campaigns.current!.id;
  // svelte-ignore state_referenced_locally
  const prep = `/campaign/${campaign}/sessions/${id}`;
  const encountersBase = `/campaign/${campaign}/encounters`;
  showHeaderBack(() => prep);

  const fmt = new Intl.NumberFormat();
  const timeFormat = new Intl.DateTimeFormat(undefined, { timeStyle: "short" });
  const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "short" });
  const AUTOSAVE_MS = 300;

  // ---- Loading -----------------------------------------------------------------------------------
  let loadError = $state<string | null>(null);
  let session = $state<Session | null>(null);
  let world = $state<World | null>(null);
  let baseline = $state<string | null>(null);
  const snapshot = () => JSON.stringify({ ...session, updated: 0 });

  // svelte-ignore state_referenced_locally
  const again = takeRunAgain(id);
  // svelte-ignore state_referenced_locally
  Promise.all([getSession(campaign, id), getWorld(campaign)]).then(
    ([opened, w]) => {
      baseline = JSON.stringify({ ...opened, updated: 0 });
      // A Played session runs again only from Run session (not when Forward comes back here after End session).
      if (opened.ended === null || again) startOrResume(opened, Date.now());
      [session, world] = [opened, w];
    },
    (err) => (loadError = strings.sessions.openError(errorMessage(err))),
  );

  // What the encounters and the party need; without it they show no rating or no one.
  // Null until loaded, so a scene's encounters don't read as deleted meanwhile.
  let all = $state<Encounter[] | null>(null);
  let encounterError = $state<string | null>(null);
  let characters = $state<Character[] | null>(null);
  let monsters = $state.raw<CatalogRow[]>([]);
  let budgets = $state<Budget[]>([]);
  let lookupErrors = $state<string[]>([]);
  const lookupFailed = (text: (reason: string) => string) => (e: unknown) => lookupErrors.push(text(errorMessage(e)));
  listEncounters(campaign).then(
    (l) => (all = l.encounters),
    (e) => (encounterError = strings.sessions.encounters.loadError(errorMessage(e))),
  );
  listCharacters(campaign).then((l) => (characters = l.characters), lookupFailed(strings.encounters.party.loadError));
  // A running session is live (addons are told, e.g. to sync more often).
  $effect(goLive);
  catalogRows("monsters", ["xp", "image"]).then((rows) => (monsters = rows), lookupFailed(strings.encounters.list.lookupError));
  encounterBudgets().then((b) => (budgets = b), lookupFailed(strings.encounters.meter.budgetError));

  const byKey = $derived(rowsByKey(monsters));
  const characterById = $derived(new Map((characters ?? []).map((c) => [c.id, c])));
  const party = $derived(encounterParty("party", [], characters).flatMap((cid) => characterById.get(cid) ?? []));
  const npcById = $derived(new Map((world?.npcs ?? []).map((n) => [n.id, n])));
  const locationById = $derived(new Map((world?.locations ?? []).map((l) => [l.id, l])));
  const encounterById = $derived(new Map((all ?? []).map((e) => [e.id, e])));

  // ---- The current scene -------------------------------------------------------------------------
  const scene = $derived(session ? currentScene(session) : null);
  const index = $derived(session && scene ? session.scenes.indexOf(scene) : -1);
  const last = $derived(session !== null && index === session.scenes.length - 1);
  const played = (sid: string) => session?.playedSceneIds.includes(sid) ?? false;
  const location = $derived(scene?.locationId ? (locationById.get(scene.locationId) ?? null) : null);
  const banner = $derived(location ? (location.pictures?.[0] ?? location.map ?? null) : null);
  const sceneNpcs = $derived((scene?.npcIds ?? []).flatMap((nid) => npcById.get(nid) ?? []));
  const locationNpcs = $derived((location?.npcIds ?? []).flatMap((nid) => npcById.get(nid) ?? []));
  const sceneChecks = $derived(scene ? { done: scene.checklist.filter((c) => c.done).length, total: scene.checklist.length } : null);
  const counts = $derived(session ? checkCounts(session) : { done: 0, total: 0 });
  const untitled = (title: string) => title || strings.sessions.scenes.untitled;

  /** The scene panel's open tab, kept when another scene shows. */
  const TABS = [
    { id: "info", icon: "ti-message-2-check", label: t.info },
    { id: "location", icon: "ti-map-pin", label: t.location },
    { id: "npcs", icon: "ti-users", label: t.npcs },
  ] as const;
  let tab = $state("info");
  // ---- An NPC's details, in a dialog ------------------------------------------------------------
  let shownNpc = $state<Npc | null>(null);
  let npcDialog: HTMLDialogElement | undefined = $state();
  async function showNpc(n: Npc) {
    shownNpc = n;
    await tick();
    npcDialog?.showModal();
  }
  let section = $state<string | null>("previously");

  function go(i: number) {
    const next = session?.scenes[i];
    if (!session || !next) return;
    session.currentSceneId = next.id;
  }

  function log(text: string) {
    session?.log.unshift({ at: Date.now(), text });
  }

  /** Shows the next scene; being shown doesn't make a scene played. */
  function next() {
    if (!last) go(index + 1);
  }

  /** Ticks the scene played, or unticks it. */
  function togglePlayed() {
    if (!session || !scene) return;
    if (played(scene.id)) session.playedSceneIds = session.playedSceneIds.filter((sid) => sid !== scene!.id);
    else markPlayed(session, scene.id);
  }

  /** The scene's meta line in the run sheet. */
  function sheetMeta(i: number): string {
    const s = session!.scenes[i];
    if (s.id === scene?.id) return t.nowPlaying;
    if (played(s.id)) return t.scenePlayed;
    const where = s.locationId ? locationById.get(s.locationId)?.name : undefined;
    const o = strings.sessions.overview;
    return [where, s.npcIds.length && o.npcCount(s.npcIds.length), s.encounterIds.length && o.encounters(s.encounterIds.length)]
      .filter(Boolean)
      .join(" · ");
  }

  // ---- Playing for: ticks every half minute while it's played ------------------------------------
  let now = $state(Date.now());
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(timer);
  });
  const played_ms = $derived(session ? timePlayed(session, now) : null);
  const playingFor = $derived(played_ms === null ? null : clock(played_ms));
  const paused = $derived(session?.pausedAt != null);
  /** Pauses the time played, or carries it on. */
  function togglePause() {
    if (!session) return;
    now = Date.now();
    if (paused) resume(session, now);
    else pause(session, now);
  }

  // ---- Notes ------------------------------------------------------------------------------------
  let note = $state("");
  function addNote(ev: SubmitEvent) {
    ev.preventDefault();
    const text = note.trim();
    if (!text) return;
    log(text);
    note = "";
  }

  // ---- Saving: every change, at once ------------------------------------------------------------
  let saving = $state(false);
  let saveError = $state<string | null>(null);
  let failed = $state<string | null>(null);
  const changed = $derived(session !== null && baseline !== null && snapshot() !== baseline);
  let inflight: Promise<boolean> | null = null;
  /** Set when leaving couldn't save: the next try leaves anyway. A save that works clears it. */
  let leaveAnyway = false;

  async function save(): Promise<boolean> {
    while (inflight) await inflight;
    if (!session || !changed) return true;
    const sent = snapshot();
    saving = true;
    inflight = saveSession(campaign, session).then(
      (saved) => {
        session!.updated = saved.updated;
        baseline = sent;
        [saveError, failed, leaveAnyway] = [null, null, false];
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

  $effect(() => {
    if (!changed || saving || snapshot() === failed) return;
    const timer = setTimeout(() => void save(), AUTOSAVE_MS);
    return () => clearTimeout(timer);
  });

  // Leaving saves first; when that fails, leaving again goes anyway (the error says what wasn't saved).
  beforeNavigate((navigation) => {
    if (leaveAnyway || !changed || !navigation.to || navigation.type === "leave") return;
    navigation.cancel();
    const to = navigation.to.url.href;
    void save().then((ok) => {
      leaveAnyway = true;
      if (!ok) return;
      if (navigation.type === "popstate") history.go(navigation.delta ?? -1);
      else void gotoReplacing(to);
    });
  });

  // ---- End session ------------------------------------------------------------------------------
  let summaryDialog: SummaryDialog | undefined = $state();
  /** End session asks first, with the summary to write: it starts with the summary so far and the new notes. */
  function askEnd() {
    if (session) summaryDialog?.open(summaryDraft(session));
  }

  async function end(summary: string): Promise<boolean> {
    if (!session) return false;
    const before = $state.snapshot(session);
    endSession(session, Date.now(), summary);
    if (!(await save())) {
      // Not saved: still running, exactly as it was (the dialog stays open with the error).
      Object.assign(session, before);
      return false;
    }
    leaveAnyway = true;
    // Back where Run session was clicked (prep, or the overview), so Back never returns to an ended run.
    if (canGoBack()) history.back();
    else await gotoReplacing(prep);
    return true;
  }

  // ---- The map ----------------------------------------------------------------------------------
  let mapDialog: HTMLDialogElement | undefined = $state();

  const ready = $derived(session !== null && world !== null);
  showHeaderLead(() => (ready ? liveInfo : null));
  showHeaderAction(() => (ready ? actions : null));
</script>

<!-- Next to the shell's Back: live or played, when it started, the time played and the counts, small. -->
{#snippet liveInfo()}
  {#if session}
    <div class="live-info">
      <span class="label kicker" class:live={session.ended === null && !paused}>
        {#if session.ended === null && !paused}<span class="dot" aria-hidden="true"></span>{/if}
        {[session.ended !== null ? t.played : paused ? t.paused : t.live, session.started && dateFormat.format(session.started), session.started && t.startedAt(timeFormat.format(session.started))]
          .filter(Boolean)
          .join(" · ")}
      </span>
      <dl class="counts">
        {#if playingFor}
          <div>
            <dt class="sr-only">{t.playingFor}</dt>
            <dd><i class="ti ti-clock" aria-hidden="true"></i>{playingFor}</dd>
          </div>
        {/if}
        <div>
          <dt class="sr-only">{t.scenes}</dt>
          <dd><i class="ti ti-masks-theater" aria-hidden="true"></i>{fmt.format(session.playedSceneIds.length)}<span class="of">/{fmt.format(session.scenes.length)}</span></dd>
        </div>
        <div>
          <dt class="sr-only">{t.checklist}</dt>
          <dd><i class="ti ti-check" aria-hidden="true"></i>{fmt.format(counts.done)}<span class="of">/{fmt.format(counts.total)}</span></dd>
        </div>
      </dl>
    </div>
  {/if}
{/snippet}

<!-- Right of the shell's search bar: the save status and End session. -->
{#snippet actions()}
  <div class="bar">
    {#if saveError}
      <span class="error small" role="alert">{saveError}</span>
    {:else if saving}
      <span class="muted small" role="status">{strings.sessions.saving}</span>
    {/if}
    {#if session?.ended === null}
      <button class="btn" onclick={togglePause}>
        <i class="ti {paused ? 'ti-player-play' : 'ti-player-pause'}" aria-hidden="true"></i>
        {paused ? t.resume : t.pause}
      </button>
      <button class="btn" onclick={askEnd}>{t.end}</button>
    {/if}
  </div>
{/snippet}

<svelte:head><title>{session ? t.title(session.number, session.title) : strings.sessions.run}</title></svelte:head>

{#if loadError}
  <div class="status"><p class="error" role="alert">{loadError}</p></div>
{:else if !session || !world}
  <p class="status muted" role="status">{strings.sessions.loading}</p>
{:else}
  <div class="run">
    {#each lookupErrors as error (error)}<p class="error small" role="alert">{error}</p>{/each}
    {#if party.length > 0}
      <header class="head">
        <ul class="party" aria-label={t.party}>
          {#each party as c (c.id)}
            <li>
              <strong class="card-name">{c.name}</strong>
              {#if c.picture}
                <img class="hex" src={pictureSrc(c.picture)} alt="" />
              {:else}
                <span class="hex" aria-hidden="true">{c.name.charAt(0)}</span>
              {/if}
              <dl class="passives">
                {#each [["per", c.passivePerception], ["ins", c.passiveInsight], ["inv", c.passiveInvestigation]] as const as [key, value] (key)}
                  <dt><span aria-hidden="true">{t.passives[key]}</span><span class="sr-only">{t.passiveNames[key]}</span></dt>
                  <dd>{value === null ? "–" : fmt.format(value)}</dd>
                {/each}
              </dl>
              <span class="hp">
                <span class="sr-only">{t.hp}</span>
                <strong>{c.currentHp === null ? "–" : fmt.format(c.currentHp)}/{c.maxHp === null ? "–" : fmt.format(c.maxHp)}</strong>
              </span>
            </li>
          {/each}
        </ul>
      </header>
    {/if}

    <div class="columns">
      <div class="col left">
        <nav class="sheet" aria-label={t.runSheet} use:keepScroll={{ id: "run-sheet", url: page.url.href, ready: true }}>
          <span class="label">{t.runSheet}</span>
          {#each session.scenes as s, i (s.id)}
            {@const current = s.id === scene?.id}
            <button class="sheet-row" class:current class:played={played(s.id) && !current} aria-current={current ? "step" : undefined} onclick={() => go(i)}>
              <span class="badge">{played(s.id) && !current ? "✓" : fmt.format(i + 1)}</span>
              <span class="text">
                <span class="title">{untitled(s.title)}</span>
                <span class="meta">{sheetMeta(i)}</span>
              </span>
            </button>
          {/each}
        </nav>

        <!-- The current scene's encounters, under the run sheet. -->
        {#if scene && scene.encounterIds.length > 0}
          <section class="encounters" aria-label={t.encounters}>
            <span class="label"><i class="ti ti-swords" aria-hidden="true"></i> {t.encounters}</span>
            {#if encounterError}
              <p class="error small" role="alert">{encounterError}</p>
            {:else if all === null}
              <p class="muted" role="status">{strings.sessions.loading}</p>
            {:else}
            {#each scene.encounterIds as eid (eid)}
              {@const e = encounterById.get(eid)}
              {#if e}
                {@const band = encounterCard(e, byKey, characters, budgets).band}
                <div class="encounter">
                  <a class="encounter-name" href="{encountersBase}/{e.id}">{e.name}</a>
                  {#if band}<i class="ti {BAND_ICONS[band]} band" role="img" aria-label={strings.encounters.meter.bands[band]}></i>{/if}
                  {#if e.status === "ready"}
                    <a class="icon-btn" href="{encountersBase}/{e.id}/run" aria-label={t.runEncounter(e.name)}><i class="ti ti-player-play" aria-hidden="true"></i></a>
                  {:else}
                    <button class="icon-btn" disabled aria-label={t.runEncounter(e.name)}><i class="ti ti-player-play" aria-hidden="true"></i></button>
                  {/if}
                </div>
              {:else}
                <p class="muted">{strings.sessions.encounters.missing}</p>
              {/if}
            {/each}
            {/if}
          </section>
        {/if}
      </div>

      <main class="col scene" aria-label={t.currentScene} use:keepScroll={{ id: "run-scene", url: page.url.href, ready: true }}>
        {#if !scene}
          <p class="muted">{t.noScenes}</p>
        {:else}
          <div class="scene-head">
            <div class="who">
              <span class="label kicker live">{t.sceneOf(index + 1, session.scenes.length)} · {played(scene.id) ? t.scenePlayed : t.nowPlaying}</span>
              <h2>{untitled(scene.title)}</h2>
            </div>
            <button class="icon-btn icon-btn-lg outline" aria-label={t.previous} disabled={index === 0} onclick={() => go(index - 1)}>
              <i class="ti ti-chevron-left" aria-hidden="true"></i>
            </button>
            <button class="icon-btn icon-btn-lg outline" aria-pressed={played(scene.id)} aria-label={t.played} disabled={session.ended !== null} onclick={togglePlayed}>
              {#if played(scene.id)}<i class="ti ti-check" aria-hidden="true"></i>{/if}
            </button>
            <button class="icon-btn icon-btn-lg outline" aria-label={t.next} disabled={last} onclick={next}>
              <i class="ti ti-chevron-right" aria-hidden="true"></i>
            </button>
          </div>

          <SideTabs tabs={TABS} bind:current={tab} id="run-scene" label={t.parts}>
            {#if tab === "info"}
              <div class="tab-body">
                <div class="tab-main">
                  {#if isEmpty(scene.summary)}
                    <p class="muted">{strings.sessions.world.nothingYet}</p>
                  {:else}
                    <div class="prose notes what">{@html cleanHtml(scene.summary, { links: true, base: libraryHome() })}</div>
                  {/if}
                </div>
                <!-- The scene's checklist, in a narrow column on the right. -->
                {#if sceneChecks && sceneChecks.total > 0}
                  <section class="tab-side" aria-label={t.sceneChecklist}>
                    <div class="box-head">
                      <span class="label">{t.sceneChecklist}</span>
                      <span class="mono muted small">{fmt.format(sceneChecks.done)}/{fmt.format(sceneChecks.total)}</span>
                    </div>
                    <Checklist bind:items={scene.checklist} placeholder="" compact locked />
                  </section>
                {/if}
              </div>
            {:else if tab === "location"}
              {#if !location}
                <p class="muted">{strings.sessions.scenes.noLocation}</p>
              {:else}
                <section class="location">
                  {#if banner}<img class="banner" src={pictureSrc(banner)} alt="" />{/if}
                  <div class="location-body">
                    <div class="location-text">
                      <strong class="location-name">{location.name || strings.sessions.manage.unnamedLocation}</strong>
                      {#each [["atmosphere", location.atmosphere], ["purpose", location.purpose], ["secrets", location.secrets]] as const as [key, html] (key)}
                        {#if !isEmpty(html)}
                          <div>
                            <span class="label">{strings.sessions.locationFields[key]}</span>
                            <div class="prose">{@html cleanHtml(html)}</div>
                          </div>
                        {/if}
                      {/each}
                    </div>
                    <!-- The map and who's found here, in a narrow column on the right. -->
                    {#if location.map || locationNpcs.length > 0}
                      <div class="location-side">
                        {#if location.map}
                          <button class="chip-map" aria-label={t.openMap} onclick={() => mapDialog?.showModal()}><i class="ti ti-map" aria-hidden="true"></i> {t.map}</button>
                        {/if}
                        {#if locationNpcs.length > 0}
                          <span class="label">{t.foundHere}</span>
                          {#each locationNpcs as n (n.id)}
                            <button class="person" onclick={() => showNpc(n)}>
                              {#if n.portrait}<img class="face lg" src={pictureSrc(n.portrait)} alt="" />{:else}<span class="face lg" aria-hidden="true">{n.name.charAt(0)}</span>{/if}
                              <span class="name"><strong>{n.name || strings.sessions.manage.unnamedNpc}</strong>{#if n.job}<span class="muted small">{n.job}</span>{/if}</span>
                            </button>
                          {/each}
                        {/if}
                      </div>
                    {/if}
                  </div>
                </section>
              {/if}
            {:else}
              {#if sceneNpcs.length === 0}
                <p class="muted">{t.noNpcs}</p>
              {/if}
              <div class="npc-list">
                {#each sceneNpcs as n (n.id)}
                  <button class="npc npc-head" onclick={() => showNpc(n)}>
                    {#if n.portrait}<img class="face xl" src={pictureSrc(n.portrait)} alt="" />{:else}<span class="face xl" aria-hidden="true">{n.name.charAt(0)}</span>{/if}
                    <span class="name"><strong>{n.name || strings.sessions.manage.unnamedNpc}</strong>{#if n.job}<span class="muted small">{n.job}</span>{/if}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </SideTabs>
        {/if}
      </main>

      <aside class="col side" aria-label={t.across} use:keepScroll={{ id: "run-side", url: page.url.href, ready: true }}>
        <section class="group" aria-label={t.across}>
          <span class="label">{t.across}</span>
          {#each [["previously", strings.sessions.sections.previously, session.previously], ["premise", strings.sessions.sections.premise, session.premise], ["partyNote", strings.sessions.sections.partyNote, session.partyNote]] as const as [key, label, text] (key)}
            <!-- Only the notes that were written. -->
            {#if text.trim()}{@render fold(key, label, null, text)}{/if}
          {/each}
          {@render fold("checklist", strings.sessions.sections.checklist, `${fmt.format(session.checklist.filter((c) => c.done).length)}/${fmt.format(session.checklist.length)}`, null)}
          {@render fold("notes", t.notes, fmt.format(session.log.length), null)}
        </section>
      </aside>
    </div>
  </div>

  {#snippet fold(key: string, label: string, meta: string | null, text: string | null)}
    {@const open = section === key}
    <div class="fold">
      <button class="fold-head" aria-expanded={open} onclick={() => (section = open ? null : key)}>
        <span class="fold-title"><strong>{label}</strong>{#if meta}<span class="mono muted small">{meta}</span>{/if}</span>
        <i class="ti ti-chevron-down" class:flipped={open} aria-hidden="true"></i>
      </button>
      {#if open}
        <div class="fold-body">
          {#if key === "checklist"}
            <Checklist bind:items={session!.checklist} placeholder="" compact locked />
          {:else if key === "notes"}
            <form class="note-form" onsubmit={addNote}>
              <input class="text-field" bind:value={note} maxlength="2000" autocomplete="off" placeholder={t.notePlaceholder} aria-label={t.newNote} />
              <button class="btn" type="submit" disabled={!note.trim()}>{t.addNote}</button>
            </form>
            {#if session!.log.length > 0}
              <ol class="log">
                {#each session!.log as entry, i (i)}
                  <li>
                    <span class="note-text">{entry.text}</span>
                    <button class="icon-btn" aria-label={t.removeNote(entry.text)} onclick={() => session!.log.splice(i, 1)}>
                      <i class="ti ti-x" aria-hidden="true"></i>
                    </button>
                  </li>
                {/each}
              </ol>
            {/if}
          {:else if text}
            <p class="plain">{text}</p>
          {/if}
        </div>
      {/if}
    </div>
  {/snippet}

  <SummaryDialog bind:this={summaryDialog} notes={session.log} message={t.confirmEnd} saveLabel={t.end} error={saveError} onsave={end} />

  <dialog bind:this={npcDialog} class="dialog npc-dialog" aria-labelledby="npc-name" onclose={() => (shownNpc = null)}>
    {#if shownNpc}
      {@const n = shownNpc}
      {@const e = strings.entry}
      {@const f = strings.sessions.npcFields}
      {@const basics = [[e.ac, n.stats.ac], [e.hp, n.stats.hp], [e.speed, n.stats.speed], [e.cr, n.stats.cr]].filter(([, v]) => v.trim())}
      {@const scores = (["str", "dex", "con", "int", "wis", "cha"] as const).filter((k) => n.stats[k].trim())}
      <header class="npc-top">
        {#if n.portrait}<img class="portrait" src={pictureSrc(n.portrait)} alt="" />{/if}
        <div>
          <h2 id="npc-name">{n.name || strings.sessions.manage.unnamedNpc}</h2>
          {#if npcSpecs(n)}<p class="muted">{npcSpecs(n)}</p>{/if}
        </div>
      </header>
      {#if basics.length || scores.length}
        <section class="npc-stats" aria-label={f.stats}>
          {#if basics.length}
            <p>{#each basics as [label, value], i (label)}{#if i}<span class="muted" aria-hidden="true"> · </span>{/if}<strong>{label}</strong> {value}{/each}</p>
          {/if}
          {#if scores.length}
            <dl class="scores">
              {#each scores as k (k)}<div><dt class="label">{k}</dt><dd>{n.stats[k]}</dd></div>{/each}
            </dl>
          {/if}
        </section>
      {/if}
      {#each [[f.summary, n.summary], [f.motivation, n.motivation], [f.traits, n.traits], [f.backstory, n.backstory]] as [label, html] (label)}
        {#if !isEmpty(html)}
          <section>
            <span class="label">{label}</span>
            <div class="prose">{@html cleanHtml(html)}</div>
          </section>
        {/if}
      {/each}
      <div class="dialog-actions">
        <button class="btn" onclick={() => npcDialog?.close()}>{strings.sessions.locationFields.close}</button>
      </div>
    {/if}
  </dialog>

  {#if location?.map}
    <dialog bind:this={mapDialog} class="dialog map-dialog" aria-label={location.name}>
      <div class="map-frame">
        <NotesPanel topic="map" notes={{ description: "", pins: location.pins }} map={location.map} locked setMap={null} />
      </div>
      <div class="dialog-actions">
        <button class="btn" onclick={() => mapDialog?.close()}>{strings.sessions.locationFields.close}</button>
      </div>
    </dialog>
  {/if}
{/if}

<style>
  .status {
    padding: var(--space-5);
  }

  .error {
    color: var(--color-danger);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  /* Nothing goes below the fold: the header stays, each column is the rest of the window's height and scrolls inside. */
  .run {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    height: 100%;
    padding: var(--space-4) var(--space-4) var(--space-5);
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .who {
    flex: 1 1 var(--search-width);
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  /* In the campaign header: the kicker over the counts, both small. */
  .live-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    white-space: nowrap;
  }

  .kicker {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .kicker.live {
    color: var(--color-accent);
  }

  .dot {
    width: var(--space-2);
    height: var(--space-2);
    border-radius: 50%;
    background: currentColor;
  }

  h2 {
    margin: 0;
    font: var(--text-h2);
    letter-spacing: var(--tracking-tight);
  }

  .counts {
    display: flex;
    gap: var(--space-4);
    margin: 0;
  }

  .counts div {
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
  }

  .counts dd {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }

  .counts dd {
    gap: var(--space-1);
  }

  .counts dd .ti {
    align-self: center;
    color: var(--color-text-muted);
    font-size: var(--icon-size-sm);
  }

  .counts dt {
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .counts dd,
  .hp strong {
    margin: 0;
    font-family: var(--font-mono);
    font-weight: var(--weight-bold);
  }

  .counts dd {
    font-size: var(--text-sm);
  }

  .hp strong {
    font-size: var(--text-md);
  }

  .of {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  /* Every card the same width. */
  /* Six across, each the same width (fewer leave room on the right); three across in a narrow window. */
  .party {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* The name on top; under it the picture, as tall as the passives, then the passives and HP. */
  /* The name on top; under it the picture (as tall as the passives) on the left, the passives and HP on the right,
     HP in the bottom corner. */
  .party li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto auto;
    align-items: center;
    column-gap: var(--space-2);
    row-gap: var(--space-1);
    min-width: 0;
    padding: var(--space-2);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  .hex {
    display: grid;
    place-items: center;
    flex: none;
    width: calc(1.6 * var(--roster-badge-width));
    height: calc(1.6 * var(--roster-badge-height));
    object-fit: cover;
    background: var(--color-border-strong);
    color: var(--color-text);
    font-weight: var(--weight-bold);
    clip-path: polygon(25% 0, 75% 0, 100% 50%, 75% 100%, 25% 100%, 0 50%);
  }

  .name {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .card-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* As tall as the three lines of passives (never the picture's own size), in the badge's proportions. */
  .party {
    --passive-line: calc(var(--text-xs) * 1.4);
  }

  .party .hex {
    height: calc(3 * var(--passive-line));
    width: calc(3 * var(--passive-line) * 36 / 31); /* --roster-badge-width / --roster-badge-height */
  }

  .party .passives {
    justify-self: start;
    line-height: var(--passive-line);
  }

  .passives {
    display: grid;
    grid-template-columns: auto auto;
    column-gap: var(--space-2);
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }

  .passives dt {
    color: var(--color-text-muted);
  }

  .passives dd {
    margin: 0;
    font-weight: var(--weight-bold);
    text-align: right;
  }

  .party li > .card-name {
    grid-column: 1 / -1;
  }

  .party li > .passives {
    grid-column: 3;
  }

  .hp {
    grid-column: 4;
    align-self: end;
    margin-left: var(--space-2); /* a little more room after the passives */
    white-space: nowrap;
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

  /* The run sheet, and the current scene's encounters at the bottom of the column. */
  .left {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .sheet {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .sheet > .label,
  .group > .label {
    padding: 0 var(--space-1);
  }

  .sheet-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    min-height: var(--control-height-xl);
    padding: var(--space-1) var(--space-3);
    border: var(--border-width) solid transparent;
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text);
    text-align: left;
  }

  .sheet-row:hover {
    background: var(--color-surface);
  }

  .sheet-row.current {
    border-color: var(--color-accent);
    background: var(--color-accent);
    color: var(--color-accent-text);
  }

  .sheet-row.played {
    opacity: 0.6;
  }

  .badge {
    display: grid;
    place-items: center;
    flex: none;
    width: var(--roster-badge-width);
    height: var(--roster-badge-height);
    background: var(--color-border-strong);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    clip-path: polygon(25% 0, 75% 0, 100% 50%, 75% 100%, 25% 100%, 0 50%);
  }

  .current .badge {
    background: var(--charcoal);
    color: var(--potion-glow);
  }

  .played .badge {
    background: var(--color-ready-fill);
    color: var(--color-ready-text);
  }

  .sheet-row .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .sheet-row .title {
    font-weight: var(--weight-bold);
  }

  .sheet-row .meta {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .sheet-row.current .meta {
    color: inherit;
    font-weight: var(--weight-bold);
  }

  .scene {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  /* Its parts keep their height; the column scrolls instead (the location and the text clip their overflow, which
     would otherwise let them shrink). */
  .scene > * {
    flex: none;
  }

  .scene-head {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: var(--space-2);
  }

  .scene-head .who {
    flex-basis: calc(var(--search-width) / 1.3);
  }

  .outline {
    border: var(--border-width) solid var(--color-border-strong);
  }

  .location {
    overflow: hidden;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  .banner {
    display: block;
    width: 100%;
    max-height: calc(var(--control-height-xl) * 3);
    object-fit: cover;
  }

  /* The text, and a narrow column on the right (the scene's checklist; the location's map and the NPCs found there);
     it goes under the text when narrow. */
  .tab-body,
  .location-body {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-5);
  }

  .location-body {
    padding: var(--space-4) var(--space-5) var(--space-5);
  }

  .tab-main,
  .location-text {
    flex: 999 1 var(--search-width);
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .tab-side {
    flex: 1 1 calc(var(--search-width) / 2);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-left: var(--space-4);
    border-left: var(--border-width) solid var(--color-border);
  }

  .location-side {
    flex: 1 1 calc(var(--search-width) / 2);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
    padding-left: var(--space-4);
    border-left: var(--border-width) solid var(--color-border);
  }

  .chip-map {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 0 var(--space-1);
    border: var(--border-width) solid var(--color-accent);
    border-radius: var(--radius-sm);
    color: var(--color-accent);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    background: none;
    cursor: pointer;
  }

  .chip-map:hover {
    background: var(--color-accent-soft);
  }

  .location-name {
    font: var(--text-h2);
  }

  .flipped {
    transform: rotate(180deg);
  }

  .face {
    display: grid;
    place-items: center;
    flex: none;
    border-radius: var(--radius-md);
    object-fit: cover;
    background: var(--color-border-strong);
    font-weight: var(--weight-bold);
  }

  .face.lg {
    width: var(--control-height);
    height: var(--control-height);
  }

  .face.xl {
    width: var(--control-height-lg);
    height: var(--control-height-lg);
    font-size: var(--text-lg);
  }

  .npc-list {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }

  .npc-list {
    align-items: stretch;
  }

  .person {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  /* Rich text fills its column here (prose.css caps it at a reading width for the Library). */
  .scene .prose {
    max-width: none;
    margin: 0;
  }

  /* A table wider than the column scrolls sideways here, not the whole column (as on Session prep). */
  .what {
    font-size: var(--text-lg);
    overflow-wrap: anywhere;
    overflow-x: auto;
  }

  /* One line each: the name (opens it in the builder), its difficulty and Run (Ready ones only). */
  .encounter {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-1) var(--space-1) var(--space-3);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .encounter-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--color-text);
    font-weight: var(--weight-bold);
    text-decoration: none;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .encounter-name:hover {
    text-decoration: underline;
  }

  .band {
    color: var(--color-text-muted);
    font-size: var(--icon-size);
  }

  .encounters {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: auto;
    padding-top: var(--space-4);
    border-top: var(--border-width) solid var(--color-border);
  }

  .side {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .fold {
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .npc {
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  .npc-head,
  .fold-head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    min-height: var(--control-height-lg);
    padding: var(--space-2) var(--space-3);
    color: var(--color-text);
    text-align: left;
  }

  .fold-head {
    border: 0;
    background: none;
  }

  .npc-head:hover,
  .person:hover {
    border-color: var(--color-accent);
  }

  .person {
    padding: var(--space-1);
    border: var(--border-width) solid transparent;
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text);
    text-align: left;
  }

  .npc-dialog {
    max-height: 85vh;
    overflow-y: auto;
  }

  .npc-dialog .prose {
    max-width: none;
    margin: 0;
  }

  .npc-top {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .npc-top h2 {
    font: var(--text-h2);
  }

  .npc-top p {
    margin: 0;
  }

  .portrait {
    flex: none;
    width: calc(2 * var(--control-height-xl));
    height: calc(2 * var(--control-height-xl));
    border-radius: var(--radius-md);
    object-fit: cover;
  }

  .npc-stats p {
    margin: 0 0 var(--space-2);
  }

  .scores {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: var(--space-2);
    margin: 0;
    text-align: center;
  }

  .scores dd {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--text-lg);
    font-weight: var(--weight-bold);
  }

  .box-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .fold-title {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .fold-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: 0 var(--space-3) var(--space-3);
  }

  .plain {
    margin: 0;
    white-space: pre-wrap;
  }

  .note-form {
    display: flex;
    gap: var(--space-2);
  }

  .note-form input {
    flex: 1;
    min-width: 0;
  }

  .log {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .log li {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
  }

  .note-text {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .map-dialog {
    width: min(90vw, calc(var(--search-width) * 3));
  }

  .map-frame {
    display: flex;
    height: 70vh;
    margin-bottom: var(--space-3);
  }

  /* A narrow window: everything stacks in one scrolling column. */
  @media (max-width: 900px) {
    .run {
      height: auto;
    }

    .columns {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: none;
    }

    .col {
      overflow: visible;
    }

    .party {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
</style>
