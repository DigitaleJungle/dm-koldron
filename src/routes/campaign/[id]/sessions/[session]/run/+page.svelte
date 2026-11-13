<script lang="ts">
  import { page } from "$app/state";
  import RunSession from "$lib/views/sessions/RunSession.svelte";
</script>

{#key page.params.session}
  <RunSession id={page.params.session ?? ""} />
{/key}
