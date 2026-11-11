<script lang="ts">
  // The Sessions overview's Locations or NPCs: the campaign's tree with a search field (a match shows with its parents,
  // all open) and Add (under the selected entry, or at the top; opens it in the editor). Rows open and close (the
  // arrow) and select (the name, to highlight its sessions). The eye opens and closes a card with what it is; the
  // pencil (shown on hover or focus) opens it in the editor, in place of the sessions. NPCs sit in groups (folders,
  // which nest): Add group adds one, renamed right in its row; a group's pencil renames it there and its trash removes
  // it (what was in it moves up).
  import { page } from "$app/state";
  import { tick } from "svelte";
  import { cubicInOut } from "svelte/easing";
  import { fade } from "svelte/transition";
  import type { Location, Npc, NpcGroup, World } from "$lib/api/world";
  import { cleanHtml, isEmpty } from "$lib/components/richtext";
  import "$lib/styles/prose.css";
  import { keepScroll } from "$lib/stores/history.svelte";
  import { reduceMotion } from "$lib/stores/theme.svelte";
  import { strings } from "$lib/strings";
  import { ancestors, newGroup, newLocation, newNpc, NPC_DRAG, npcSpecs, removeGroup, treeRows } from "./sessions";

  let {
    kind,
    world = $bindable(),
    selected,
    onselect,
    onedit,
  }: {
    kind: "location" | "npc";
    world: World;
    /** The selected entry of this tree, if any. */
    selected: string | null;
    onselect: (id: string | null) => void;
    /** Open the entry in the editor. */
    onedit: (id: string) => void;
  } = $props();

  const w = strings.sessions.world;
  const m = strings.sessions.manage;
  /** The tree's entries: the locations, or the NPC groups and the NPCs. */
  const list = $derived<(Location | Npc | NpcGroup)[]>(kind === "location" ? world.locations : [...world.npcGroups, ...world.npcs]);
  const groups = $derived(new Set(world.npcGroups.map((g) => g.id)));
  const title = $derived(kind === "location" ? strings.sessions.overview.locations : strings.sessions.overview.npcs);
  const unnamed = $derived(kind === "location" ? m.unnamedLocation : m.unnamedNpc);

  let query = $state("");
  let open = $state(new Set<string>());
  const rows = $derived(treeRows(list, open, query));

  function toggle(id: string) {
    const next = new Set(open);
    if (!next.delete(id)) next.add(id);
    open = next;
  }

  /** Where something new goes: in the selected location or group (an NPC's group when an NPC is selected), or at the top. */
  function parentFor(): string | null {
    if (!selected || !list.some((e) => e.id === selected)) return null;
    if (kind === "location" || groups.has(selected)) return selected;
    return world.npcs.find((n) => n.id === selected)?.parent ?? null;
  }

  /** Shows the new entry: its parents open, the search cleared. */
  function reveal(parent: string | null) {
    if (parent) open = new Set([...open, parent, ...ancestors(list, parent).map((a) => a.id)]);
    query = "";
  }

  function add() {
    const parent = parentFor();
    let created: Location | Npc;
    if (kind === "location") world.locations.push((created = newLocation(strings.sessions.scenes.newLocationName, parent)));
    else world.npcs.push((created = newNpc(w.newNpc, parent)));
    reveal(parent);
    onedit(created.id);
  }

  // ---- Groups: renamed in their row ----------------------------------------------------------------
  let renaming = $state<string | null>(null);
  let before = "";

  function addGroup() {
    const parent = parentFor();
    const created = newGroup(w.newGroup, parent);
    world.npcGroups.push(created);
    reveal(parent);
    rename(created);
  }

  function rename(group: { id: string; name: string }) {
    [renaming, before] = [group.id, group.name];
  }

  /** Ends renaming; Escape puts back the name it had. */
  async function renamed(group: { id: string; name: string }, keep: boolean) {
    if (renaming !== group.id) return;
    if (!keep) group.name = before;
    renaming = null;
    await tick();
    document.querySelector<HTMLElement>(`[data-edit="${group.id}"]`)?.focus();
  }

  function dropGroup(id: string) {
    if (selected === id) onselect(null);
    removeGroup(world, id);
  }

  const focusSelect = (node: HTMLInputElement) => {
    node.focus();
    node.select();
  };

  // ---- The card: hovering or focusing the eye opens it, leaving the eye and the card closes it (a click toggles it),
  // left of the eye (right of it only when there's no room), so it never
  // covers the eyes and pencils of the rows above and below. It fades in and out; Escape or scrolling closes it. ----
  const GAP = 12;
  const cardId = $props.id();
  let card: HTMLElement | undefined = $state();
  let shown = $state<{ id: string; eye: HTMLElement } | null>(null);
  const fadeIt = () => ({ duration: reduceMotion() ? 0 : 150, easing: cubicInOut });

  const HIDE_MS = 150;
  let timer: ReturnType<typeof setTimeout> | undefined;

  function show(id: string, eye: HTMLElement) {
    clearTimeout(timer);
    if (shown?.id !== id) shown = { id, eye };
  }

  /** After a moment, so the pointer can move from the eye into the card. */
  function hide() {
    clearTimeout(timer);
    timer = setTimeout(() => (shown = null), HIDE_MS);
  }

  function toggleCard(id: string, eye: HTMLElement) {
    clearTimeout(timer);
    shown = shown?.id === id ? null : { id, eye };
  }

  function close() {
    clearTimeout(timer);
    shown?.eye.focus();
    shown = null;
  }
  $effect(() => () => clearTimeout(timer));

  $effect(() => {
    if (!shown || !card) return;
    if (!card.matches(":popover-open")) card.showPopover();
    const at = shown.eye.getBoundingClientRect();
    const [w, h] = [card.offsetWidth, card.offsetHeight];
    const left = at.left - GAP - w >= 8 ? at.left - GAP - w : Math.min(at.right + GAP, innerWidth - w - 8);
    card.style.left = `${left}px`;
    card.style.top = `${Math.max(8, Math.min(at.top + at.height / 2 - h / 2, innerHeight - h - 8))}px`;
  });

  // Groups have no card.
  const entry = $derived(shown ? (kind === "location" ? world.locations : world.npcs as (Location | Npc)[]).find((e) => e.id === shown!.id) : undefined);
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && shown && close()} onscrollcapture={(e) => shown && !card?.contains(e.target as Node) && (shown = null)} />

