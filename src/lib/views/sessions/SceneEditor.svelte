<script lang="ts">
  // The open scene: its title and Ready (an icon); what happens; its links: the location, NPCs and encounters as cards
  // (a dropdown each to pick or link, so dragging is never the only way; a pencil opens a location or NPC in its editor,
  // a cross unlinks); its checklist; and Remove scene at the bottom. A new location opens its editor to name it.
  import { tick, untrack } from "svelte";
  import type { Session } from "$lib/api/sessions";
  import type { World } from "$lib/api/world";
  import { strings } from "$lib/strings";
  import RichText from "$lib/components/RichText.svelte";
  import SideTabs from "$lib/components/SideTabs.svelte";
  import TagPreview from "$lib/components/TagPreview.svelte";
  import { cleanHtml, isEmpty } from "$lib/components/richtext";
  import { libraryHome } from "$lib/stores/campaigns.svelte";
  import EncounterCard from "$lib/views/encounters/EncounterCard.svelte";
  import type { EncounterCardData } from "$lib/views/encounters/card";
  import Checklist from "./Checklist.svelte";
  import LocationCard from "./LocationCard.svelte";
  import NpcCard from "./NpcCard.svelte";
  import { ancestors, link, newLocation, treeOrder, unlink } from "./sessions";

  let {
    session = $bindable(),
    world = $bindable(),
    index,
    encounterName,
    encounterCard,
    encountersHref,
    encounterChoices,
    onedit,
    onremove,
    current = $bindable(),
  }: {
    session: Session;
    /** The campaign's locations and NPCs. */
    world: World;
    /** The open scene's place in the session. */
    index: number;
    /** An encounter's name (or that it's loading or gone), and its card once known, by id. */
    encounterName: (id: string) => string;
    encounterCard: (id: string) => EncounterCardData | undefined;
    encountersHref: string;
    /** The campaign's encounters that are ready to run. */
    encounterChoices: { id: string; name: string }[];
    /** Open a location or NPC in its editor (a new location to name it). */
    onedit: (kind: "location" | "npc", id: string) => void;
    onremove: () => void;
    /** The open tab; kept by the page, so it's still open after a location or NPC editor took the scene's place. */
    current: string;
  } = $props();

  const t = strings.sessions.scenes;
  const m = strings.sessions.manage;
  const scene = $derived(session.scenes[index]);
  const location = $derived(world.locations.find((l) => l.id === scene.locationId) ?? null);
  const linkedNpcs = $derived(scene.npcIds.flatMap((id) => world.npcs.find((n) => n.id === id) ?? []));
  /** Indented under their parents, as the trees show them. */
  const indent = (depth: number, name: string) => "  ".repeat(depth) + name;
  const locationChoices = $derived(treeOrder(world.locations));
  const npcChoices = $derived(treeOrder(world.npcs).filter((r) => !scene.npcIds.includes(r.entry.id)));

  let titleInput: HTMLInputElement | undefined = $state();
  // A new scene has no title yet: start typing it.
  $effect(() => {
    void scene.id;
    if (!untrack(() => scene.title)) titleInput?.focus();
  });

  // The scene's parts, a tab each (kept when another scene opens).
  const TABS = [
    { id: "text", icon: "ti-message-2-check", label: t.whatHappens },
    { id: "links", icon: "ti-link", label: t.links },
    { id: "checklist", icon: "ti-list", label: t.checklist },
  ] as const;

  let confirming = $state(false);
  /** Asks first (a check over a cross in the trash's place), with the focus on the cross. */
  async function askRemove() {
    confirming = true;
    await tick();
    document.getElementById("scene-action-cancel")?.focus();
  }

  async function keepScene() {
    confirming = false;
    await tick();
    document.getElementById("scene-action")?.focus();
  }
  $effect(() => {
    void scene.id;
    confirming = false;
  });

  function pickLocation(ev: Event & { currentTarget: HTMLSelectElement }) {
    const value = ev.currentTarget.value;
    if (value === "__new") {
      const created = newLocation(t.newLocationName);
      world.locations.push(created);
      scene.locationId = created.id;
      onedit("location", created.id);
    } else scene.locationId = value || null;
  }

  function pickLink(ev: Event & { currentTarget: HTMLSelectElement }, ids: string[]) {
    const value = ev.currentTarget.value;
    if (value) link(ids, value);
    ev.currentTarget.value = "";
  }
