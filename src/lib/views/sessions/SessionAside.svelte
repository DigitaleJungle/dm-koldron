<script lang="ts">
  // Across session: Previously, Premise, Summary, Party note, Locations, NPCs, Encounters and Checklist, one open at a time (opening one
  // closes the other; the open one closes again). Location, NPC and encounter cards (the campaign's encounters that are
  // ready to run) can be dragged onto a scene; locations and NPCs are the campaign's, in tree order. Locations, NPCs and the checklist open their full view in the editor;
  // Manage encounters opens the Encounters screen.
  import type { Encounter } from "$lib/api/encounters";
  import type { Session } from "$lib/api/sessions";
  import type { World } from "$lib/api/world";
  import { strings } from "$lib/strings";
  import Checklist from "./Checklist.svelte";
  import { linkedIn, NPC_DRAG, npcSpecs, scenesWith, toggleSection, treeOrder, type Linkable, type Pane, type Section } from "./sessions";

  let {
    session = $bindable(),
    world,
    open = $bindable(),
    lastDrop,
    ready,
    encounterError,
    encountersHref,
    encounterMeta,
    onmanage,
    ondragstart,
    ondragend,
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
    ondragstart: (item: Linkable, name: string) => void;
    ondragend: () => void;
  } = $props();

  const t = strings.sessions.sections;
  const m = strings.sessions.manage;
  const written = (text: string) => (text.trim() ? t.written : t.notWritten);

  /** `depth`: how far under the top of its tree. */
  type Card = { item: Linkable; name: string; sub: string; depth: number };
  const cards = $derived<Record<"locations" | "npcs" | "encounters", Card[]>>({
    locations: treeOrder(world.locations).map(({ entry: l, depth }) => ({
      item: { kind: "location", id: l.id },
      name: l.name || m.unnamedLocation,
      sub: t.inScenes(scenesWith(session, { kind: "location", id: l.id }).length),
      depth,
    })),
    npcs: treeOrder(world.npcs).map(({ entry: n, depth }) => ({
      item: { kind: "npc", id: n.id },
      name: n.name || m.unnamedNpc,
      sub: npcSpecs(n) || t.noSpecs,
      depth,
    })),
    encounters: (ready ?? []).map((x) => ({ item: { kind: "encounter", id: x.id }, name: x.name, sub: encounterMeta(x.id) ?? "", depth: 0 })),
  });

  const done = $derived(session.checklist.filter((c) => c.done).length);
  const metas = $derived<Record<Section, string>>({
    previously: written(session.previously),
    premise: written(session.premise),
    summary: written(session.summary),
    partyNote: written(session.partyNote),
    locations: t.inSession(linkedIn(session, "location").filter((id) => world.locations.some((l) => l.id === id)).length),
    npcs: t.inSession(linkedIn(session, "npc").filter((id) => world.npcs.some((n) => n.id === id)).length),
    encounters: ready === null ? "" : t.readyToRun(ready.length),
    checklist: t.doneOf(done, session.checklist.length),
  });
  const SECTIONS: Section[] = ["previously", "premise", "summary", "partyNote", "locations", "npcs", "encounters", "checklist"];
</script>

<aside aria-labelledby="across-session">
  <h2 id="across-session" class="label">{t.title}</h2>
  <p class="dropped" role="status">
    {#if lastDrop}<i class="ti ti-check" aria-hidden="true"></i>{lastDrop}{/if}
  </p>

  {#each SECTIONS as key (key)}
    <div class="section">
      <button class="toggle" aria-expanded={open === key} aria-controls="section-{key}" onclick={() => (open = toggleSection(open, key))}>
        <span class="text">
          <span class="name">{t[key]}</span>
          <span class="meta">{metas[key]}</span>
        </span>
        <i class="ti ti-chevron-down chevron" class:turned={open === key} aria-hidden="true"></i>
      </button>
      {#if open === key}
        <div id="section-{key}" class="body">
          {#if key === "previously" || key === "premise" || key === "summary" || key === "partyNote"}
            <label for="text-{key}" class="sr-only">{t[key]}</label>
            <textarea id="text-{key}" class="text-field" rows="6" maxlength="20000" bind:value={session[key]}></textarea>
          {:else if key === "checklist"}
            <Checklist bind:items={session.checklist} placeholder={strings.sessions.checklist.short} compact />
            <button class="btn small" onclick={() => onmanage("checklist")}>{t.manage.checklist}</button>
          {:else if key === "encounters" && (encounterError || ready === null || ready.length === 0)}
            <p class:error={encounterError} class="meta" role={encounterError ? "alert" : "status"}>
              {encounterError ?? (ready === null ? strings.sessions.loading : t.noReady)}
            </p>
            <a class="btn small" href={encountersHref}>{t.manage.encounters}</a>
          {:else}
            {#each cards[key] as card (card.item.id)}
              <!-- Dragging is one way; the scene's dropdowns do the same from the keyboard. -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="card"
                style:--depth={card.depth}
                draggable="true"
                ondragstart={(ev) => {
                  // Our own type, not text: a text field it passes over must not take it.
                  ev.dataTransfer?.setData("application/x-koldron", `${card.item.kind}:${card.item.id}`);
                  if (card.item.kind === "npc") ev.dataTransfer?.setData(NPC_DRAG, card.item.id); // a location editor takes it too
                  if (ev.dataTransfer) ev.dataTransfer.effectAllowed = "copy";
                  ondragstart(card.item, card.name);
                }}
                {ondragend}
              >
                <i class="ti ti-grip-vertical grip" aria-hidden="true"></i>
                <span class="text">
                  <strong>{card.name}</strong>
                  {#if card.sub}<span class="meta">{card.sub}</span>{/if}
                </span>
              </div>
            {/each}
            {#if key === "encounters"}
              <a class="btn small" href={encountersHref}>{t.manage.encounters}</a>
            {:else}
              <button class="btn small" onclick={() => onmanage(key)}>{t.manage[key]}</button>
            {/if}
          {/if}
        </div>
      {/if}
    </div>
  {/each}
</aside>

<style>
  aside {
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

  textarea {
    resize: vertical;
    font: inherit;
  }

  /* Under its parent, a step in per level. */
  .card {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-left: calc(var(--depth, 0) * var(--space-4));
    padding: var(--space-2);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    cursor: grab;
  }

  .grip {
    color: var(--color-text-muted);
    font-size: var(--icon-size-sm);
  }

  .small {
    align-self: flex-start;
    text-decoration: none;
  }

  .error {
    color: var(--color-danger);
  }
</style>
