<script lang="ts">
  // The Sessions overview's Locations or NPCs: the campaign's tree with a search field (a match shows with its parents,
  // all open) and Add (under the selected entry, or at the top; opens it in the editor). Rows open and close (the
  // arrow) and select (the name, to highlight its sessions). The eye opens and closes a card with what it is; the
  // pencil (shown on hover or focus) opens it in the editor, in place of the sessions. NPCs sit in groups (folders,
  // which nest): Add group adds one, renamed right in its row; a group's pencil renames it there and its trash removes
  // it (what was in it moves up); renaming also shows Inside, to move it into another group from the keyboard. Any row
  // can be dragged: onto a location (into it), a group (into it), an NPC (beside
  // it, in its group) or the tree's free space (to the top); never into itself or what's under it.
  import { page } from "$app/state";
  import { tick } from "svelte";
  import { cubicInOut } from "svelte/easing";
  import { fade } from "svelte/transition";
  import { pictureSrc } from "$lib/api/campaigns";
  import type { Location, Npc, NpcGroup, World } from "$lib/api/world";
  import { cleanHtml, isEmpty } from "$lib/components/richtext";
  import "$lib/styles/prose.css";
  import { keepScroll } from "$lib/stores/history.svelte";
  import { reduceMotion } from "$lib/stores/theme.svelte";
  import { strings } from "$lib/strings";
  import { ancestors, descendants, dropParent, newGroup, newLocation, newNpc, NPC_DRAG, npcSpecs, removeGroup, treeOrder, treeRows } from "./sessions";

  let {
    kind,
    world = $bindable(),
    selected,
    onselect,
    onedit,
    embedded = false,
    ondragentry,
    ondragentryend,
  }: {
    kind: "location" | "npc";
    world: World;
    /** The selected entry of this tree, if any. */
    selected: string | null;
    onselect: (id: string | null) => void;
    /** Open the entry in the editor. */
    onedit: (id: string) => void;
    /** Inside a section that names it already (session prep): no title, border or Add buttons of its own (the
     *  section has them and calls `add` and `addGroup`). */
    embedded?: boolean;
    /** A location or NPC (not a group) picked up, to drop on a scene. */
    ondragentry?: (id: string, name: string) => void;
    ondragentryend?: () => void;
  } = $props();

  const w = strings.sessions.world;
  const lf = strings.sessions.locationFields;
  const e = strings.entry;
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
    if (lastFolder && groups.has(lastFolder)) return lastFolder;
    if (!selected || !list.some((e) => e.id === selected)) return null;
    if (kind === "location") return selected;
    return world.npcs.find((n) => n.id === selected)?.parent ?? null;
  }

  /** The group last opened or closed by a click (groups aren't selected): Add puts new things in it, until an NPC is
   *  selected. */
  let lastFolder = $state<string | null>(null);

  function clickRow(id: string, group: boolean) {
    if (group) {
      lastFolder = id;
      if (!query.trim()) toggle(id);
    } else {
      lastFolder = null;
      onselect(selected === id ? null : id);
    }
  }

  /** Shows the new entry: its parents open, the search cleared. */
  function reveal(parent: string | null) {
    if (parent) open = new Set([...open, parent, ...ancestors(list, parent).map((a) => a.id)]);
    query = "";
  }

  export function add() {
    const parent = parentFor();
    let created: Location | Npc;
    if (kind === "location") world.locations.push((created = newLocation(strings.sessions.scenes.newLocationName, parent)));
    else world.npcs.push((created = newNpc(w.newNpc, parent)));
    reveal(parent);
    onedit(created.id);
  }

  // ---- Groups: renamed in their row ----------------------------------------------------------------
  let renaming = $state<string | null>(null);
  /** The group's name and place when renaming began: Escape puts them back. */
  let before = { name: "", parent: null as string | null };

  export function addGroup() {
    const parent = parentFor();
    const created = newGroup(w.newGroup, parent);
    world.npcGroups.push(created);
    reveal(parent);
    rename(created);
  }

  function rename(group: NpcGroup) {
    [renaming, before] = [group.id, { name: group.name, parent: group.parent }];
  }

  /** Ends renaming; Escape puts back the name and place it had. */
  async function renamed(group: NpcGroup, keep: boolean) {
    if (renaming !== group.id) return;
    if (!keep) [group.name, group.parent] = [before.name, before.parent];
    renaming = null;
    await tick();
    document.querySelector<HTMLElement>(`[data-edit="${group.id}"]`)?.focus();
  }

  /** The groups a group can go into: never itself or one inside it. */
  function groupChoices(id: string) {
    const under = new Set(descendants(world.npcGroups, id));
    return treeOrder(world.npcGroups).filter((r) => !under.has(r.entry.id));
  }

  function moveGroup(group: NpcGroup, parent: string | null) {
    group.parent = parent;
    if (parent) open = new Set([...open, parent, ...ancestors(list, parent).map((a) => a.id)]);
  }

  function dropGroup(id: string) {
    if (selected === id) onselect(null);
    removeGroup(world, id);
  }

  // ---- Dragging to move: `dropOn` is the row the drop would go into (null: the tree itself) ----
  const MOVE = "application/x-koldron-tree-move";
  let moving = $state<string | null>(null);
  let dropOn = $state<string | null | undefined>(undefined);
  const canHold = (id: string) => kind === "location" || groups.has(id);

  function dragStart(ev: DragEvent, id: string) {
    if (!ev.dataTransfer) return;
    moving = id;
    ev.dataTransfer.setData(MOVE, id);
    // An NPC can also be dragged onto the location being edited, to link it.
    if (kind === "npc" && !groups.has(id)) ev.dataTransfer.setData(NPC_DRAG, id);
    ev.dataTransfer.effectAllowed = "copyMove";
    if (!groups.has(id)) ondragentry?.(id, list.find((e) => e.id === id)?.name || unnamed);
  }

  function dragOver(ev: DragEvent, target: string | null) {
    if (!moving) return;
    if (target !== null) ev.stopPropagation(); // a row, even one it can't go into, never the tree around it
    if (dropParent(list, moving, target, canHold) === undefined) return void (dropOn = undefined);
    ev.preventDefault();
    ev.dataTransfer!.dropEffect = "move";
    dropOn = target;
  }

  function drop(ev: DragEvent, target: string | null) {
    if (moving !== null && target !== null) ev.stopPropagation(); // let go on itself: nothing moves
    const parent = moving === null ? undefined : dropParent(list, moving, target, canHold);
    const entry = list.find((e) => e.id === moving);
    if (parent === undefined || !entry) return dragEnded();
    ev.preventDefault();
    entry.parent = parent;
    if (parent) open = new Set([...open, parent, ...ancestors(list, parent).map((a) => a.id)]); // the search stays
    dragEnded();
  }

  const dragEnded = () => {
    [moving, dropOn] = [null, undefined];
    ondragentryend?.();
  };

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

