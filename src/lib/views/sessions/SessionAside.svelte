<script lang="ts">
  // Across session: Previously, Premise, Summary, Party note and Checklist on the right, Locations, NPCs and Encounters under
  // the scenes (the page renders this twice, sharing `open`), one open at a time (opening one
  // closes the other; the open one closes again). Locations and NPCs are the campaign's trees, as on the Sessions
  // overview (a pencil opens one in the editor); their rows and the encounter cards (the campaign's encounters that are
  // ready to run) can be dragged onto a scene. Manage checklist opens its full view in the editor; Manage encounters
  // opens the Encounters screen.
  import type { Encounter } from "$lib/api/encounters";
  import type { Session } from "$lib/api/sessions";
  import type { World } from "$lib/api/world";
  import { strings } from "$lib/strings";
  import Checklist from "./Checklist.svelte";
  import WorldTree from "./WorldTree.svelte";
  import { cubicInOut } from "svelte/easing";
  import { slide } from "svelte/transition";
  import { reduceMotion } from "$lib/stores/theme.svelte";
  import { linkedIn, toggleSection, type Linkable, type Pane, type Section } from "./sessions";

  let {
    session = $bindable(),
    world = $bindable(),
    open = $bindable(),
    lastDrop,
    ready,
    encounterError,
    encountersHref,
    encounterMeta,
    onmanage,
    onedit,
    ondragstart,
    ondragend,
    keys,
    heading = true,
  }: {
    session: Session;
    world: World;
    open: Section | null;
    /** What the last drop added, said once. */
    lastDrop: string;
    /** The campaign's encounters that are ready to run; null until they have loaded. */
    ready: Encounter[] | null;
    encounterError: string | null;
    encountersHref: string;
    encounterMeta: (id: string) => string | null;
    onmanage: (pane: Pane) => void;
    /** Open a location or NPC in the editor. */
    onedit: (kind: "location" | "npc", id: string) => void;
    ondragstart: (item: Linkable, name: string) => void;
    ondragend: () => void;
    /** Which sections this column shows, in order. */
    keys: Section[];
    /** The "Across session" heading and the drop message; only one column shows them. */
    heading?: boolean;
  } = $props();

  const t = strings.sessions.sections;
  const fmt = new Intl.NumberFormat();
  const written = (text: string) => (text.trim() ? t.written : "");

  const encounterCards = $derived((ready ?? []).map((x) => ({ item: { kind: "encounter", id: x.id } as Linkable, name: x.name, sub: encounterMeta(x.id) ?? "" })));
  /** Each tree's selected entry: Add puts new ones under it. */
  let picked = $state<Record<"location" | "npc", string | null>>({ location: null, npc: null });
  let trees = $state<Record<"location" | "npc", WorldTree | undefined>>({ location: undefined, npc: undefined });
  const w = strings.sessions.world;
  /** Opening grows the section, closing shrinks it; not with animations off. */
  const grow = () => ({ duration: reduceMotion() ? 0 : 200, easing: cubicInOut });
  let encounterQuery = $state("");
  const shownEncounters = $derived(encounterCards.filter((c) => c.name.toLowerCase().includes(encounterQuery.trim().toLowerCase())));


  /** Beside the name: how many the session's scenes link (removed locations and NPCs don't count). */
  const added = (n: number) => t.added(fmt.format(n));
  const counts = $derived<Partial<Record<Section, string>>>({
    locations: added(linkedIn(session, "location").filter((id) => world.locations.some((l) => l.id === id)).length),
    npcs: added(linkedIn(session, "npc").filter((id) => world.npcs.some((n) => n.id === id)).length),
    encounters: added(linkedIn(session, "encounter").length),
  });

  const metas = $derived<Partial<Record<Section, string>>>({
    previously: written(session.previously),
    premise: written(session.premise),
    partyNote: written(session.partyNote),
    checklist: "",
  });
</script>

