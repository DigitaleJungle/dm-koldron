<script lang="ts">
  import { strings } from "$lib/strings";
  import SettingsDialog, { type SectionId } from "$lib/components/SettingsDialog.svelte";

  // `section`: the one to show when it opens; the gear keeps the last one shown.
  let { open = $bindable(false), section }: { open?: boolean; section?: SectionId } = $props();
</script>

<button class="icon-btn icon-btn-lg" aria-label={strings.settings.open} aria-haspopup="dialog" onclick={() => (open = true)}>
  <i class="ti ti-settings" aria-hidden="true"></i>
</button>

<SettingsDialog bind:open {section} />
