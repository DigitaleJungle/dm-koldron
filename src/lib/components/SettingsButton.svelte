<script lang="ts">
  import { strings } from "$lib/strings";
  import SettingsDialog, { type SectionId } from "$lib/components/SettingsDialog.svelte";

  // `section`: the one to show when it opens; the gear keeps the last one shown.
  let { open = $bindable(false), section }: { open?: boolean; section?: SectionId } = $props();
</script>

<button class="settings" aria-label={strings.settings.open} title={strings.settings.open} aria-haspopup="dialog" onclick={() => (open = true)}>
  <i class="ti ti-settings" aria-hidden="true"></i>
</button>

<SettingsDialog bind:open {section} />

<style>
  .settings {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--control-height);
    height: var(--control-height);
    padding: 0;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .settings:hover {
    background: var(--color-surface-sunken);
    color: var(--color-text);
  }
</style>
