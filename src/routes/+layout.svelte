<script lang="ts">
  import "$lib/styles/tokens.css";
  import "$lib/styles/base.css";
  // shortcut: filled icons left out; their stylesheet redefines .ti and every .ti-* name, so loading both breaks outline icons. Add a scoped filled class when a filled icon is needed.
  import "../../packs/tabler-icons/tabler-icons.min.css";
  import type { Snippet } from "svelte";
  import { onMount } from "svelte";
  import Notification from "$lib/components/Notification.svelte";
  import { loadTheme } from "$lib/stores/theme.svelte";
  import { trackHistory } from "$lib/stores/history.svelte";

  let { children }: { children: Snippet } = $props();

  trackHistory();
  onMount(loadTheme);
</script>

<!-- How the user last moved around: a click or tap hides focus rings, Tab brings them back (base.css). Only Tab, not
     typing, so writing in a field just clicked doesn't show its ring. -->
<svelte:document
  onpointerdowncapture={() => (document.documentElement.dataset.input = "pointer")}
  onkeydowncapture={(e) => e.key === "Tab" && (document.documentElement.dataset.input = "keyboard")}
/>

{@render children()}
<Notification />
