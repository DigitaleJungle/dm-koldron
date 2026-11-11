<script lang="ts">
  // A checklist: tick, remove (not in the compact one) and add items. Used for a scene, for the session, and compact
  // in the right column.
  import type { Check } from "$lib/api/sessions";
  import { strings } from "$lib/strings";

  let {
    items = $bindable(),
    placeholder,
    compact = false,
    locked = false,
  }: { items: Check[]; placeholder: string; compact?: boolean; locked?: boolean } = $props();

  const t = strings.sessions.checklist;
  let draft = $state("");

  function add(ev: SubmitEvent) {
    ev.preventDefault();
    const text = draft.trim();
    if (!text) return;
    items.push({ text, done: false });
    draft = "";
  }
</script>

{#if items.length > 0}
  <ul class="checks" class:compact>
    {#each items as item, i (i)}
      <li class:done={item.done}>
        <button class="tick" aria-pressed={item.done} aria-label={t.tick(item.text, item.done)} onclick={() => (item.done = !item.done)}>
          {#if item.done}<i class="ti ti-check" aria-hidden="true"></i>{/if}
        </button>
        <span class="text">{item.text}</span>
        {#if !compact && !locked}
          <button class="icon-btn" aria-label={t.remove(item.text)} onclick={() => items.splice(i, 1)}>
            <i class="ti ti-x" aria-hidden="true"></i>
          </button>
        {/if}
      </li>
    {/each}
  </ul>
{/if}
{#if !locked}
  <form class="add" class:compact onsubmit={add}>
    <input class="text-field" bind:value={draft} maxlength="200" autocomplete="off" {placeholder} aria-label={t.newItem} />
    <button class="btn" type="submit" disabled={!draft.trim()}>{t.add}</button>
  </form>
{/if}

<style>
  .checks {
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
  }

  /* The box is small, its button a full touch target. */
  .tick {
    display: grid;
    place-items: center;
    flex: none;
    width: var(--control-height);
    height: var(--control-height);
    padding: 0;
    border: 0;
    background: none;
    color: var(--on-fill);
    cursor: pointer;
  }

  .tick::before {
    content: "";
    grid-area: 1 / 1;
    width: var(--space-6);
    height: var(--space-6);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
  }

  .tick[aria-pressed="true"]::before {
    border-color: var(--color-accent);
    background: var(--color-accent);
  }

  .tick .ti {
    grid-area: 1 / 1;
    z-index: 1;
    font-size: var(--icon-size-sm);
  }

  .text {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .done .text {
    color: var(--color-text-muted);
    text-decoration: line-through;
  }

  .compact .text {
    font-size: var(--text-sm);
  }

  .add {
    display: flex;
    gap: var(--space-2);
    padding-top: var(--space-2);
  }

  .add input {
    flex: 1;
    min-width: 0;
  }
</style>
