<script lang="ts">
  import TagPreview from "$lib/components/TagPreview.svelte";
  import { goto } from "$app/navigation";
  import { catalogEntry, type CatalogEntry } from "$lib/api/catalog";
  import Menu from "$lib/components/Menu.svelte";
  import PackPicture from "$lib/components/PackPicture.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import { page } from "$app/state";
  import { errorMessage } from "$lib/api/storage";
  import { libraryHome } from "$lib/stores/campaigns.svelte";
  import { keepScroll } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";
  import { entryAddress, entryCategory, entryFacts, sectionAddress, type Category } from "./catalog";
  import { listHomebrew, type Book } from "$lib/api/homebrew";
  import { entryType } from "$lib/views/homebrew/entryTypes";
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
    // The entry of this pack only: another pack's with the same index (what a reference falls back to) isn't this one.
    catalogEntry(kind, p, i)
      .then((e) => !stale && (e.pack === p ? (loaded = e) : (error = t.notInPack)))
      .catch((e) => !stale && (error = errorMessage(e)));
    return () => (stale = true);
  });

  const facts = $derived(loaded ? entryFacts(category, loaded) : []);

  // A rule section opens in the Rules tab; another entry on its own page. The pack shows when it's another one.
  const alsoHref = (a: Also) => (a.chapterIndex ? sectionAddress(a.pack, a.chapterIndex, a.index, libraryHome()) : entryAddress(a.kind, a.pack, a.index, libraryHome()));
  const alsoLabel = (a: Also) =>
    `${a.chapterName ?? entryCategory(a.kind).label} → ${a.name}${a.pack !== loaded?.pack ? ` (${a.source})` : ""}`;
  // Each opens its place as a step, so Back returns here.
  const alsoItems = $derived((loaded?.also ?? []).map((a) => ({ label: alsoLabel(a), onselect: () => goto(alsoHref(a)) })));

  // Copy as homebrew, into one of the homebrew books (the Homebrew tab is in the Library outside a campaign only).
  const canCopy = $derived(libraryHome() === "/library" && !!entryType(category.id));
  let books = $state<Book[]>([]);
  $effect(() => {
    if (!canCopy) return;
    listHomebrew()
      .then((list) => (books = list.toSorted((a, b) => a.name.localeCompare(b.name))))
      .catch((e) => notify(errorMessage(e)));
  });
  const copyHref = (book: string) =>
    `/library?${new URLSearchParams({ tab: "homebrew", book, type: category.id, entry: "new", from: `${loaded!.pack}/${loaded!.entry.index}` })}`;
  const copyItems = $derived(
    books.length
      ? books.map((b) => ({ label: strings.homebrew.copyInto(b.name), onselect: () => goto(copyHref(b.id)) }))
      : [{ label: strings.homebrew.empty, disabled: true }],
  );
</script>

<div class="entry" use:keepScroll={{ id: "entry", url: page.url.href, ready: !!loaded }}>
  {#if error}
    <p class="message error" role="alert">{t.loadError(error)}</p>
  {:else if !loaded}
    <p class="message" role="status">{t.loading}</p>
  {:else}
    <TagPreview>
    <article class="prose">
      <!-- Its picture, when its pack's images are on (Settings → Rules); none offline before it was downloaded. -->
      {#if typeof loaded.entry.image === "string"}
        <PackPicture pack={loaded.pack} image={loaded.entry.image} />
      {/if}
      <!-- The other places with the same rule, in a menu right of the name. -->
      <div class="title">
        <h1>{loaded.entry.name}</h1>
        {#if alsoItems.length}
          <Menu id="entry-also" label={strings.entry.alsoIn} items={alsoItems} align="start" triggerClass="btn small also">
            {#snippet trigger()}
              {strings.entry.alsoIn}
              <i class="ti ti-chevron-down" aria-hidden="true"></i>
            {/snippet}
          </Menu>
        {/if}
        {#if canCopy}
          <Menu id="entry-copy" label={strings.homebrew.copy} items={copyItems} align="start" triggerClass="btn small also">
            {#snippet trigger()}
              <i class="ti ti-copy" aria-hidden="true"></i>{strings.homebrew.copy}
              <i class="ti ti-chevron-down" aria-hidden="true"></i>
            {/snippet}
          </Menu>
        {/if}
      </div>
      <p class="source">{loaded.source}</p>
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
    </article>
    </TagPreview>
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

  .title {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-4);
    margin-bottom: var(--space-4);
  }

  /* The row keeps the heading's space under it, so the source line sits as before. */
  .title h1 {
    margin-bottom: 0;
  }

  .title :global(.also) {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
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
