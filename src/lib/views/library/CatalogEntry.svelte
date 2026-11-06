<script lang="ts">
  import { catalogEntry, type CatalogEntry } from "$lib/api/catalog";
  import { entryImage } from "$lib/api/images";
  import { notify } from "$lib/stores/notifications.svelte";
  import { page } from "$app/state";
  import { errorMessage } from "$lib/api/storage";
  import { keepScroll } from "$lib/stores/history.svelte";
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
    pack,
    index,
    reloadKey,
  }: { category: Category; pack: string; index: string; reloadKey: number } = $props();

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

  // Its picture, when its pack's images are on (Settings → Rules); none offline before it was downloaded.
  let picture = $state<string | null>(null);
  $effect(() => {
    picture = null;
    const image = loaded?.entry.image;
    if (!loaded || typeof image !== "string") return;
    let stale = false;
    entryImage(loaded.pack, image)
      .then((src) => !stale && (picture = src))
      .catch((e) => notify(errorMessage(e))); // reported, but the entry still shows
    return () => (stale = true);
  });

  const facts = $derived(loaded ? entryFacts(category, loaded) : []);

  // A rule section opens in the Rules tab; another entry on its own page. The pack shows when it's another one.
  const alsoHref = (a: Also) => (a.chapterIndex ? sectionAddress(a.pack, a.chapterIndex, a.index) : entryAddress(a.kind, a.pack, a.index));
  const alsoLabel = (a: Also) =>
    `${a.chapterName ?? entryCategory(a.kind).label} → ${a.name}${a.pack !== loaded?.pack ? ` (${a.source})` : ""}`;
</script>

<div class="entry" use:keepScroll={{ id: "entry", url: page.url.href, ready: !!loaded }}>
  {#if error}
    <p class="message error" role="alert">{t.loadError(error)}</p>
  {:else if !loaded}
    <p class="message" role="status">{t.loading}</p>
  {:else}
    <article class="prose">
      {#if picture}
        <img class="picture" src={picture} alt="" />
      {/if}
      <h1>{loaded.entry.name}</h1>
      <p class="source">{loaded.source}</p>
      {#if loaded.also.length}
        <p class="also">
          {strings.entry.alsoIn}
          {#each loaded.also as a, i (`${a.kind}/${a.pack}/${a.index}`)}{#if i > 0},{" "}{/if}<a href={alsoHref(a)}>{alsoLabel(a)}</a>{/each}
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

  .prose {
    width: 100%;
  }

  /* Fixed size, so the text doesn't move when it has loaded. */
  .picture {
    float: right;
    width: var(--entry-picture-size);
    height: var(--entry-picture-size);
    margin: 0 0 var(--space-3) var(--space-4);
    object-fit: contain;
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
