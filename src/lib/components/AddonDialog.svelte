<script lang="ts">
  // An addon's page in a modal dialog (an item in the Add character menu, or Edit link on a linked character's form).
  // The page closes it with `ui.close`; Escape and ✕ close it too.
  import type { AddonSummary } from "$lib/api/addons";
  import { strings } from "$lib/strings";
  import AddonFrame from "./AddonFrame.svelte";

  let {
    addon,
    entry,
    title,
    context,
    onclose,
  }: { addon: AddonSummary; entry: string; title: string; context: Record<string, unknown>; onclose: () => void } = $props();
  const id = $props.id();
  let dialog: HTMLDialogElement | undefined = $state();

  $effect(() => {
    if (dialog && !dialog.open) dialog.showModal();
  });
</script>

<dialog bind:this={dialog} class="dialog" aria-labelledby="{id}-title" {onclose}>
  <header>
    <h2 id="{id}-title">{title}</h2>
    <button type="button" class="icon-btn icon-btn-lg" aria-label={strings.addons.close} onclick={onclose}>
      <i class="ti ti-x" aria-hidden="true"></i>
    </button>
  </header>
  <AddonFrame {addon} {entry} {context} {onclose} />
</dialog>

<style>
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    margin-bottom: var(--space-3);
  }

  h2 {
    font-size: var(--text-lg);
    font-weight: var(--weight-semibold);
  }
</style>
