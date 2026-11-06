<script lang="ts">
  // The party as a dropdown: a button with how many characters are ticked, opening a list of checkboxes (name and
  // level). Escape or a click outside closes it (the browser's popover does that).
  import type { Character } from "$lib/api/characters";
  import { strings } from "$lib/strings";
  import { characterLevel } from "./roster";

  let {
    characters,
    selected,
    error,
    ontoggle,
    disabled = false,
  }: {
    /** Null while loading. */
    characters: Character[] | null;
    selected: Set<string>;
    error: string | null;
    ontoggle: (id: string, on: boolean) => void;
    /** A Ready encounter's party can't change. */
    disabled?: boolean;
  } = $props();

  const t = strings.encounters.party;
  const id = $props.id();
  let button: HTMLButtonElement | undefined = $state();
  let list: HTMLElement | undefined = $state();
  const count = $derived((characters ?? []).filter((c) => selected.has(c.id)).length);
  const label = $derived(error ? t.title : characters === null ? t.loading : characters.length === 0 ? t.none : t.chosen(count));

  // Below the button, as wide as it at least, kept inside the window.
  function place() {
    if (!button || !list) return;
    const at = button.getBoundingClientRect();
    list.style.minWidth = `${at.width}px`;
    list.style.top = `${Math.max(8, Math.min(at.bottom + 4, innerHeight - list.offsetHeight - 8))}px`;
    list.style.left = `${Math.max(8, Math.min(at.left, innerWidth - list.offsetWidth - 8))}px`;
  }
</script>

<div class="picker">
  <button
    bind:this={button}
    class="trigger"
    popovertarget="{id}-list"
    aria-label={`${t.title}, ${label}`}
    disabled={disabled || !characters?.length}
  >
    <span>{label}</span>
    <i class="ti ti-chevron-down" aria-hidden="true"></i>
  </button>
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div bind:this={list} id="{id}-list" popover="auto" class="list" role="group" aria-label={t.title} ontoggle={(e) => e.newState === "open" && place()}>
    <ul>
      {#each characters ?? [] as c (c.id)}
        {@const level = characterLevel(c.classes)}
        <li>
          <label>
            <input type="checkbox" checked={selected.has(c.id)} onchange={(e) => ontoggle(c.id, e.currentTarget.checked)} />
            <span class="name">{c.name}</span>
            <span class={level === null ? "warn" : "muted"}>{level === null ? t.noLevel : t.level(level)}</span>
          </label>
        </li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    width: 100%;
    min-height: var(--control-height);
    padding: 0 var(--space-2) 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: var(--color-text);
    text-align: left;
    cursor: pointer;
  }

  .trigger span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .trigger:disabled {
    color: var(--color-text-muted);
    cursor: default;
  }

  .list {
    position: fixed;
    inset: auto;
    margin: 0;
    padding: var(--space-1);
    max-height: min(60vh, calc(100vh - 2 * var(--space-4)));
    overflow-y: auto;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: var(--shadow-drawer);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    white-space: nowrap;
    cursor: pointer;
  }

  label:hover {
    background: var(--color-surface-sunken);
  }

  .name {
    flex: 1;
    font-weight: var(--weight-medium);
  }

  .muted {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .warn,
  .error {
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  .error {
    margin: 0;
  }
</style>
