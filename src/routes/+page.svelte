<script lang="ts">
  import { strings } from "$lib/strings";
</script>

<main>
  <h1>{strings.appName}</h1>
  <p>{strings.startPlaceholder}</p>
</main>

<style>
  main {
    padding: var(--space-7);
  }
  p {
    color: var(--color-text-muted);
  }
</style>
