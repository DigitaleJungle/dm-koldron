<script lang="ts">
  // The scenes in order: a numbered hexagon (filled once ready), the title, and a line with the location, the NPC
  // count and whether it has an encounter; Add scene at the bottom. While something is dragged, every scene is a drop
  // target (outlined, the one under the pointer highlighted).
  import type { Session } from "$lib/api/sessions";
  import { strings } from "$lib/strings";

  let {
    session,
    selected,
    dragging,
    onselect,
    onadd,
    ondrop,
  }: {
    session: Session;
    /** The open scene, if a scene is open. */
    selected: string | null;
    /** Something is being dragged. */
    dragging: boolean;
    onselect: (id: string) => void;
    onadd: () => void;
    ondrop: (id: string) => void;
  } = $props();

  const t = strings.sessions.scenes;
  let over = $state<string | null>(null);
  $effect(() => {
    if (!dragging) over = null;
  });

  const locationName = (id: string | null) => session.locations.find((l) => l.id === id)?.name || (id ? strings.sessions.manage.unnamedLocation : t.noLocation);
</script>

<nav aria-label={t.title}>
  <h2 class="label">{t.title}</h2>
  <ol>
    {#each session.scenes as scene, i (scene.id)}
      <li>
        <button
          class="scene"
          class:selected={scene.id === selected}
          class:target={dragging}
          class:over={over === scene.id}
          aria-current={scene.id === selected}
          onclick={() => onselect(scene.id)}
          ondragover={(ev) => {
            if (!dragging) return;
            ev.preventDefault();
            over = scene.id;
          }}
          ondragleave={() => over === scene.id && (over = null)}
          ondrop={(ev) => {
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

  .label {
    margin: 0;
    padding: 0 var(--space-1) var(--space-1);
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .scene {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    min-height: var(--control-height-lg);
    padding: var(--space-1) var(--space-3);
    border: var(--border-width) solid transparent;
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text);
    text-align: left;
    cursor: pointer;
  }

  .scene:hover {
    background: var(--color-surface-sunken);
  }

  .scene.selected {
    border-color: var(--potion-glow);
    background: var(--potion-glow);
    color: var(--charcoal);
  }

  .scene.target {
    outline: var(--border-width) dashed var(--color-accent);
    outline-offset: calc(-1 * var(--border-width));
  }

  .scene.over {
    outline: var(--focus-ring) solid var(--color-text);
    outline-offset: calc(-1 * var(--focus-ring));
    background: var(--potion-glow);
    color: var(--charcoal);
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
