<script lang="ts">
  import { catalogEntry, type CatalogEntry } from "$lib/api/catalog";
  import { errorMessage } from "$lib/api/storage";
  import { strings } from "$lib/strings";
  import type { Category } from "./catalog";

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

  // The same facts as the row's columns; the whole entry's fields stand in for the row's.
  const facts = $derived(
    loaded ? category.columns.map((col) => ({ label: col.label, value: col.value({ ...loaded!, kind: category.id, index, name: "", fields: loaded!.entry }) })) : [],
  );
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
      {#if facts.length}
        <dl>
          {#each facts as fact (fact.label)}
            <dt>{fact.label}</dt>
            <dd>{fact.value}</dd>
          {/each}
        </dl>
      {/if}
      <!-- Rendered in Rust from the pack's text, with raw HTML, links and images removed. -->
      {@html loaded.html}
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

  .source {
    margin-top: calc(-1 * var(--space-3));
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-1) var(--space-4);
    margin: 0 0 var(--space-4);
    font-size: var(--text-sm);
  }

  dt {
    font-weight: var(--weight-semibold);
  }

  dd {
    margin: 0;
  }

  .message {
    color: var(--color-text-muted);
  }

  .message.error {
    color: var(--color-danger);
  }
</style>
