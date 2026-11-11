<script lang="ts">
  // The open scene: its title and Ready (an icon); what happens; one block with its location (and that location's
  // description), linked NPCs and linked encounters (chips and a dropdown each, so dragging is never the only way); its
  // checklist; and Remove scene at the bottom. A new location opens Locations to name it.
  import { untrack } from "svelte";
  import type { Session } from "$lib/api/sessions";
  import { strings } from "$lib/strings";
  import RichText from "$lib/components/RichText.svelte";
  import Checklist from "./Checklist.svelte";
  import { link, newLocation, unlink } from "./sessions";

  let {
    session = $bindable(),
    index,
    encounterName,
    encounterMeta,
    encounterChoices,
    onnewlocation,
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
    /** A location was just made for this scene: name it. */
    onnewlocation: (id: string) => void;
    onremove: () => void;
  } = $props();

  const t = strings.sessions.scenes;
  const m = strings.sessions.manage;
  const scene = $derived(session.scenes[index]);
  const location = $derived(session.locations.find((l) => l.id === scene.locationId) ?? null);
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

  function pickLocation(ev: Event & { currentTarget: HTMLSelectElement }) {
    const value = ev.currentTarget.value;
    if (value === "__new") {
      const created = newLocation(t.newLocationName);
      session.locations.push(created);
      scene.locationId = created.id;
      onnewlocation(created.id);
    } else scene.locationId = value || null;
  }

  function pickLink(ev: Event & { currentTarget: HTMLSelectElement }, ids: string[]) {
    const value = ev.currentTarget.value;
    if (value) link(ids, value);
    ev.currentTarget.value = "";
  }
</script>

<div class="head">
  <input id="scene-title" bind:this={titleInput} bind:value={scene.title} maxlength="100" autocomplete="off" placeholder={t.untitled} aria-label={t.titleField} />
  <button class="btn icon ready" aria-pressed={scene.ready} aria-label={t.ready} onclick={() => (scene.ready = !scene.ready)}>
    <i class="ti ti-check" aria-hidden="true"></i>
  </button>
</div>

<div class="field">
  <span class="label">{t.whatHappens}</span>
  <!-- Keyed: the field fills itself once, so another scene needs a fresh one. -->
  {#key scene.id}
    <RichText label={t.whatHappens} extended bind:value={scene.summary} />
  {/key}
</div>

<!-- One block, a row each: the label on the left, its controls on the right. -->
<section class="block links" aria-label={t.links}>
  <div class="link-row">
    <label for="scene-loc" class="label"><i class="ti ti-map-pin" aria-hidden="true"></i>{t.location}</label>
    <div class="controls">
      <select id="scene-loc" class="choose" value={scene.locationId ?? ""} onchange={pickLocation}>
        <option value="">{t.noLocation}</option>
        {#each session.locations as l (l.id)}
          <option value={l.id}>{l.name || m.unnamedLocation}</option>
        {/each}
        <option value="__new">{t.newLocation}</option>
      </select>
      {#if location?.description}<p class="note">{location.description}</p>{/if}
    </div>
  </div>
  <div class="link-row">
    <label for="link-npc" class="label"><i class="ti ti-user" aria-hidden="true"></i>{t.npcs}</label>
    <div class="controls chips">
      {#each scene.npcIds as id (id)}
        <span class="link">
          <strong>{npcName(id)}</strong>
          <button class="icon-btn" aria-label={t.unlink(npcName(id))} onclick={() => unlink(scene.npcIds, id)}>
            <i class="ti ti-x" aria-hidden="true"></i>
          </button>
        </span>
      {/each}
      <select id="link-npc" class="pick" value="" onchange={(ev) => pickLink(ev, scene.npcIds)}>
        <option value="">{t.link}</option>
        {#each session.npcs.filter((n) => !scene.npcIds.includes(n.id)) as n (n.id)}
          <option value={n.id}>{n.name || m.unnamedNpc}</option>
        {/each}
      </select>
    </div>
  </div>
  <div class="link-row">
    <label for="link-enc" class="label"><i class="ti ti-sword" aria-hidden="true"></i>{t.encounters}</label>
    <div class="controls chips">
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
        <option value="">{t.link}</option>
        {#each encounterChoices.filter((x) => !scene.encounterIds.includes(x.id)) as x (x.id)}
          <option value={x.id}>{x.name}</option>
        {/each}
      </select>
    </div>
  </div>
</section>

<section class="block" aria-labelledby="scene-checklist">
  <div class="row">
    <h3 id="scene-checklist" class="label"><i class="ti ti-list-check" aria-hidden="true"></i>{t.checklist}</h3>
    <span class="mono">{scene.checklist.filter((c) => c.done).length}/{scene.checklist.length}</span>
  </div>
  <Checklist bind:items={scene.checklist} placeholder={strings.sessions.checklist.short} />
</section>

<div class="remove">
  {#if confirming}
    <button class="btn" onclick={() => (confirming = false)}>{strings.sessions.cancel}</button>
    <button class="btn btn-danger" onclick={onremove}>{t.remove}</button>
  {:else}
    <button class="btn danger" onclick={() => (confirming = true)}>{t.remove}</button>
  {/if}
</div>

<style>
  /* The title and the Ready icon on one line, centred on each other. */
  .head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  #scene-title {
    flex: 1;
    min-width: 0;
    min-height: var(--control-height-lg);
    padding: 0;
    border: 0;
    border-bottom: var(--border-width) solid var(--color-border-strong);
    background: transparent;
    color: var(--color-text);
    font: var(--text-h2);
  }

  .remove {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .ready[aria-pressed="true"] {
    border-color: var(--color-accent);
    background: var(--color-accent);
    color: var(--on-fill);
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

  .label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
  }

  /* The scene's links: a row each, divided by lines, the labels in one column. */
  .links {
    gap: 0;
    padding-block: var(--space-1);
  }

  .link-row {
    display: grid;
    grid-template-columns: var(--scene-label-width) minmax(0, 1fr);
    align-items: start;
    gap: var(--space-3);
    padding-block: var(--space-2);
  }

  .link-row + .link-row {
    border-top: var(--border-width) solid var(--color-border);
  }

  .link-row .label {
    min-height: var(--control-height);
  }

  .controls {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-1);
  }

  .choose {
    min-height: var(--control-height);
    padding: 0 var(--space-2);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
    font-weight: var(--weight-bold);
    cursor: pointer;
  }

  .label .ti {
    font-size: var(--icon-size);
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

  .controls.chips {
    flex-direction: row;
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
