<script lang="ts">
  import { page } from "$app/state";
  import Prepare from "$lib/views/encounters/Prepare.svelte";
</script>

<!-- Keyed, so opening another encounter (or a new one) starts the screen afresh. -->
{#key page.params.encounter}
  <Prepare id={page.params.encounter === "new" ? null : (page.params.encounter ?? null)} />
{/key}
