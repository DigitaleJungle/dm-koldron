<script lang="ts">
  // The open scene: its title, Ready, move up/down and Remove scene; what happens; its location (picked, or a new one,
  // its name and description edited in place, shared with every scene at it); linked NPCs and encounters (chips and a
  // dropdown each, so dragging is never the only way); and its checklist.
  import { tick, untrack } from "svelte";
  import type { Session } from "$lib/api/sessions";
  import { strings } from "$lib/strings";
  import Checklist from "./Checklist.svelte";
  import { link, moveScene, newLocation, scenesWith, unlink } from "./sessions";

  let {
    session = $bindable(),
    index,
    encounterName,
    encounterMeta,
    encounterChoices,
    onremove,
  }: {
    session: Session;
    /** The open scene's place in the session. */
    index: number;
    /** An encounter's name, and its difficulty and XP, by id (null while unknown). */
    encounterName: (id: string) => string;
    encounterMeta: (id: string) => string | null;
    /** The campaign's encounters that are ready to run. */
    encounterChoices: { id: string; name: string }[];
    onremove: () => void;
  } = $props();

  const t = strings.sessions.scenes;
  const m = strings.sessions.manage;
  const scene = $derived(session.scenes[index]);
  const location = $derived(session.locations.find((l) => l.id === scene.locationId) ?? null);
  const sharedWith = $derived(location ? scenesWith(session, { kind: "location", id: location.id }).length - 1 : 0);
  const npcName = (id: string) => session.npcs.find((n) => n.id === id)?.name || m.unnamedNpc;

  let titleInput: HTMLInputElement | undefined = $state();
  // A new scene has no title yet: start typing it.
  $effect(() => {
    void scene.id;
    if (!untrack(() => scene.title)) titleInput?.focus();
  });

  let confirming = $state(false);
  $effect(() => {
    void scene.id;
    confirming = false;
  });

  async function move(by: -1 | 1, ev: MouseEvent) {
    const button = ev.currentTarget as HTMLButtonElement;
    moveScene(session, index, by);
    await tick();
    if (button.disabled) document.getElementById(by < 0 ? "scene-down" : "scene-up")?.focus(); // keep focus on a working button
  }

  function pickLocation(ev: Event & { currentTarget: HTMLSelectElement }) {
    const value = ev.currentTarget.value;
    if (value === "__new") {
      const created = newLocation(t.newLocationName);
      session.locations.push(created);
      scene.locationId = created.id;
      void tick().then(() => document.getElementById("loc-name")?.focus());
    } else scene.locationId = value || null;
  }

  function pickLink(ev: Event & { currentTarget: HTMLSelectElement }, ids: string[]) {
    const value = ev.currentTarget.value;
    if (value) link(ids, value);
    ev.currentTarget.value = "";
  }
</script>

