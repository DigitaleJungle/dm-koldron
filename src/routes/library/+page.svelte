<script lang="ts">
  import Library from "$lib/views/library/Library.svelte";
  import { campaigns } from "$lib/stores/campaigns.svelte";

  // Outside any campaign: every switched-on pack (a campaign's own screens set it again when opened).
  campaigns.scope = null;
</script>

<Library />
