<script lang="ts">
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
</script>

<!-- Placeholder: the campaign shell and Home come in later items. -->
<main>
  <a class="btn" href="/"><i class="ti ti-arrow-left" aria-hidden="true"></i> {strings.campaign.backToStart}</a>
  <h1>{campaigns.current?.name}</h1>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-7);
  }

  .btn {
    color: var(--color-text);
    text-decoration: none;
  }
</style>
