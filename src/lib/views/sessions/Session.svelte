<script lang="ts">
  // Sessions → one session: the header (number, title, date, time, players, prep progress) stays put; the steps, the
  // open step and the right column each scroll inside their own column. Leaving with unsaved changes asks Save /
  // Don't save / Cancel. A new session (id null) gets its address once it's saved.
  import { beforeNavigate, goto } from "$app/navigation";
  import { page } from "$app/state";
  import { tick } from "svelte";
  import BackButton from "$lib/components/BackButton.svelte";
  import { catalogRows, type CatalogRow } from "$lib/api/catalog";
  import { listCharacters, type Character } from "$lib/api/characters";
  import { encounterBudgets, listEncounters, saveEncounter, type Budget, type Encounter } from "$lib/api/encounters";
  import { deleteSession, getSession, listSessions, saveSession, setSessionMap, type Session, type Step, type StepKind } from "$lib/api/sessions";
  import { errorMessage } from "$lib/api/storage";
  import { closeWindow, onCloseRequested } from "$lib/api/window";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { gotoReplacing, isReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import { strings } from "$lib/strings";
  import { difficulty, monsterXp } from "$lib/views/encounters/difficulty";
  import { characterLevel, encounterParty, entryKey, rowsByKey } from "$lib/views/encounters/roster";
  import SessionAside from "./SessionAside.svelte";
  import StepList from "./StepList.svelte";
  import StepPanel from "./StepPanel.svelte";
  import { newStep, nextNumber, readyCount } from "./sessions";

  /** The saved session to open, or null for a new one. */
  let { id }: { id: string | null } = $props();

  const t = strings.sessions;
  const e = strings.encounters;
  const campaign = campaigns.current!.id;
  const base = `/campaign/${campaign}/sessions`;
  const encountersBase = `/campaign/${campaign}/encounters`;

  // ---- Loading -----------------------------------------------------------------------------------
  let loadError = $state<string | null>(null);
  let draft = $state<Session>({ id: "", number: 1, title: "", date: null, time: null, steps: [], partyNote: "", toBring: [], updated: 0 });
  let selected = $state<string | null>(null);
  /** What was last saved (or loaded), to tell whether there are changes. Null until it has loaded. */
  let baseline = $state<string | null>(null);

  // A Map step's picture is stored at once, not with Save, so it isn't a change.
  const snapshot = () => JSON.stringify({ ...draft, title: draft.title.trim(), updated: 0, steps: draft.steps.map((s) => ({ ...s, map: null })) });
  /** The Map steps as last saved (or loaded): only those can be given a picture. */
  let savedMaps = $state(new Set<string>());
  const mapsOf = (s: Session) => new Set(s.steps.filter((st) => st.kind === "map").map((st) => st.id));

  /** Sets a Map step's picture. A step not saved yet (or a new session) is saved first, as New encounter does. */
  function mapSetter(step: Step) {
    return async (path: string | null) => {
      if (!draft.id || !savedMaps.has(step.id)) {
        if (!(await save(false))) throw new Error(saveError ?? t.saveError(""));
      }
      step.map = await setSessionMap(campaign, draft.id, step.id, path);
      // A new session gets its own address once it has its map.
      if (!id) {
        leaving = true;
        await gotoReplacing(`${base}/${draft.id}`);
      }
    };
  }

  // svelte-ignore state_referenced_locally
  const opening = id ? getSession(campaign, id) : listSessions(campaign).then((l) => nextNumber(l.sessions));
  opening.then(
    (opened) => {
      if (typeof opened === "number") draft.number = opened;
      else {
        draft = opened;
        savedMaps = mapsOf(opened);
      }
      selected = draft.steps[0]?.id ?? null;
      baseline = snapshot();
    },
    (err) => (loadError = t.openError(errorMessage(err))),
  );

  // What the encounter step and the party need; without it they show no rating or no one.
  let all = $state<Encounter[] | null>(null);
  let encounterError = $state<string | null>(null);
  let characters = $state<Character[] | null>(null);
  let monsters = $state<CatalogRow[]>([]);
  let budgets = $state<Budget[]>([]);
  listEncounters(campaign).then(
    (l) => (all = l.encounters),
    (err) => (encounterError = t.items.loadError(errorMessage(err))),
  );
  listCharacters(campaign).then((l) => (characters = l.characters), () => {});
  catalogRows("monsters", ["xp"]).then((rows) => (monsters = rows), () => {});
  encounterBudgets().then((b) => (budgets = b), () => {});

  const byKey = $derived(rowsByKey(monsters));
  const characterById = $derived(new Map((characters ?? []).map((c) => [c.id, c])));
  const party = $derived(encounterParty("party", [], characters).flatMap((cid) => characterById.get(cid) ?? [])); // as in Prepare

  /** Difficulty and XP as the Encounters cards compute them. */
  function rate(encounter: Encounter) {
    const levels = encounterParty(encounter.partyMode, encounter.party, characters).map((cid) => characterLevel(characterById.get(cid)?.classes));
    const xps = encounter.monsters.map((m) => monsterXp(byKey.get(entryKey(m))?.fields.xp));
    const result = difficulty(levels, encounter.monsters.map((m, i) => ({ quantity: m.quantity, xp: xps[i] })), budgets);
    return { band: result.band, xp: result.total };
  }

  // ---- Steps -------------------------------------------------------------------------------------
  const index = $derived(draft.steps.findIndex((s) => s.id === selected));
  const ready = $derived(readyCount(draft.steps));

  function addStep(kind: StepKind) {
    const step = newStep(kind);
    draft.steps.push(step);
    selected = step.id;
  }

  function removeStep(id: string) {
    const at = draft.steps.findIndex((s) => s.id === id);
    if (at < 0) return;
    draft.steps.splice(at, 1);
    if (selected === id) selected = (draft.steps[Math.min(at, draft.steps.length - 1)] ?? null)?.id ?? null;
  }

  // ---- Saving and leaving ------------------------------------------------------------------------
  let saving = $state(false);
  let saveError = $state<string | null>(null);
  const valid = $derived(Number.isInteger(draft.number) && draft.number >= 1);

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
  const changed = $derived(baseline !== null && snapshot() !== baseline);
  let leaving = false;

  /** Saves the changes; a new session then gets its own address (unless `go` is false). */
  async function save(go = true): Promise<boolean> {
    if (saving) return false;
    if (!valid) {
      saveError = t.saveError(t.number);
      return false;
    }
    saving = true;
    saveError = null;
    const sent = snapshot();
    try {
      const saved = await saveSession(campaign, { ...draft, title: draft.title.trim() || t.newTitle });
      draft.id = saved.id;
      draft.updated = saved.updated;
      savedMaps = mapsOf(saved);
      if (sent === snapshot()) {
        draft.title = saved.title; // a blank title was saved as "New session"
        baseline = snapshot();
      } else baseline = sent; // typed on while it saved: those changes are still unsaved
      notify(t.saved(saved.title));
      if (!id && go) {
        leaving = true;
        await gotoReplacing(`${base}/${saved.id}`);
      }
      return true;
    } catch (err) {
      saveError = t.saveError(errorMessage(err));
      return false;
    } finally {
      saving = false;
    }
  }

  type LeaveAnswer = "save" | "discard" | "cancel";
  let leaveDialog: HTMLDialogElement | undefined = $state();
  let resolveLeave: ((answer: LeaveAnswer) => void) | null = null;

  function askLeave(): Promise<LeaveAnswer> {
    leaveDialog?.showModal();
    return new Promise((resolve) => (resolveLeave = resolve));
  }

  // Escape closes the dialog too, which counts as Cancel.
  function answerLeave(answer: LeaveAnswer) {
    const resolve = resolveLeave;
    resolveLeave = null;
    if (leaveDialog?.open) leaveDialog.close();
    resolve?.(answer);
  }

  // Hold the navigation (Back included), ask, then go on.
  beforeNavigate((navigation) => {
    if (leaving || !changed || !navigation.to || navigation.type === "leave") return;
    const replace = isReplacing();
    navigation.cancel();
    void askLeave().then(async (answer) => {
      if (answer === "cancel" || (answer === "save" && !(await save(false)))) return;
      leaving = true;
      if (navigation.type === "popstate") history.go(navigation.delta ?? -1);
      else void (replace ? gotoReplacing : goto)(navigation.to!.url.href);
    });
  });

  // Closing the window is leaving too (SvelteKit's "leave" can't be held), so ask the same question first.
  $effect(() => {
    const unlisten = onCloseRequested(async (event) => {
      if (leaving || !changed) return;
      event.preventDefault();
      const answer = await askLeave();
      if (answer === "cancel" || (answer === "save" && !(await save(false)))) return;
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
    if (!draft.id) return;
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

  // ---- A new encounter for the open step -----------------------------------------------------------
  let creating = $state(false);
  /** An empty Draft encounter linked to the step, saved with the session (a new one gets its address first), then opened in Prepare. */
  async function createEncounter() {
    const step = draft.steps[index];
    if (creating || saving || !step) return;
    if (!valid) {
      saveError = t.saveError(t.number);
      return;
    }
    creating = true;
    saveError = null;
    try {
      const encounter = await saveEncounter(campaign, {
        id: "",
        name: e.newTitle,
        monsters: [],
        party: [],
        status: "draft",
        partyMode: "party",
        notes: { description: "", pins: [] },
        savedAt: 0,
      });
      step.encounter = encounter.id;
      all = [...(all ?? []), encounter];
      if (!(await save())) return;
      leaving = true;
      await goto(`${encountersBase}/${encounter.id}`);
    } catch (err) {
      saveError = t.items.createError(errorMessage(err));
    } finally {
      creating = false;
    }
  }

  const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "full" });
</script>

<svelte:head><title>{draft.title.trim() || t.newTitle}</title></svelte:head>

{#if loadError}
  <div class="status">
    <p class="error" role="alert">{loadError}</p>
    <BackButton fallback={base} />
  </div>
{:else if baseline === null}
  <p class="status muted" role="status">{t.loading}</p>
{:else}
  <div class="session">
    <header class="top">
      <BackButton fallback={base} />
      {#if saveError}<span class="error small" role="alert">{saveError}</span>{/if}
      <span class="spacer"></span>
      {#if draft.id}
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
      {/if}
      <button class="btn save" disabled={saving || !changed || !valid} onclick={() => save()}>
        <i class="ti ti-device-floppy" aria-hidden="true"></i>
        {saving ? t.saving : t.save}
      </button>
    </header>

    <section class="head">
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
      <div class="prep">
        <span class="count">{t.readyOf(ready, draft.steps.length)}</span>
        <progress max={Math.max(draft.steps.length, 1)} value={ready} aria-label={t.prep}></progress>
      </div>
    </section>

    <div class="columns">
      <div class="col" use:keepScroll={{ id: "session-steps", url: page.url.href, ready: true }}>
        <StepList steps={draft.steps} {selected} onselect={(sid) => (selected = sid)} onadd={addStep} onremove={removeStep} />
      </div>
      <div class="col" use:keepScroll={{ id: "session-step", url: page.url.href, ready: true }}>
        {#if index >= 0}
          <StepPanel
            bind:step={draft.steps[index]}
            {encountersBase}
            encounters={all}
            {encounterError}
            {rate}
            busy={creating || saving}
            oncreate={createEncounter}
            setMap={mapSetter(draft.steps[index])}
          />
        {/if}
      </div>
      <div class="col" use:keepScroll={{ id: "session-aside", url: page.url.href, ready: true }}>
        <SessionAside {campaign} {party} bind:partyNote={draft.partyNote} bind:toBring={draft.toBring} />
      </div>
    </div>
  </div>
{/if}

<dialog bind:this={leaveDialog} class="dialog leave" aria-labelledby="leave-title" aria-describedby="leave-question" onclose={() => answerLeave("cancel")}>
  <h2 id="leave-title">{t.leave.title}</h2>
  <p id="leave-question">{t.leave.question(draft.title.trim() || t.newTitle)}</p>
  <div class="actions">
    <button class="btn" onclick={() => answerLeave("cancel")}>{t.leave.cancel}</button>
    <button class="btn" onclick={() => answerLeave("discard")}>{t.leave.discard}</button>
    <!-- svelte-ignore a11y_autofocus: the safe answer is the default -->
    <button class="btn btn-primary" autofocus onclick={() => answerLeave("save")}>{t.leave.save}</button>
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

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .spacer {
    flex: 1;
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
    align-items: flex-end;
    gap: var(--space-5);
    padding: var(--space-3) var(--space-5);
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

  h1 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
  }

  .rename {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
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

  .rename .inline {
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
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

  .prep {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: var(--select-width);
  }

  .count {
    font: var(--text-dice);
  }

  progress {
    width: 100%;
    accent-color: var(--color-accent);
  }

  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: var(--session-steps-width) minmax(0, 1fr) var(--session-aside-width);
    grid-template-rows: minmax(0, 1fr);
    gap: var(--space-3);
  }

  .col {
    min-width: 0;
    overflow-y: auto;
  }

  .leave .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .muted {
    color: var(--color-text-muted);
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

    .col {
      overflow: visible;
    }
  }
</style>
