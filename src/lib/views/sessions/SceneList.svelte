<script lang="ts">
  // The scenes in order: a numbered hexagon (filled once ready), the title, and a line with the location, the NPC
  // count and whether it has an encounter; Add scene at the bottom. A scene can be dragged to another place, or moved
  // with its grip and the up and down arrow keys. While a card is dragged, every scene is a drop target (outlined, the
  // one under the pointer highlighted).
  import { tick } from "svelte";
  import type { Session } from "$lib/api/sessions";
  import { strings } from "$lib/strings";

  let {
    session,
    selected,
    dragging,
    onselect,
    onadd,
    ondrop,
    onmove,
  }: {
    session: Session;
    /** The open scene, if a scene is open. */
    selected: string | null;
    /** Something is being dragged. */
    dragging: boolean;
    onselect: (id: string) => void;
    onadd: () => void;
    ondrop: (id: string) => void;
    /** Moves the scene at `from` to `to`. */
    onmove: (from: number, to: number) => void;
  } = $props();

  const t = strings.sessions.scenes;
  let over = $state<string | null>(null);
  $effect(() => {
    if (!dragging) over = null;
  });

  /** The scene being dragged to another place, and the place under the pointer. */
  let moving = $state<number | null>(null);
  let place = $state<number | null>(null);

  function startMove(ev: DragEvent, i: number, id: string) {
    moving = i;
    ev.dataTransfer?.setData("application/x-koldron-scene", id); // not text: a text field must not take it
    if (ev.dataTransfer) ev.dataTransfer.effectAllowed = "move";
  }

  async function onGripKey(ev: KeyboardEvent, i: number, id: string) {
    const to = ev.key === "ArrowUp" ? i - 1 : ev.key === "ArrowDown" ? i + 1 : null;
    if (to === null || to < 0 || to >= session.scenes.length) return;
    ev.preventDefault();
    onmove(i, to);
    await tick();
    document.querySelector<HTMLElement>(`[data-grip="${id}"]`)?.focus();
  }

  const locationName = (id: string | null) => session.locations.find((l) => l.id === id)?.name || (id ? strings.sessions.manage.unnamedLocation : t.noLocation);
</script>

<nav aria-label={t.title}>
  <h2 class="label">{t.title}</h2>
  <ol>
    {#each session.scenes as scene, i (scene.id)}
      <li
        class:selected={scene.id === selected}
        class:target={dragging}
        class:over={over === scene.id}
        class:moving={moving === i}
        class:place={place === i && moving !== null && moving !== i}
        ondragover={(ev) => {
          if (moving === null) return;
          ev.preventDefault();
          place = i;
        }}
        ondrop={(ev) => {
          if (moving === null) return;
          ev.preventDefault();
          onmove(moving, i);
          moving = place = null;
        }}
      >
        <button
          class="scene"
          aria-current={scene.id === selected}
          draggable="true"
          onclick={() => onselect(scene.id)}
          ondragstart={(ev) => startMove(ev, i, scene.id)}
          ondragend={() => (moving = place = null)}
          ondragover={(ev) => {
            if (!dragging) return;
            ev.preventDefault();
            over = scene.id;
          }}
          ondragleave={() => over === scene.id && (over = null)}
          ondrop={(ev) => {
            if (!dragging) return;
            ev.preventDefault();
            ondrop(scene.id);
          }}
        >
          <span class="hex" class:ready={scene.ready}>{i + 1}</span>
          <span class="text">
            <span class="title" class:untitled={!scene.title}>{scene.title || t.untitled}</span>
            <span class="meta">
              {[locationName(scene.locationId), scene.npcIds.length > 0 && t.npcCount(scene.npcIds.length), scene.encounterIds.length > 0 && t.encounter]
                .filter(Boolean)
                .join(" · ")}
            </span>
          </span>
          {#if scene.ready}<i class="ti ti-check ready-mark" role="img" aria-label={t.ready}></i>{/if}
        </button>
        <button
          class="icon-btn grip"
          data-grip={scene.id}
          draggable="true"
          aria-label={t.move(scene.title || t.untitled)}
          onkeydown={(ev) => onGripKey(ev, i, scene.id)}
          ondragstart={(ev) => startMove(ev, i, scene.id)}
          ondragend={() => (moving = place = null)}
        >
          <i class="ti ti-grip-vertical" aria-hidden="true"></i>
        </button>
      </li>
    {:else}
      <li class="empty">{t.empty}</li>
    {/each}
  </ol>
  <button class="add" onclick={onadd}>
    <i class="ti ti-plus" aria-hidden="true"></i>
    {t.add}
  </button>
</nav>

<style>
  nav,
  ol {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  ol {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* The row carries the background and outlines, so they run under the grip to the end. */
  li {
    display: flex;
    align-items: center;
    padding-right: var(--space-1);
    border: var(--border-width) solid transparent;
    border-radius: var(--radius-md);
  }

  li:not(.empty):hover {
    background: var(--color-surface-sunken);
  }

  li.selected {
    border-color: var(--potion-glow);
    background: var(--potion-glow);
    color: var(--charcoal);
  }

  li.target {
    outline: var(--border-width) dashed var(--color-accent);
    outline-offset: calc(-1 * var(--border-width));
  }

  li.over {
    outline: var(--focus-ring) solid var(--color-text);
    outline-offset: calc(-1 * var(--focus-ring));
    background: var(--potion-glow);
    color: var(--charcoal);
  }

  li.selected .grip,
  li.over .grip {
    color: inherit;
  }

  li.moving {
    opacity: 0.5;
  }

  /* Where a dragged scene will land. */
  li.place {
    outline: var(--focus-ring) solid var(--color-accent);
  }

  /* The grip shows on the row's hover or focus. */
  .grip {
    opacity: 0;
    cursor: grab;
  }

  li:hover .grip,
  li:focus-within .grip {
    opacity: 1;
  }

  .label {
    margin: 0;
    padding: 0 var(--space-1) var(--space-1);
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .scene {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: var(--control-height-lg);
    padding: var(--space-1) var(--space-3);
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .hex {
    display: grid;
    place-items: center;
    flex: none;
    width: var(--roster-badge-width);
    height: var(--roster-badge-height);
    background: var(--color-border-strong);
    color: var(--color-text);
    font: var(--text-dice);
    font-weight: var(--weight-bold);
    clip-path: polygon(25% 0, 75% 0, 100% 50%, 75% 100%, 25% 100%, 0 50%);
  }

  .hex.ready {
    background: var(--charcoal);
    color: var(--potion-glow);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: var(--weight-bold);
  }

  .untitled {
    font-weight: var(--weight-regular);
  }

  .meta {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .selected .meta,
  .over .meta {
    color: inherit;
  }

  .ready-mark {
    font-size: var(--icon-size);
  }

  .empty {
    padding: var(--space-3);
    color: var(--color-text-muted);
  }

  .add {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    min-height: var(--control-height-lg);
    margin-top: var(--space-1);
    padding: 0 var(--space-3);
    border: var(--border-width) dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text);
    font-weight: var(--weight-bold);
    cursor: pointer;
  }

  .add:hover {
    background: var(--color-surface-sunken);
  }
</style>
