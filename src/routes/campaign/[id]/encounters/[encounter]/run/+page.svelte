<script lang="ts">
  import { page } from "$app/state";
  import Run from "$lib/views/encounters/Run.svelte";
</script>

{#key page.params.encounter}
  <Run id={page.params.encounter ?? ""} />
{/key}