<section class="tree-panel" class:embedded aria-labelledby="{kind}-tree-title">
  <div class="head" class:sr-only={embedded}>
    <h2 id="{kind}-tree-title">{title}</h2>
    {#if !embedded}
      {#if kind === "npc"}
        <button class="btn icon" aria-label={w.addGroup} onclick={addGroup}>
          <i class="ti ti-folder-plus" aria-hidden="true"></i>
        </button>
      {/if}
      <button class="btn icon" aria-label={kind === "location" ? w.addLocation : w.addNpc} onclick={add}>
        <i class="ti ti-plus" aria-hidden="true"></i>
      </button>
    {/if}
  </div>
  <label for="{kind}-search" class="sr-only">{kind === "location" ? w.searchLocations : w.searchNpcs}</label>
  <input id="{kind}-search" class="text-field search" type="search" autocomplete="off" placeholder={kind === "location" ? w.searchLocations : w.searchNpcs} bind:value={query} />

  <!-- The tree scrolls inside the panel; its head and search stay. -->
  <!-- svelte-ignore a11y_no_static_element_interactions: dropping here moves a dragged row to the top; Inside (the editor's, or a group's while renaming) does the same from the keyboard -->
  <div
    class="scroll"
    class:drop-top={moving !== null && dropOn === null}
    use:keepScroll={{ id: `${kind}-tree`, url: page.url.href, ready: true }}
    ondragover={(ev) => dragOver(ev, null)}
    ondragleave={(ev) => !ev.currentTarget.contains(ev.relatedTarget as Node | null) && (dropOn = undefined)}
    ondrop={(ev) => drop(ev, null)}
  >
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
            class:drop-target={moving !== null && dropOn === r.entry.id}
            class:moving={moving === r.entry.id}
            style:padding-left="calc({r.depth} * var(--space-4))"
            draggable={renaming !== r.entry.id}
            ondragstart={(ev) => dragStart(ev, r.entry.id)}
            ondragend={dragEnded}
            ondragover={(ev) => dragOver(ev, r.entry.id)}
            ondrop={(ev) => drop(ev, r.entry.id)}
          >
            <!-- A group's folder takes the arrow's place (open while its contents show). -->
            {#if r.kids}
              <button class="arrow" aria-label={r.open ? w.collapse(name) : w.expand(name)} disabled={!!query.trim()} onclick={() => (group ? clickRow(r.entry.id, true) : toggle(r.entry.id))}>
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
              {@const g = r.entry as NpcGroup}
              <!-- The name and Inside: leaving both keeps them, Enter keeps them, Escape puts both back. -->
              <!-- svelte-ignore a11y_no_static_element_interactions: only notices the focus leaving its fields -->
              <span
                class="rename"
                onfocusout={(e) => !e.currentTarget.contains(e.relatedTarget as Node | null) && renamed(g, true)}
                onkeydown={(e) => {
                  if (e.key === "Enter") renamed(g, true);
                  if (e.key === "Escape") {
                    e.stopPropagation();
                    renamed(g, false);
                  }
                }}
              >
                <input class="text-field" aria-label={w.groupName} maxlength="100" autocomplete="off" bind:value={g.name} use:focusSelect />
                <select aria-label={w.inside} value={g.parent ?? ""} onchange={(e) => moveGroup(g, e.currentTarget.value || null)}>
                  <option value="">{w.topLevel}</option>
                  {#each groupChoices(g.id) as c (c.entry.id)}
                    <option value={c.entry.id}>{"  ".repeat(c.depth) + (c.entry.name || w.unnamedGroup)}</option>
                  {/each}
                </select>
              </span>
            {:else}
              <!-- A group's name opens and closes it (as its folder does); another entry's selects it. -->
              <button
                class="name"
                aria-expanded={group && r.kids ? r.open : undefined}
                onclick={() => clickRow(r.entry.id, group)}
              >
                <span class="text" class:parent={r.kids > 0}>{name}</span>
              </button>
            {/if}
            {#if group}
              {#if renaming !== r.entry.id}
                <button class="edit pencil" aria-label={w.removeGroup(name)} onclick={() => dropGroup(r.entry.id)}>
                  <i class="ti ti-trash" aria-hidden="true"></i>
                </button>
                <button class="edit pencil" data-edit={r.entry.id} aria-label={w.renameGroup(name)} onclick={() => rename(r.entry)}>
                  <i class="ti ti-pencil" aria-hidden="true"></i>
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
    {#if "atmosphere" in entry}
      <!-- A location: its first image (or its map) across the top, then what it's like and who's there. -->
      {@const banner = entry.pictures?.[0] ?? entry.map}
      {@const here = entry.npcIds.map((id) => world.npcs.find((n) => n.id === id)).filter((n) => !!n)}
      {#if banner}<img class="banner" src={pictureSrc(banner)} alt="" />{/if}
      <header>
        <h3>{entry.name || unnamed}</h3>
        {#if path.length}<p class="muted">{path.join(" › ")}</p>{/if}
      </header>
      <div class="preview-body">
        {#each [[lf.atmosphere, entry.atmosphere], [lf.purpose, entry.purpose]] as [label, text] (label)}
          {#if !isEmpty(text)}
            <section>
              <h4 class="small-label">{label}</h4>
              <div class="prose text-block">{@html cleanHtml(text)}</div>
            </section>
          {/if}
        {/each}
        {#if here.length}
          <p><span class="small-label">{lf.npcs}:</span> {here.map((n) => n.name || m.unnamedNpc).join(", ")}</p>
        {/if}
        {#if isEmpty(entry.atmosphere) && isEmpty(entry.purpose) && !here.length}<p class="muted">{w.nothingYet}</p>{/if}
      </div>
    {:else}
      <!-- An NPC: the portrait beside who they are, then their stats, summary and motivation. -->
      {@const stats = [[e.ac, entry.stats.ac], [e.hp, entry.stats.hp], [e.speed, entry.stats.speed], [e.cr, entry.stats.cr]].filter(([, v]) => v.trim())}
      <header class="npc-head">
        {#if entry.portrait}<img class="portrait" src={pictureSrc(entry.portrait)} alt="" />{/if}
        <div>
          <h3>{entry.name || unnamed}</h3>
          {#if npcSpecs(entry)}<p>{npcSpecs(entry)}</p>{/if}
          {#if path.length}<p class="muted">{path.join(" › ")}</p>{/if}
        </div>
      </header>
      <div class="preview-body">
        {#if stats.length}
          <p class="stats">
            {#each stats as [label, value], i (label)}{#if i}<span class="dot" aria-hidden="true">·</span>{/if}<span><strong>{label}</strong> {value}</span>{/each}
          </p>
        {/if}
        {#if !isEmpty(entry.summary)}<div class="prose text-block">{@html cleanHtml(entry.summary)}</div>{/if}
        {#if !isEmpty(entry.motivation)}
          <section>
            <h4 class="small-label">{strings.sessions.npcFields.motivation}</h4>
            <div class="prose text-block">{@html cleanHtml(entry.motivation)}</div>
          </section>
        {/if}
        {#if !stats.length && isEmpty(entry.summary) && isEmpty(entry.motivation)}<p class="muted">{w.nothingYet}</p>{/if}
      </div>
    {/if}
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

  .embedded {
    flex: 1;
    padding: 0;
    border: 0;
    background: none;
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

  /* Dragging: the row it would go into, and the tree itself when it would go to the top. */
  li.drop-target {
    background: var(--color-accent-soft);
    outline: var(--focus-ring) dashed var(--color-accent);
    outline-offset: calc(-1 * var(--focus-ring));
  }

  li.moving {
    opacity: 0.5;
  }

  /* A group being renamed: its name, and Inside beside it. */
  .rename {
    flex: 1;
    display: flex;
    gap: var(--space-1);
    min-width: 0;
  }

  .rename input {
    flex: 1;
    min-width: 0;
  }

  .rename input,
  .rename select {
    min-height: var(--control-height-sm);
    font-size: var(--text-sm);
  }

  .rename select {
    max-width: 45%;
  }

  .scroll.drop-top {
    outline: var(--focus-ring) dashed var(--color-accent);
    outline-offset: calc(-1 * var(--focus-ring));
    border-radius: var(--radius-sm);
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

  /* .prose centres itself at a reading width; in the card it fills the width, from the left. */
  .preview-body .prose {
    max-width: none;
    margin: 0;
    font-size: inherit; /* the card's size, as the rest of it */
  }

  /* Long words and names wrap inside the card. */
  .preview-card {
    overflow-wrap: anywhere;
  }

  .preview-body section {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  /* A location's picture across the top of the card. */
  .banner {
    display: block;
    width: 100%;
    height: var(--location-thumb-size);
    object-fit: cover;
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
  }

  /* An NPC's portrait beside their name. */
  .npc-head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .portrait {
    flex: none;
    width: var(--control-height-xl);
    height: var(--control-height-xl);
    object-fit: cover;
    border-radius: var(--radius-md);
  }

  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-2);
  }

  .dot {
    color: var(--color-text-muted);
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
