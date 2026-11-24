<script lang="ts">
  // An entry's page below its name, as the Catalog shows it: a stat block, a spell's details, a class's table… or its
  // facts and text. The Catalog's entry page and the Homebrew form's live preview both show it.
  import type { CatalogEntry } from "$lib/api/catalog";
  import { entryFacts, type Category } from "./catalog";
  import BackgroundDetails from "./entries/BackgroundDetails.svelte";
  import ClassDetails from "./entries/ClassDetails.svelte";
  import MonsterBlock from "./entries/MonsterBlock.svelte";
  import SpeciesDetails from "./entries/SpeciesDetails.svelte";
  import SpellDetails from "./entries/SpellDetails.svelte";

  let { category, loaded }: { category: Category; loaded: CatalogEntry } = $props();
  const facts = $derived(entryFacts(category, loaded));
</script>

{#if category.id === "spells"}
  <SpellDetails {loaded} />
{:else if category.id === "monsters"}
  <MonsterBlock {loaded} />
{:else if category.id === "classes"}
  <ClassDetails {loaded} />
{:else if category.id === "species"}
  <SpeciesDetails {loaded} />
{:else if category.id === "backgrounds"}
  <BackgroundDetails {loaded} />
{:else}
  {#if facts.length}
    <dl class="facts">
      {#each facts as fact (fact.label)}
        <dt>{fact.label}</dt>
        <dd>{fact.value}</dd>
      {/each}
    </dl>
  {/if}
  <!-- Rendered in Rust from the pack's text, with raw HTML, links and images removed (a tag stays a link). -->
  {@html loaded.html}
{/if}
