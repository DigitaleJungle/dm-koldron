<script lang="ts">
  // Tabs stacked down the left of a panel, each an icon in a hexagon (flat top and bottom, its left corner cut off so it
  // sits flush with the panel's left edge; the open one filled green), the open part beside them. Up and down move
  // between them (round), Home and End to the first or last. An optional action (a red trash can) sits at the bottom of
  // the column; while it asks first (`confirm`), a cross takes its spot (cancel) with a red check above it (do it). Used by a scene and a location in the session prep. Fills the rest of its panel, which must have the
  // panel padding (--space-5) the column reaches out over.
  import type { Snippet } from "svelte";

  type Tab = { id: string; icon: string; label: string };

  let {
    tabs,
    current = $bindable(),
    id,
    label,
    action,
    children,
  }: {
    tabs: readonly Tab[];
    current: string;
    /** Prefix for the tabs' and panel's ids. */
    id: string;
    /** The tab list's name, for screen readers. */
    label: string;
    action?: {
      icon: string;
      label: string;
      onclick: () => void;
      /** Set while asking first. */
      confirm?: { label: string; cancel: string; onconfirm: () => void; oncancel: () => void };
    };
    children: Snippet;
  } = $props();

  function onkey(ev: KeyboardEvent, i: number) {
    const n = tabs.length;
    const to = { ArrowUp: (i + n - 1) % n, ArrowDown: (i + 1) % n, Home: 0, End: n - 1 }[ev.key];
    if (to === undefined) return;
    ev.preventDefault();
    current = tabs[to].id;
    document.getElementById(`${id}-tab-${current}`)?.focus();
  }
</script>

<div class="tabbed">
  <div class="side">
    <div class="tabs" role="tablist" aria-orientation="vertical" aria-label={label}>
      {#each tabs as tab, i (tab.id)}
        <button
          id="{id}-tab-{tab.id}"
          class="tab"
          role="tab"
          aria-selected={current === tab.id}
          aria-controls="{id}-panel"
          aria-label={tab.label}
          tabindex={current === tab.id ? 0 : -1}
          onclick={() => (current = tab.id)}
          onkeydown={(ev) => onkey(ev, i)}
        >
          <span class="hex"><i class="ti {tab.icon}" aria-hidden="true"></i></span>
        </button>
      {/each}
    </div>
    {#if action?.confirm}
      {@const c = action.confirm}
      <!-- svelte-ignore a11y_no_static_element_interactions: Escape cancels, as the cross does -->
      <div class="confirm" onkeydown={(ev) => ev.key === "Escape" && (ev.stopPropagation(), c.oncancel())}>
        <button class="tab action" aria-label={c.label} onclick={c.onconfirm}>
          <span class="hex"><i class="ti ti-check" aria-hidden="true"></i></span>
        </button>
        <button id="{id}-action-cancel" class="tab" aria-label={c.cancel} onclick={c.oncancel}>
          <span class="hex"><i class="ti ti-x" aria-hidden="true"></i></span>
        </button>
      </div>
    {:else if action}
      <button id="{id}-action" class="tab action" aria-label={action.label} onclick={action.onclick}>
        <span class="hex"><i class="ti {action.icon}" aria-hidden="true"></i></span>
      </button>
    {/if}
  </div>
  <div class="panel" id="{id}-panel" role="tabpanel" aria-labelledby="{id}-tab-{current}">
    {@render children()}
  </div>
</div>

<style>
  /* The tabs down the left, the open part beside them, filling the rest of the panel. */
  .tabbed {
    flex: 1;
    display: flex;
    gap: var(--space-4);
  }

  /* The tabs, and the action at the bottom. Flush with the panel's left edge: out over its padding. */
  .side {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: var(--space-2);
    margin-left: calc(-1 * var(--space-5));
  }

  .tabs,
  .confirm {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .action .hex,
  .tab.action:hover .hex {
    color: var(--color-danger);
  }

  /* The button keeps the focus ring; the hexagon inside is clipped: flat top and bottom, its left corner cut off. */
  .tab {
    display: block;
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
  }

  .tab .hex {
    display: grid;
    place-items: center;
    width: var(--control-height-lg);
    height: var(--control-height-lg);
    background: var(--color-surface-sunken);
    color: var(--color-text-muted);
    font-size: var(--icon-size);
    clip-path: polygon(0 0, 75% 0, 100% 50%, 75% 100%, 0 100%);
  }

  /* Inside the button: at the panel's edge, a ring outside it would be cut off by the scrolling column. */
  .tab:focus-visible {
    outline-offset: calc(-1 * var(--focus-ring));
  }

  .tab:hover .hex {
    color: var(--color-text);
  }

  .tab[aria-selected="true"] .hex {
    background: var(--color-accent);
    color: var(--on-fill);
  }

  .panel {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
</style>
