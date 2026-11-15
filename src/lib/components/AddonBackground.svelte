<script lang="ts">
  // Each addon's background page, hidden, for as long as the app is open. Its context says which campaign is open,
  // whether something live is on screen (a running encounter or session) and which screen is shown; it's sent again
  // whenever any of that changes, so the page knows when a screen loads.
  import { page } from "$app/state";
  import { addons, loadAddons } from "$lib/stores/addons.svelte";
  import AddonFrame from "./AddonFrame.svelte";

  loadAddons();
  const context = $derived({ campaign: page.params.id ?? null, live: addons.live > 0, screen: page.url.pathname });
</script>

{#each addons.list.filter((a) => a.background) as addon (addon.id)}
  <AddonFrame {addon} entry={addon.background!} {context} hidden />
{/each}
