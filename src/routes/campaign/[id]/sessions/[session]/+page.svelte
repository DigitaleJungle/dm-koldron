<script lang="ts">
  import { page } from "$app/state";
  import Session from "$lib/views/sessions/Session.svelte";
</script>

<!-- Keyed, so opening another session (or a new one) starts the screen afresh. -->
{#key page.params.session}
  <Session id={page.params.session === "new" ? null : (page.params.session ?? null)} />
{/key}
