<script lang="ts">
  // An addon's screen in the campaign (its manifest's `campaign`).
  import { page } from "$app/state";
  import CampaignScreen from "$lib/views/addon/CampaignScreen.svelte";
</script>

<CampaignScreen campaign={page.params.id ?? ""} addonId={page.params.addon ?? ""} />
