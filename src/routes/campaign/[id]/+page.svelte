<script lang="ts">
  import { campaigns } from "$lib/stores/campaigns.svelte";
</script>

<svelte:head><title>{campaigns.current?.name}</title></svelte:head>

<!-- Placeholder: Home's party and encounters come in TODO 11. -->
<main>
  <h1>{campaigns.current?.name}</h1>
</main>

<style>
  main {
    padding: var(--space-6);
  }

  h1 {
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
  }
</style>
