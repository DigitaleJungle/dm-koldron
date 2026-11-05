<script lang="ts">
  import BackButton from "$lib/components/BackButton.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
</script>

<!-- Placeholder: the campaign shell and Home come in later items. -->
<main>
  <BackButton />
  <div class="search"><SearchBar /></div>
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

  .search {
    width: min(100%, var(--search-width));
  }
</style>
