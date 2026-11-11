<script lang="ts">
  // Manage locations / Manage NPCs: the campaign's, in tree order, with where this session uses each; a new one by name,
  // at the top, opened at once. Both are a list: one opens in the full editor (as on the Sessions overview), with a
  // cross back to the list. Removing one unlinks it from this session's scenes (what was under it moves up).
  import { tick } from "svelte";
  import type { Session } from "$lib/api/sessions";
  import type { Location, Npc, World } from "$lib/api/world";
  import { strings } from "$lib/strings";
  import WorldEntry from "./WorldEntry.svelte";
  import { newLocation, newNpc, removeLocation, removeNpc, scenesWith, treeOrder } from "./sessions";

  let {
    kind,
    session = $bindable(),
    world = $bindable(),
    editing = $bindable(null),
  }: {
    kind: "location" | "npc";
    session: Session;
    world: World;
    /** The location or NPC open in the editor, if any. */
    editing?: string | null;
  } = $props();

  const t = strings.sessions.manage;
  /** The NPCs with their groups, which show as headings. */
  const rows = $derived(treeOrder<{ id: string; parent: string | null; name: string }>(kind === "location" ? world.locations : [...world.npcGroups, ...world.npcs]));
  const groups = $derived(new Set(world.npcGroups.map((g) => g.id)));
  const open = $derived(editing && rows.some((r) => r.entry.id === editing && !groups.has(editing)) ? editing : null);
  let draft = $state("");

  async function add(ev: SubmitEvent) {
    ev.preventDefault();
    const name = draft.trim();
    if (!name) return;
    draft = "";
    const created = kind === "location" ? newLocation(name) : newNpc(name);
    if (kind === "location") world.locations.push(created as Location);
    else world.npcs.push(created as Npc);
    await edit(created.id);
  }

  async function edit(id: string) {
    editing = id;
    await tick();
    document.getElementById(`${kind === "location" ? "loc" : "npc"}-name-${id}`)?.focus();
  }

  async function back() {
    const id = editing;
    editing = null;
    await tick();
    document.querySelector<HTMLElement>(`[data-location-row="${id}"]`)?.focus();
  }

  function note(id: string) {
    const where = scenesWith(session, { kind, id }).map((sc) => sc.title || strings.sessions.scenes.untitled);
    if (kind === "location") return where.length ? t.usedIn(where.join(", ")) : t.notUsed;
    return where.length ? t.appearsIn(where.join(", ")) : t.notLinked;
  }

  function remove(id: string) {
    (kind === "location" ? removeLocation : removeNpc)(world, session, id);
    if (editing === id) editing = null;
  }
</script>

{#if open}
  <WorldEntry {kind} bind:world id={open} note={note(open)} onremove={remove}>
    {#snippet end()}
      <button class="icon-btn icon-btn-lg" aria-label={strings.sessions.world.close} onclick={back}>
        <i class="ti ti-x" aria-hidden="true"></i>
      </button>
    {/snippet}
  </WorldEntry>
{:else}
  <h2>{kind === "location" ? strings.sessions.sections.locations : strings.sessions.sections.npcs}</h2>
  {#if rows.length}
    <ul class="locations">
      {#each rows as r (r.entry.id)}
        <li style:margin-left="calc({r.depth} * var(--space-5))">
          {#if groups.has(r.entry.id)}
            <h3 class="group"><i class="ti ti-folder" aria-hidden="true"></i>{r.entry.name || strings.sessions.world.unnamedGroup}</h3>
          {:else}
          <button class="location" data-location-row={r.entry.id} onclick={() => edit(r.entry.id)}>
            <strong>{r.entry.name || (kind === "location" ? t.unnamedLocation : t.unnamedNpc)}</strong>
            <span class="note">{note(r.entry.id)}</span>
          </button>
          {/if}
        </li>
      {/each}
    </ul>
  {:else}
    <p class="note">{kind === "location" ? t.noLocations : t.noNpcs}</p>
  {/if}
  <form class="add" onsubmit={add}>
    <input class="text-field" bind:value={draft} maxlength="100" autocomplete="off" placeholder={kind === "location" ? t.newLocation : t.newNpc} aria-label={kind === "location" ? t.newLocation : t.newNpc} />
    <button class="btn" type="submit" disabled={!draft.trim()}>{kind === "location" ? t.addLocation : t.addNpc}</button>
  </form>
{/if}

<style>
  .locations {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .location {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-1);
    width: 100%;
    padding: var(--space-3) var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    color: var(--color-text);
    text-align: left;
    cursor: pointer;
  }

  .group {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: var(--space-2) 0 0;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .location:hover {
    border-color: var(--color-border-strong);
  }
</style>
