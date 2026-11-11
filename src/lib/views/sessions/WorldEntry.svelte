<script lang="ts">
  // One of the campaign's locations or NPCs, edited in place: name, Inside (its parent: never itself or one under it),
  // a location's own fields (LocationFields) or an NPC's (NpcFields), a note on where it's used (Manage locations / NPCs), and remove, which
  // asks first.
  // Manage locations / Manage NPCs show one per entry; the Sessions overview shows the one being edited, in place of the sessions.
  import { tick, type Snippet } from "svelte";
  import type { Location, Npc, World } from "$lib/api/world";
  import { strings } from "$lib/strings";
  import LocationFields from "./LocationFields.svelte";
  import NpcFields from "./NpcFields.svelte";
  import { descendants, treeOrder } from "./sessions";

  let {
    kind,
    world = $bindable(),
    id,
    note,
    onremove,
    end,
  }: {
    kind: "location" | "npc";
    world: World;
    id: string;
    /** Where it's used. */
    note?: string;
    /** Remove it (the caller unlinks it where it must). */
    onremove: (id: string) => void;
    /** More at the end of the first row (the overview's Close). */
    end?: Snippet;
  } = $props();

  const t = strings.sessions.manage;
  const w = strings.sessions.world;
  const prefix = $derived(kind === "location" ? "loc" : "npc");
  const list = $derived<(Location | Npc)[]>(kind === "location" ? world.locations : world.npcs);
  const entry = $derived(list.find((e) => e.id === id));
  const unnamed = $derived(kind === "location" ? t.unnamedLocation : t.unnamedNpc);
  const under = $derived(new Set(descendants(list, id)));
  /** A location is inside another location; an NPC inside a group. */
  const parents = $derived(treeOrder<{ id: string; parent: string | null; name: string }>(kind === "location" ? world.locations : world.npcGroups).filter((r) => !under.has(r.entry.id)));
  const indent = (depth: number, name: string) => "  ".repeat(depth) + name;

  let confirming = $state(false);
  let cancel: HTMLButtonElement | undefined = $state();
  let trash: HTMLButtonElement | undefined = $state();

  async function ask(yes: boolean) {
    confirming = yes;
    await tick();
    (yes ? cancel : trash)?.focus();
  }
</script>

{#if entry}
  <div class="entry">
    <div class="row">
      <div class="field grow">
        <label for="{prefix}-name-{id}" class="small-label">{t.name}</label>
        <input id="{prefix}-name-{id}" class="text-field strong" bind:value={entry.name} maxlength="100" autocomplete="off" />
      </div>
      <div class="field">
        <label for="{prefix}-parent-{id}" class="small-label">{w.inside}</label>
        <select id="{prefix}-parent-{id}" class="text-field" value={entry.parent ?? ""} onchange={(ev) => (entry.parent = ev.currentTarget.value || null)}>
          <option value="">{w.topLevel}</option>
          {#each parents as r (r.entry.id)}
            <option value={r.entry.id}>{indent(r.depth, r.entry.name || (kind === "location" ? unnamed : w.unnamedGroup))}</option>
          {/each}
        </select>
      </div>
      {#if !confirming}
        <button bind:this={trash} class="btn icon" aria-label={t.remove(entry.name || unnamed)} onclick={() => ask(true)}>
          <i class="ti ti-trash" aria-hidden="true"></i>
        </button>
      {/if}
      {@render end?.()}
    </div>
    {#if "atmosphere" in entry}
      <LocationFields bind:location={() => entry as Location, (v) => Object.assign(entry!, v)} {world} />
    {:else}
      <NpcFields bind:npc={() => entry as Npc, (v) => Object.assign(entry!, v)} />
    {/if}
    {#if note}<p class="note">{note}</p>{/if}
    {#if confirming}
      <div class="confirm" role="alert">
        <span>{w.confirmRemove(entry.name || unnamed)}</span>
        <button bind:this={cancel} class="btn" onclick={() => ask(false)}>{w.cancel}</button>
        <button class="btn btn-danger" onclick={() => onremove(id)}>{t.remove(entry.name || unnamed)}</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  /* Fills its panel, so the tabs (LocationFields, NpcFields) are as tall as it. */
  .entry {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-2);
  }

  .grow {
    flex: 1 1 var(--space-10);
    min-width: 0;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }

  .small-label {
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
  }

  .strong {
    font-weight: var(--weight-bold);
  }

  select {
    max-width: 100%;
  }

  .btn.icon {
    justify-content: center;
    width: var(--control-height);
    padding: 0;
  }

  .note {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .confirm {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  .confirm span {
    flex: 1;
  }
</style>