</script>

<div class="head">
  {#if scene.ready}
    <h2 id="scene-title" class:untitled={!scene.title}>{scene.title || t.untitled}</h2>
  {:else}
    <input id="scene-title" bind:this={titleInput} bind:value={scene.title} maxlength="100" autocomplete="off" placeholder={t.untitled} aria-label={t.titleField} />
  {/if}
  <button class="btn icon ready" aria-pressed={scene.ready} aria-label={t.ready} onclick={() => (scene.ready = !scene.ready)}>
    <i class="ti ti-check" aria-hidden="true"></i>
  </button>
</div>

<!-- The trash at the bottom of the tab column asks first, in its place. -->
<SideTabs
  tabs={TABS}
  bind:current
  id="scene"
  label={t.parts}
  action={{
    icon: "ti-trash",
    label: t.remove,
    onclick: askRemove,
    confirm: confirming ? { label: t.remove, cancel: strings.sessions.cancel, onconfirm: onremove, oncancel: keepScene } : undefined,
  }}
>
{#if current === "text"}
<!-- Ready: the text shown, as locked encounter notes are. Otherwise the field, keyed: it fills itself once, so another
     scene needs a fresh one. Its name is for screen readers only. -->
{#if scene.ready}
  <section class="what-happens" aria-label={t.whatHappens}>
    {#if isEmpty(scene.summary)}
      <p class="note">{strings.sessions.sections.notWritten}</p>
    {:else}
      <TagPreview>
        <div class="prose notes">{@html cleanHtml(scene.summary, { links: true, base: libraryHome() })}</div>
      </TagPreview>
    {/if}
  </section>
{:else}
  {#key scene.id}
    <RichText label={t.whatHappens} extended bind:value={scene.summary} />
  {/key}
{/if}
{:else if current === "links"}

<!-- A group each: its name and dropdown, then its cards. -->
<div class="links">
  <section class="group" aria-labelledby="links-location">
    <div class="group-head">
      <h3 id="links-location" class="label"><i class="ti ti-map-pin" aria-hidden="true"></i>{t.location}</h3>
      <select class="choose" value={scene.locationId ?? ""} onchange={pickLocation} aria-label={t.location}>
        <option value="">{t.noLocation}</option>
        {#each locationChoices as r (r.entry.id)}
          <option value={r.entry.id}>{indent(r.depth, r.entry.name || m.unnamedLocation)}</option>
        {/each}
        <option value="__new">{t.newLocation}</option>
      </select>
    </div>
    {#if location}
      <div class="cards">
        <LocationCard
          {location}
          path={ancestors(world.locations, location.id).map((a) => a.name || m.unnamedLocation)}
          onedit={() => onedit("location", location.id)}
          onremove={() => (scene.locationId = null)}
        />
      </div>
    {/if}
  </section>

  <section class="group" aria-labelledby="links-npcs">
    <div class="group-head">
      <h3 id="links-npcs" class="label"><i class="ti ti-user" aria-hidden="true"></i>{t.npcs}</h3>
      <select class="chip-pick" value="" onchange={(ev) => pickLink(ev, scene.npcIds)} aria-label={t.npcs}>
        <option value="">{t.link}</option>
        {#each npcChoices as r (r.entry.id)}
          <option value={r.entry.id}>{indent(r.depth, r.entry.name || m.unnamedNpc)}</option>
        {/each}
      </select>
    </div>
    {#if linkedNpcs.length}
      <div class="cards">
        {#each linkedNpcs as npc (npc.id)}
          <NpcCard {npc} onedit={() => onedit("npc", npc.id)} onremove={() => unlink(scene.npcIds, npc.id)} />
        {/each}
      </div>
    {/if}
  </section>

  <section class="group" aria-labelledby="links-encounters">
    <div class="group-head">
      <h3 id="links-encounters" class="label"><i class="ti ti-sword" aria-hidden="true"></i>{t.encounters}</h3>
      <select class="chip-pick" value="" onchange={(ev) => pickLink(ev, scene.encounterIds)} aria-label={t.encounters}>
        <option value="">{t.link}</option>
        {#each encounterChoices.filter((x) => !scene.encounterIds.includes(x.id)) as x (x.id)}
          <option value={x.id}>{x.name}</option>
        {/each}
      </select>
    </div>
    {#if scene.encounterIds.length}
      <div class="cards">
        {#each scene.encounterIds as id (id)}
          {@const card = encounterCard(id)}
          {#if card}
            <EncounterCard {card} base={encountersHref} compact onremove={() => unlink(scene.encounterIds, id)} removeLabel={t.unlink(card.name)} />
          {:else}
            <!-- Still loading, or removed from the campaign: its name or that it's gone, and the cross. -->
            <article class="link-card missing">
              <div class="title">
                <h4 class="muted">{encounterName(id)}</h4>
                <button class="icon-btn" aria-label={t.unlink(encounterName(id))} onclick={() => unlink(scene.encounterIds, id)}>
                  <i class="ti ti-x" aria-hidden="true"></i>
                </button>
              </div>
            </article>
          {/if}
        {/each}
      </div>
    {/if}
  </section>
</div>
{:else}
<section class="block" aria-labelledby="scene-checklist">
  <div class="row">
    <h3 id="scene-checklist" class="label"><i class="ti ti-list-check" aria-hidden="true"></i>{t.checklist}</h3>
    <span class="mono">{scene.checklist.filter((c) => c.done).length}/{scene.checklist.length}</span>
  </div>
  <Checklist bind:items={scene.checklist} placeholder={strings.sessions.checklist.short} locked={scene.ready} />
</section>
{/if}
</SideTabs>


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

  /* A Ready scene's title is text, not a field: same place and size, no underline. */
  h2#scene-title {
    display: flex;
    align-items: center;
    margin: 0;
    border-bottom-color: transparent;
    overflow-wrap: anywhere;
  }

  h2#scene-title.untitled {
    color: var(--color-text-muted);
  }

  /* Without .prose's reading width and centring, as in the encounter notes. */
  /* A table wider than the panel scrolls sideways here, not the whole column. */
  .what-happens .prose {
    max-width: none;
    margin: 0;
    overflow-wrap: anywhere;
    overflow-x: auto;
  }

  .what-happens .prose > :global(:first-child) {
    margin-top: 0;
  }

  .what-happens .prose > :global(:last-child) {
    margin-bottom: 0;
  }

  /* Boxed text as in the editor (RichText.svelte): on the page colour, so it stands out from the panel. */
  .what-happens :global(blockquote) {
    margin: var(--space-3) 0;
    padding: var(--space-3) var(--space-4);
    border: 0;
    border-left: var(--border-width-thick) solid var(--color-border-strong);
    border-radius: 0;
    background: var(--color-bg);
  }

  /* Tables: no borders; a header row (only header cells) shaded, then banded rows starting plain. */
  .what-happens :global(th),
  .what-happens :global(td) {
    border: 0;
    background: none;
  }

  .what-happens :global(tr:not(:has(> td)) > th) {
    background: var(--color-border);
  }

  .what-happens :global(tr:nth-child(even of :has(> td)) > *) {
    background: var(--color-surface-sunken);
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

  .label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
  }

  /* The scene's links: a group each (its name and dropdown on one line, then its cards), divided by lines. */
  /* The encounter card's collage, smaller here than on the Encounters screen. */
  .links {
    --encounter-art-height: var(--link-card-art-height);
    --encounter-hex-width: var(--link-card-hex-width);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .group + .group {
    padding-top: var(--space-4);
    border-top: var(--border-width) solid var(--color-border);
  }

  .group-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--link-card-min-width), 1fr));
    gap: var(--space-3);
  }

  /* What the location, NPC and missing-encounter cards share (the encounter card has its own). */
  .links :global(.link-card) {
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .links :global(.link-card .title) {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .links :global(.link-card h4) {
    flex: 1;
    min-width: 0;
    margin: 0;
    font: var(--text-body);
    font-weight: var(--weight-bold);
    overflow-wrap: anywhere;
  }

  .links :global(.link-card .muted) {
    margin: 0;
    color: var(--color-text-muted);
    font: var(--text-body-sm);
  }

  .missing {
    padding: var(--space-3);
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

  .mono {
    color: var(--color-text-muted);
    font: var(--text-dice);
  }
</style>
