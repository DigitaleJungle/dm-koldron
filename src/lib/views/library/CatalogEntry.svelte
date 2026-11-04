<script lang="ts">
  import { catalogEntry, type CatalogEntry } from "$lib/api/catalog";
  import { errorMessage } from "$lib/api/storage";
  import { strings } from "$lib/strings";
  import { entryAddress, entryCategory, entryFacts, sectionAddress, type Category } from "./catalog";
  import type { Also } from "$lib/api/catalog";
  import BackgroundDetails from "./entries/BackgroundDetails.svelte";
  import ClassDetails from "./entries/ClassDetails.svelte";
  import MonsterBlock from "./entries/MonsterBlock.svelte";
  import SpeciesDetails from "./entries/SpeciesDetails.svelte";
  import SpellDetails from "./entries/SpellDetails.svelte";

  let {
    category,
    backLabel,
    pack,
    index,
    reloadKey,
    backHref,
  }: { category: Category; backLabel: string; pack: string; index: string; reloadKey: number; backHref: string } = $props();

  const t = strings.catalog;
  let loaded = $state<CatalogEntry>();
  let error = $state<string>();

  $effect(() => {
    const [kind, p, i] = [category.id, pack, index];
    void reloadKey;
    loaded = undefined;
    error = undefined;
    let stale = false;
    catalogEntry(kind, p, i)
      .then((e) => !stale && (loaded = e))
      .catch((e) => !stale && (error = errorMessage(e)));
    return () => (stale = true);
  });

  const facts = $derived(loaded ? entryFacts(category, loaded) : []);

  // A rule section opens in the Rules tab; another entry on its own page. The pack shows when it's another one.
  const alsoHref = (a: Also) => (a.chapterIndex ? sectionAddress(a.pack, a.chapterIndex, a.index) : entryAddress(a.kind, a.pack, a.index));
  const alsoLabel = (a: Also) =>
    `${a.chapterName ?? entryCategory(a.kind).label} → ${a.name}${a.pack !== loaded?.pack ? ` (${a.source})` : ""}`;
</script>

<div class="entry">
  <a class="btn back" href={backHref} data-sveltekit-replacestate>
    <i class="ti ti-chevron-left" aria-hidden="true"></i>
    {t.back(backLabel)}
  </a>

  {#if error}
    <p class="message error" role="alert">{t.loadError(error)}</p>
  {:else if !loaded}
    <p class="message" role="status">{t.loading}</p>
  {:else}
    <article class="prose">
      <h1>{loaded.entry.name}</h1>
      <p class="source">{loaded.source}</p>
      {#if loaded.also.length}
        <p class="also">
          {strings.entry.alsoIn}
          {#each loaded.also as a, i (`${a.kind}/${a.pack}/${a.index}`)}{#if i > 0},{" "}{/if}<a href={alsoHref(a)} data-sveltekit-replacestate>{alsoLabel(a)}</a>{/each}
        </p>
      {/if}
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
        <!-- Rendered in Rust from the pack's text, with raw HTML, links and images removed. -->
        {@html loaded.html}
      {/if}
    </article>
  {/if}
</div>

<style>
  .entry {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-6) var(--corner-clearance);
  }

  .back {
    align-self: flex-start;
    gap: var(--space-1);
    padding-left: var(--space-2);
    color: var(--color-text);
    text-decoration: none;
  }

  .prose {
    width: 100%;
  }

  .also {
    margin-top: calc(-1 * var(--space-2));
    font-size: var(--text-sm);
  }

  .source {
    margin-top: calc(-1 * var(--space-3));
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .message {
    color: var(--color-text-muted);
  }

  .message.error {
    color: var(--color-danger);
  }
</style>