<section class="tree-panel" aria-labelledby="{kind}-tree-title">
  <div class="head">
    <h2 id="{kind}-tree-title">{title}</h2>
    {#if kind === "npc"}
      <button class="btn icon" aria-label={w.addGroup} onclick={addGroup}>
        <i class="ti ti-folder-plus" aria-hidden="true"></i>
      </button>
    {/if}
    <button class="btn icon" aria-label={kind === "location" ? w.addLocation : w.addNpc} onclick={add}>
      <i class="ti ti-plus" aria-hidden="true"></i>
    </button>
  </div>
  <label for="{kind}-search" class="sr-only">{kind === "location" ? w.searchLocations : w.searchNpcs}</label>
  <input id="{kind}-search" class="text-field search" type="search" autocomplete="off" placeholder={kind === "location" ? w.searchLocations : w.searchNpcs} bind:value={query} />

  <!-- The tree scrolls inside the panel; its head and search stay. -->
  <div class="scroll" use:keepScroll={{ id: `${kind}-tree`, url: page.url.href, ready: true }}>
    {#if list.length === 0}
      <p class="note">{kind === "location" ? w.noLocations : w.noNpcs}</p>
    {:else if rows.length === 0}
      <p class="note" role="status">{w.noMatches}</p>
    {:else}
      <ul role="tree" aria-label={title}>
        {#each rows as r (r.entry.id)}
          {@const isSelected = r.entry.id === selected}
          {@const cardOpen = shown?.id === r.entry.id}
          {@const group = groups.has(r.entry.id)}
          {@const name = r.entry.name || (group ? w.unnamedGroup : unnamed)}
          <li
            role="treeitem"
            aria-level={r.depth + 1}
            aria-expanded={r.kids ? r.open : undefined}
            aria-selected={isSelected}
            class:selected={isSelected}
            style:padding-left="calc({r.depth} * var(--space-4))"
            draggable={kind === "npc" && !group}
            ondragstart={(ev) => {
              // An NPC can be dragged onto the location being edited, to link it.
              ev.dataTransfer?.setData(NPC_DRAG, r.entry.id);
              if (ev.dataTransfer) ev.dataTransfer.effectAllowed = "copy";
            }}
          >
            <!-- A group's folder takes the arrow's place (open while its contents show). -->
            {#if r.kids}
              <button class="arrow" aria-label={r.open ? w.collapse(name) : w.expand(name)} disabled={!!query.trim()} onclick={() => toggle(r.entry.id)}>
                {#if group}
                  <i class="ti {r.open ? 'ti-folder-open' : 'ti-folder'}" aria-hidden="true"></i>
                {:else}
                  <i class="ti ti-chevron-right" class:turned={r.open} aria-hidden="true"></i>
                {/if}
              </button>
            {:else}
              <span class="arrow" aria-hidden="true">{#if group}<i class="ti ti-folder"></i>{/if}</span>
            {/if}
            {#if renaming === r.entry.id}
              <input
                class="text-field rename"
                aria-label={w.groupName}
                maxlength="100"
                autocomplete="off"
                bind:value={r.entry.name}
                use:focusSelect
                onkeydown={(e) => {
                  if (e.key === "Enter") renamed(r.entry, true);
                  if (e.key === "Escape") {
                    e.stopPropagation();
                    renamed(r.entry, false);
                  }
                }}
                onblur={() => renamed(r.entry, true)}
              />
            {:else}
              <button class="name" onclick={() => onselect(isSelected ? null : r.entry.id)}>
                <span class="text" class:parent={r.kids > 0}>{name}</span>
              </button>
            {/if}
            {#if group}
              {#if renaming !== r.entry.id}
                <button class="edit pencil" data-edit={r.entry.id} aria-label={w.renameGroup(name)} onclick={() => rename(r.entry)}>
                  <i class="ti ti-pencil" aria-hidden="true"></i>
                </button>
                <button class="edit pencil" aria-label={w.removeGroup(name)} onclick={() => dropGroup(r.entry.id)}>
                  <i class="ti ti-trash" aria-hidden="true"></i>
                </button>
              {/if}
            {:else}
            <button class="edit" aria-label={w.details(name)} aria-expanded={cardOpen} aria-controls={cardOpen ? `${cardId}-card` : undefined} onclick={(e) => toggleCard(r.entry.id, e.currentTarget)}
              onmouseenter={(e) => show(r.entry.id, e.currentTarget)}
              onmouseleave={hide}
              onfocus={(e) => show(r.entry.id, e.currentTarget)}
              onblur={hide}
            >
              <i class="ti {cardOpen ? 'ti-eye-off' : 'ti-eye'}" aria-hidden="true"></i>
            </button>
            <button class="edit pencil" data-edit={r.entry.id} aria-label={w.edit(name)} onclick={() => onedit(r.entry.id)}>
              <i class="ti ti-pencil" aria-hidden="true"></i>
            </button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</section>

{#if shown && entry}
  {@const path = ancestors(list, entry.id).map((a) => a.name || unnamed)}
  <!-- svelte-ignore a11y_no_static_element_interactions: hovering keeps it open, nothing to press -->
  <div
    bind:this={card}
    id="{cardId}-card"
    popover="manual"
    class="preview-card"
    role="region"
    aria-label={entry.name || unnamed}
    transition:fade={fadeIt()}
    onmouseenter={() => clearTimeout(timer)}
    onmouseleave={hide}
  >
    <header>
      <h3>{entry.name || unnamed}</h3>
      {#if path.length}<p class="muted">{path.join(" › ")}</p>{/if}
    </header>
    <div class="preview-body">
      {#if "atmosphere" in entry}
        {#if !isEmpty(entry.atmosphere)}<div class="prose text-block">{@html cleanHtml(entry.atmosphere)}</div>{/if}
      {:else}
        {#if npcSpecs(entry)}<p class="muted">{npcSpecs(entry)}</p>{/if}
        {#if !isEmpty(entry.summary)}<div class="prose text-block">{@html cleanHtml(entry.summary)}</div>{/if}
        {#if !isEmpty(entry.motivation)}
          <h4 class="small-label">{strings.sessions.npcFields.motivation}</h4>
          <div class="prose text-block">{@html cleanHtml(entry.motivation)}</div>
        {/if}
      {/if}
    </div>
  </div>
{/if}

<style>
  .tree-panel {
    box-sizing: border-box;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  /* Out to the panel's edges, so the rows start at its left; its notes keep the panel's inset. */
  .scroll {
    flex: 1;
    min-height: 0;
    margin-inline: calc(var(--space-1) - var(--space-4));
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  /* A narrow window scrolls as a whole. */
  @media (max-width: 900px) {
    .scroll {
      overflow: visible;
    }
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  /* The buttons together on the right. */
  h2 {
    flex: 1;
    margin: 0;
    font: var(--text-h2);
  }

  .btn.icon {
    width: var(--control-height);
    padding: 0;
  }

  .search {
    width: 100%;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  li {
    display: flex;
    align-items: center;
    border-radius: var(--radius-sm);
  }

  li[draggable="true"] {
    cursor: grab;
  }

  li.selected {
    background: var(--color-accent-soft);
  }

  .arrow,
  .edit {
    display: grid;
    place-items: center;
    flex: none;
    width: var(--control-height-sm);
    height: var(--control-height-sm);
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .arrow:disabled {
    cursor: default;
  }

  /* The pencil shows while the row is hovered or has focus. */
  .pencil {
    opacity: 0;
  }

  li:hover .pencil,
  li:focus-within .pencil {
    opacity: 1;
  }

  .edit[aria-expanded="true"] {
    color: var(--color-text);
  }

  .edit:hover {
    color: var(--color-text);
  }

  .turned {
    transform: rotate(90deg);
  }

  .arrow i {
    transition: transform 0.15s;
  }

  :global([data-motion="reduce"]) .arrow i {
    transition: none;
  }

  .name {
    flex: 1;
    min-width: 0;
    min-height: var(--control-height-sm);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: 0 var(--space-2);
    border: 0;
    background: transparent;
    color: var(--color-text);
    font: var(--text-body-sm);
    text-align: left;
    cursor: pointer;
  }

  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .text.parent {
    font-weight: var(--weight-bold);
  }

  .note {
    margin: 0;
    padding-inline: calc(var(--space-4) - var(--space-1));
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  /* The card */
  header {
    padding: var(--space-2) var(--space-4);
    border-bottom: var(--border-width) solid var(--color-border);
  }

  header p,
  .preview-body p {
    margin: 0;
  }

  .preview-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .text-block {
    white-space: pre-line;
  }

  .small-label {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
  }

  .muted {
    color: var(--color-text-muted);
  }
</style>
