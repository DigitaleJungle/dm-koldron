<script lang="ts">
  // The Sessions overview's Locations or NPCs: the campaign's tree with a search field (a match shows with its parents,
  // all open) and Add (under the selected entry, or at the top). Rows open and close (the arrow) and select (the name);
  // each shows how many entries are under it, or for one with none, in how many sessions it appears. The selected one
  // shows under the tree, edited in place.
  import { tick } from "svelte";
  import { page } from "$app/state";
  import type { SessionRow } from "$lib/api/sessions";
  import type { Location, Npc, World } from "$lib/api/world";
  import { keepScroll } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";
  import WorldEntry from "./WorldEntry.svelte";
  import { ancestors, descendants, newLocation, newNpc, removeLocation, removeNpc, treeRows } from "./sessions";

  let {
    kind,
    world = $bindable(),
    sessions,
    selected,
    onselect,
  }: {
    kind: "location" | "npc";
    world: World;
    sessions: SessionRow[];
    /** The selected entry of this tree, if any. */
    selected: string | null;
    onselect: (id: string | null) => void;
  } = $props();

  const w = strings.sessions.world;
  const m = strings.sessions.manage;
  const fmt = new Intl.NumberFormat();
  const list = $derived<(Location | Npc)[]>(kind === "location" ? world.locations : world.npcs);
  const title = $derived(kind === "location" ? strings.sessions.overview.locations : strings.sessions.overview.npcs);
  const unnamed = $derived(kind === "location" ? m.unnamedLocation : m.unnamedNpc);

  let query = $state("");
  let open = $state(new Set<string>());
  const rows = $derived(treeRows(list, open, query));

  /** The sessions the entry, or anything under it, appears in. */
  function sessionsWith(id: string): SessionRow[] {
    const ids = descendants(list, id);
    return sessions.filter((s) => (kind === "location" ? s.locationIds : s.npcIds).some((x) => ids.includes(x)));
  }

  function toggle(id: string) {
    const next = new Set(open);
    if (!next.delete(id)) next.add(id);
    open = next;
  }

  async function add() {
    const parent = selected && list.some((e) => e.id === selected) ? selected : null;
    let created: Location | Npc;
    if (kind === "location") world.locations.push((created = newLocation(strings.sessions.scenes.newLocationName, parent)));
    else world.npcs.push((created = newNpc(w.newNpc, parent)));
    if (parent) open = new Set([...open, parent, ...ancestors(list, parent).map((a) => a.id)]);
    query = "";
    onselect(created.id);
    await tick();
    const field = document.getElementById(`${kind === "location" ? "loc" : "npc"}-name-${created.id}`) as HTMLInputElement | null;
    field?.focus();
    field?.select();
  }

  function remove(id: string) {
    (kind === "location" ? removeLocation : removeNpc)(world, null, id);
    onselect(null);
  }

  const detail = $derived(selected ? list.find((e) => e.id === selected) : undefined);
  const path = $derived(detail ? ancestors(list, detail.id).map((a) => a.name || unnamed) : []);
  const detailNote = $derived.by(() => {
    if (!detail) return "";
    const numbers = sessionsWith(detail.id).map((s) => s.number).toSorted((a, b) => a - b);
    return numbers.length ? w.appearsIn(numbers.map((n) => fmt.format(n)).join(", ")) : w.notInSession;
  });
</script>

<section class="tree-panel" aria-labelledby="{kind}-tree-title">
  <div class="head">
    <h2 id="{kind}-tree-title">{title}</h2>
    <button class="btn icon" aria-label={kind === "location" ? w.addLocation : w.addNpc} onclick={add}>
      <i class="ti ti-plus" aria-hidden="true"></i>
    </button>
  </div>
  <label for="{kind}-search" class="sr-only">{kind === "location" ? w.searchLocations : w.searchNpcs}</label>
  <input id="{kind}-search" class="text-field search" type="search" autocomplete="off" placeholder={kind === "location" ? w.searchLocations : w.searchNpcs} bind:value={query} />

  <!-- The tree and the selected entry scroll inside the panel; its head and search stay. -->
  <div class="scroll" use:keepScroll={{ id: `${kind}-tree`, url: page.url.href, ready: true }}>
  {#if list.length === 0}
    <p class="note">{kind === "location" ? w.noLocations : w.noNpcs}</p>
  {:else if rows.length === 0}
    <p class="note" role="status">{w.noMatches}</p>
  {:else}
    <ul role="tree" aria-label={title}>
      {#each rows as r (r.entry.id)}
        {@const isSelected = r.entry.id === selected}
        {@const count = r.kids ? 0 : sessionsWith(r.entry.id).length}
        {@const name = r.entry.name || unnamed}
        <li
          role="treeitem"
          aria-level={r.depth + 1}
          aria-expanded={r.kids ? r.open : undefined}
          aria-selected={isSelected}
          class:selected={isSelected}
          style:padding-left="calc({r.depth} * var(--space-4))"
        >
          {#if r.kids}
            <button class="arrow" aria-label={r.open ? w.collapse(name) : w.expand(name)} disabled={!!query.trim()} onclick={() => toggle(r.entry.id)}>
              <i class="ti ti-chevron-right" class:turned={r.open} aria-hidden="true"></i>
            </button>
          {:else}
            <span class="arrow" aria-hidden="true"></span>
          {/if}
          <button class="name" onclick={() => onselect(isSelected ? null : r.entry.id)}>
            <span class="text" class:parent={r.kids > 0}>{name}</span>
            {#if r.kids}
              <span class="meta" aria-label={w.under(r.kids)}>{fmt.format(r.kids)}</span>
            {:else}
              <span class="meta" aria-label={w.inSessionsLabel(count)}>{count ? w.inSessions(count) : w.notInAny}</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if detail}
    <div class="detail">
      {#if path.length}<span class="path">{path.join(" › ")}</span>{/if}
      <WorldEntry {kind} bind:world id={detail.id} note={detailNote} onremove={remove} />
    </div>
  {/if}
  </div>
</section>

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

  .scroll {
    flex: 1;
    min-height: 0;
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
    justify-content: space-between;
    gap: var(--space-2);
  }

  h2 {
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

  li.selected {
    background: var(--color-accent-soft);
  }

  .arrow {
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

  .meta {
    flex: none;
    color: var(--color-text-muted);
    font: var(--text-xs) / 1 var(--font-mono);
  }

  .note {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-1);
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
  }

  .path {
    color: var(--color-text-muted);
    font-size: var(--text-xs);
  }
</style>
