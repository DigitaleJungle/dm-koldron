<script lang="ts">
  import "$lib/styles/tokens.css";
  import "$lib/styles/base.css";
  // shortcut: filled icons left out; their stylesheet redefines .ti and every .ti-* name, so loading both breaks outline icons. Add a scoped filled class when a filled icon is needed.
  import "../../packs/tabler-icons/tabler-icons.min.css";
  import type { Snippet } from "svelte";
  import { onMount } from "svelte";
  import AddonBackground from "$lib/components/AddonBackground.svelte";
  import CampaignDialog from "$lib/components/CampaignDialog.svelte";
  import Notification from "$lib/components/Notification.svelte";
  import SettingsDialog, { settingsOpen } from "$lib/components/SettingsDialog.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import { checkForUpdate } from "$lib/stores/updates.svelte";
  import { strings } from "$lib/strings";
  import { loadTheme } from "$lib/stores/theme.svelte";
  import { trackHistory } from "$lib/stores/history.svelte";

  let { children }: { children: Snippet } = $props();

  trackHistory();
  onMount(loadTheme);

  // Settings on About, opened from the "new version" notification.
  let aboutOpen = $state(false);
  onMount(() => {
    checkForUpdate().then((latest) => {
      if (latest) notify(strings.updateAvailable(latest), {
          label: strings.viewUpdate,
          // Not over a Settings dialog that's already open: About is one click away in it.
          run: () => !settingsOpen.count && (aboutOpen = true),
        });
    });
  });
</script>

<!-- How the user last moved around: a click or tap hides focus rings, Tab brings them back (base.css). Only Tab, not
     typing, so writing in a field just clicked doesn't show its ring. -->
<svelte:document
  onpointerdowncapture={() => (document.documentElement.dataset.input = "pointer")}
  onkeydowncapture={(e) => e.key === "Tab" && (document.documentElement.dataset.input = "keyboard")}
/>

{@render children()}
<Notification />
<SettingsDialog bind:open={aboutOpen} section="about" />
<CampaignDialog />
<AddonBackground />
