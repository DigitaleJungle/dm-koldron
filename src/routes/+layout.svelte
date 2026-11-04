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

{@render children()}
<Notification />