<div class="head">
  <div class="title">
    <label for="scene-title" class="label">{t.position(index + 1, session.scenes.length)}</label>
    <input id="scene-title" bind:this={titleInput} bind:value={scene.title} maxlength="100" autocomplete="off" placeholder={t.untitled} />
  </div>
  <div class="actions">
    <button class="btn" aria-pressed={scene.ready} onclick={() => (scene.ready = !scene.ready)}>
      <i class="ti ti-check" aria-hidden="true"></i>
      {scene.ready ? t.ready : t.markReady}
    </button>
    <button id="scene-up" class="btn icon" disabled={index === 0} aria-label={t.moveUp} onclick={(ev) => move(-1, ev)}>
      <i class="ti ti-chevron-up" aria-hidden="true"></i>
    </button>
    <button id="scene-down" class="btn icon" disabled={index === session.scenes.length - 1} aria-label={t.moveDown} onclick={(ev) => move(1, ev)}>
      <i class="ti ti-chevron-down" aria-hidden="true"></i>
    </button>
    {#if confirming}
      <button class="btn" onclick={() => (confirming = false)}>{strings.sessions.cancel}</button>
      <button class="btn btn-danger" onclick={onremove}>{t.remove}</button>
    {:else}
      <button class="btn danger" onclick={() => (confirming = true)}>{t.remove}</button>
    {/if}
  </div>
</div>

<div class="field">
  <label for="scene-summary" class="label">{t.whatHappens}</label>
  <textarea id="scene-summary" class="text-field" rows="2" maxlength="20000" bind:value={scene.summary}></textarea>
</div>

<section class="block" aria-label={t.location}>
  <h3 class="label"><i class="ti ti-map-pin" aria-hidden="true"></i>{t.location}</h3>
  <div class="field">
    <label for="scene-loc" class="small-label">{t.whereItHappens}</label>
    <select id="scene-loc" class="text-field" value={scene.locationId ?? ""} onchange={pickLocation}>
      <option value="">{t.noLocation}</option>
      {#each session.locations as l (l.id)}
        <option value={l.id}>{l.name || m.unnamedLocation}</option>
      {/each}
      <option value="__new">{t.newLocation}</option>
    </select>
  </div>
  {#if location}
    <div class="field">
      <label for="loc-name" class="small-label">{t.name}</label>
      <input id="loc-name" class="text-field" bind:value={location.name} maxlength="100" autocomplete="off" />
    </div>
    <div class="field">
      <label for="loc-desc" class="small-label">{t.description}</label>
      <textarea id="loc-desc" class="text-field" rows="2" maxlength="20000" bind:value={location.description}></textarea>
    </div>
    {#if sharedWith > 0}<p class="note">{t.shared(sharedWith)}</p>{/if}
  {/if}
</section>

<section class="block" aria-label={t.npcs}>
  <label for="link-npc" class="label"><i class="ti ti-user" aria-hidden="true"></i>{t.npcs}</label>
  <div class="chips">
    {#each scene.npcIds as id (id)}
      <span class="link">
        <strong>{npcName(id)}</strong>
        <button class="icon-btn" aria-label={t.unlink(npcName(id))} onclick={() => unlink(scene.npcIds, id)}>
          <i class="ti ti-x" aria-hidden="true"></i>
        </button>
      </span>
    {/each}
    <select id="link-npc" class="pick" value="" onchange={(ev) => pickLink(ev, scene.npcIds)}>
      <option value="">{t.linkNpc}</option>
      {#each session.npcs.filter((n) => !scene.npcIds.includes(n.id)) as n (n.id)}
        <option value={n.id}>{n.name || m.unnamedNpc}</option>
      {/each}
    </select>
  </div>
</section>

<section class="block" aria-label={t.encounters}>
  <label for="link-enc" class="label"><i class="ti ti-swords" aria-hidden="true"></i>{t.encounters}</label>
  <div class="chips">
    {#each scene.encounterIds as id (id)}
      {@const meta = encounterMeta(id)}
      <span class="link">
        <strong>{encounterName(id)}</strong>
        {#if meta}<span class="mono">{meta}</span>{/if}
        <button class="icon-btn" aria-label={t.unlink(encounterName(id))} onclick={() => unlink(scene.encounterIds, id)}>
          <i class="ti ti-x" aria-hidden="true"></i>
        </button>
      </span>
    {/each}
    <select id="link-enc" class="pick" value="" onchange={(ev) => pickLink(ev, scene.encounterIds)}>
      <option value="">{t.linkEncounter}</option>
      {#each encounterChoices.filter((x) => !scene.encounterIds.includes(x.id)) as x (x.id)}
        <option value={x.id}>{x.name}</option>
      {/each}
    </select>
  </div>
</section>

<section class="block" aria-label={t.checklist}>
  <div class="row">
    <h3 class="label"><i class="ti ti-list-check" aria-hidden="true"></i>{t.checklist}</h3>
    <span class="mono">{scene.checklist.filter((c) => c.done).length}/{scene.checklist.length}</span>
  </div>
  <Checklist bind:items={scene.checklist} placeholder={t.checkPlaceholder} />
</section>

<style>
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-3);
  }

  .title {
    flex: 1 1 var(--search-width);
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  #scene-title {
    width: 100%;
    min-height: var(--control-height-lg);
    padding: 0;
    border: 0;
    border-bottom: var(--border-width) solid var(--color-border-strong);
    background: transparent;
    color: var(--color-text);
    font: var(--text-h2);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .icon {
    padding: 0;
    width: var(--control-height);
    justify-content: center;
  }

  .danger {
    border-color: var(--color-danger);
    color: var(--color-danger);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  textarea {
    resize: vertical;
    font: inherit;
  }

  .label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .label .ti {
    font-size: var(--icon-size);
  }

  .small-label {
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .note {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--control-height);
    padding-left: var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    font-size: var(--text-sm);
  }

  .pick {
    min-height: var(--control-height);
    padding: 0 var(--space-2);
    border: var(--border-width) dashed var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    cursor: pointer;
  }

  .mono {
    color: var(--color-text-muted);
    font: var(--text-dice);
  }
</style>
