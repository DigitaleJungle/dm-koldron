<script lang="ts">
  import type { Snippet } from "svelte";

  // The Library's left column (Rules' contents, the Catalog's sources): its items scroll, its foot (the gear) stays.
  // Hidden, it eases to no width and then can't be reached; shown, it eases back.
  let { id, collapsed, children, bottom }: { id: string; collapsed: boolean; children: Snippet; bottom?: Snippet } = $props();
</script>

<aside {id} class:collapsed>
  <!-- The column's items (a layout container only, as in the campaign sidebar). -->
  <div class="items">{@render children()}</div>
  {#if bottom}<div class="bottom">{@render bottom()}</div>{/if}
</aside>

<style>
  /* Hiding and showing ease its width; hidden, it turns invisible once closed (so the keyboard can't reach it) and
     visible at once when it opens. */
  aside {
    flex: 0 0 var(--library-contents-width);
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-right: var(--border-width) solid var(--color-border);
    background: var(--color-surface);
    transition:
      flex-basis var(--duration-drawer) ease-in-out,
      visibility 0s;
  }

  aside.collapsed {
    flex-basis: 0;
    border-right-width: 0;
    visibility: hidden;
    transition:
      flex-basis var(--duration-drawer) ease-in-out,
      visibility 0s var(--duration-drawer);
  }

  :global([data-motion="reduce"]) aside {
    transition: none;
  }

  /* Their full width while the column eases, so the items don't rewrap as it narrows. */
  .items,
  .bottom {
    flex: none;
    width: var(--library-contents-width);
    padding: 0 var(--space-3);
  }

  .items {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-top: var(--space-3);
    padding-bottom: var(--space-3);
  }

  .bottom {
    padding-top: var(--space-2);
    padding-bottom: var(--space-2);
  }

  /* Narrow windows: above the pane, in a box of its own (the parent stacks them). */
  @media (max-width: 720px) {
    aside {
      flex: 0 0 var(--library-stacked-height);
      padding-bottom: var(--space-3);
      border-right: none;
      border-bottom: var(--border-width) solid var(--color-border);
    }
  }
</style>