<aside aria-labelledby={heading ? "across-session" : undefined} aria-label={heading ? undefined : t.linkTitle}>
  {#if heading}
    <h2 id="across-session" class="label">{t.title}</h2>
    <p class="dropped" role="status">
      {#if lastDrop}<i class="ti ti-check" aria-hidden="true"></i>{lastDrop}{/if}
    </p>
  {/if}

  {#each keys as key (key)}
    {@const actions = open !== key ? 0 : key === "npcs" || key === "encounters" ? 2 : key === "locations" ? 1 : 0}
    {@const fixed = key === "locations" || key === "npcs" || key === "encounters"}
    <div class="section" class:grows={fixed && open === key}>
      <div class="head">
        <button class="toggle" aria-expanded={open === key} aria-controls="section-{key}" onclick={() => (open = toggleSection(open, key))}>
          <span class="text" style:--actions={actions}>
            <span class="name">
              {t[key]}
              {#if counts[key]}<span class="count">{counts[key]}</span>{/if}
            </span>
            {#if metas[key]}<span class="meta">{metas[key]}</span>{/if}
          </span>
          <i class="ti ti-chevron-down chevron" class:turned={open === key} aria-hidden="true"></i>
        </button>
        <!-- Beside the arrow while open; outside the toggle, as a button can't hold buttons. -->
        {#if open === key && key === "encounters"}
          <span class="actions">
            <a class="icon-btn" href={encountersHref} aria-label={t.manage.encounters}>
              <i class="ti ti-external-link" aria-hidden="true"></i>
            </a>
            <a class="icon-btn" href="{encountersHref}/new" aria-label={t.newEncounter}>
              <i class="ti ti-plus" aria-hidden="true"></i>
            </a>
          </span>
        {:else if open === key && (key === "locations" || key === "npcs")}
          <span class="actions">
            {#if key === "npcs"}
              <button class="icon-btn" aria-label={w.addGroup} onclick={() => trees.npc?.addGroup()}>
                <i class="ti ti-folder-plus" aria-hidden="true"></i>
              </button>
            {/if}
            <button class="icon-btn" aria-label={key === "locations" ? w.addLocation : w.addNpc} onclick={() => trees[key === "locations" ? "location" : "npc"]?.add()}>
              <i class="ti ti-plus" aria-hidden="true"></i>
            </button>
          </span>
        {/if}
      </div>
      {#if open === key}
        <div id="section-{key}" class="body" class:fixed transition:slide={grow()}>
          {#if key === "previously" || key === "premise" || key === "partyNote"}
            <label for="text-{key}" class="sr-only">{t[key]}</label>
            <textarea id="text-{key}" class="text-field" rows="6" maxlength="20000" bind:value={session[key]}></textarea>
          {:else if key === "checklist"}
            <Checklist bind:items={session.checklist} placeholder={strings.sessions.checklist.short} compact />
            <button class="btn small" onclick={() => onmanage("checklist")}>{t.manage.checklist}</button>
          {:else if key === "encounters" && (encounterError || ready === null || ready.length === 0)}
            <p class:error={encounterError} class="meta" role={encounterError ? "alert" : "status"}>
              {encounterError ?? (ready === null ? strings.sessions.loading : t.noReady)}
            </p>
          {:else if key === "locations" || key === "npcs"}
            {@const kind = key === "locations" ? "location" : "npc"}
            <WorldTree
              bind:this={trees[kind]}
              {kind}
              bind:world
              embedded
              selected={picked[kind]}
              onselect={(id) => (picked[kind] = id)}
              onedit={(id) => onedit(kind, id)}
              ondragentry={(id, name) => ondragstart({ kind, id }, name)}
              ondragentryend={ondragend}
            />
          {:else}
            <!-- As the Locations and NPCs trees: a search, and rows under it that scroll. -->
            <label for="encounter-search" class="sr-only">{t.searchEncounters}</label>
            <input id="encounter-search" class="text-field" type="search" autocomplete="off" placeholder={t.searchEncounters} bind:value={encounterQuery} />
            <div class="rows">
              {#if shownEncounters.length === 0}
                <p class="meta" role="status">{strings.sessions.world.noMatches}</p>
              {:else}
                <ul aria-label={t.encounters}>
                  {#each shownEncounters as card (card.item.id)}
                    <!-- Dragging is one way; the scene's dropdowns do the same from the keyboard. -->
                    <li
                      draggable="true"
                      ondragstart={(ev) => {
                        // Our own type, not text: a text field it passes over must not take it.
                        ev.dataTransfer?.setData("application/x-koldron", `${card.item.kind}:${card.item.id}`);
                        if (ev.dataTransfer) ev.dataTransfer.effectAllowed = "copy";
                        ondragstart(card.item, card.name);
                      }}
                      {ondragend}
                    >
                      <span class="row-text">
                        <span class="row-name">{card.name}</span>
                        {#if card.sub}<span class="meta">{card.sub}</span>{/if}
                      </span>
                      <a class="row-edit" href="{encountersHref}/{card.item.id}" aria-label={t.openEncounter(card.name)}>
                        <i class="ti ti-pencil" aria-hidden="true"></i>
                      </a>
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/each}
</aside>

<style>
  aside {
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .label {
    margin: 0;
    padding: 0 var(--space-1);
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .dropped {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    padding: 0 var(--space-1);
    color: var(--color-accent);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
  }

  .dropped:empty {
    display: none;
  }

  .section {
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .head {
    position: relative;
  }

  /* Over the toggle's right end, just before the arrow; the toggle's text leaves room for them. */
  .actions {
    position: absolute;
    top: 50%;
    right: calc(var(--space-3) + var(--icon-size) + var(--space-2));
    display: flex;
    gap: var(--space-1);
    transform: translateY(-50%);
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    min-height: var(--control-height-lg);
    padding: var(--space-2) var(--space-3);
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text);
    text-align: left;
    cursor: pointer;
  }

  .toggle:hover {
    background: var(--color-surface-sunken);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding-right: calc(var(--actions, 0) * (var(--control-height-sm) + var(--space-1)));
  }

  .count {
    margin-left: var(--space-1);
    color: var(--color-text-muted);
    font-size: var(--text-sm);
    font-weight: var(--weight-regular);
  }

  .name {
    font-weight: var(--weight-bold);
  }

  .meta {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .chevron {
    flex: none;
    font-size: var(--icon-size);
    transition: transform 0.15s;
  }

  .chevron.turned {
    transform: rotate(180deg);
  }

  :global([data-motion="reduce"]) .chevron {
    transition: none;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: 0 var(--space-3) var(--space-3);
  }

  /* Locations, NPCs and Encounters open to one height, less when the column has no room for it (it never goes below
     the fold); what's inside scrolls (a tree under its search). */
  .section.grows {
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .body.fixed {
    box-sizing: border-box;
    flex: 0 1 auto;
    min-height: 0;
    height: var(--session-list-height);
    overflow-y: auto;
  }

  .section:not(.grows) {
    flex: none;
  }

  textarea {
    resize: vertical;
    font: inherit;
  }

  /* The encounter rows, as the trees' rows: the list scrolls under the search. */
  .rows {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    margin-inline: calc(var(--space-1) - var(--space-3));
  }

  ul {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--control-height-sm);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    cursor: grab;
  }

  li:hover {
    background: var(--color-surface-sunken);
  }

  .row-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .row-name {
    font: var(--text-body-sm);
  }

  /* The pencil shows while the row is hovered or has focus, as in the trees. */
  .row-edit {
    display: grid;
    place-items: center;
    flex: none;
    width: var(--control-height-sm);
    height: var(--control-height-sm);
    color: var(--color-text-muted);
    opacity: 0;
  }

  li:hover .row-edit,
  li:focus-within .row-edit {
    opacity: 1;
  }

  .actions a,
  .row-edit {
    text-decoration: none;
  }

  .row-edit:hover {
    color: var(--color-text);
  }

  .small {
    align-self: flex-start;
    text-decoration: none;
  }

  .error {
    color: var(--color-danger);
  }
</style>
