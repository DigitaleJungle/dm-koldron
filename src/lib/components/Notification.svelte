<script lang="ts">
  import { fly } from "svelte/transition";
  import { dismiss, notification } from "$lib/stores/notifications.svelte";
  import { reduceMotion } from "$lib/stores/theme.svelte";
  import { strings } from "$lib/strings";

  // The single notification popup in the bottom-right corner. Clicking it closes it.
  // Only an entrance animation: a replaced one must vanish at once, so there's never two on screen.
  const motion = () => ({ y: 16, duration: reduceMotion() ? 0 : 200 });

  // A popover sits in the browser's top layer, so it shows over an open Settings dialog too.
  const topLayer = (node: HTMLElement) => node.showPopover();
</script>

<!-- The live region always exists, so screen readers hear each new message. -->
<div class="region" role="status" aria-live="polite">
  {#if notification.current}
    {#key notification.current.id}
      <button class="popup" popover="manual" use:topLayer title={strings.notification.dismiss} in:fly={motion()} onclick={dismiss}>
        <span class="message">{notification.current.message}</span>
        <i class="ti ti-x" aria-hidden="true"></i>
        <span class="sr-only">{strings.notification.dismiss}</span>
      </button>
    {/key}
  {/if}
</div>

<style>
  .popup {
    position: fixed;
    inset: auto var(--space-4) var(--space-4) auto;
    margin: 0;
    width: min(var(--notification-width), calc(100vw - 2 * var(--space-4)));
    overflow: visible;
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    box-shadow: var(--shadow-drawer);
    color: var(--color-text);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
    text-align: left;
    cursor: pointer;
  }

  .popup:hover {
    background: var(--color-surface-sunken);
  }

  .message {
    flex: 1;
    min-width: 0;
  }

  .ti {
    flex: none;
    font-size: var(--space-4);
    margin-top: calc(var(--space-1) / 2);
    color: var(--color-text-muted);
  }
</style>
