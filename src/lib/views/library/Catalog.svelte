<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { catalogRows, type CatalogRow } from "$lib/api/catalog";
  import { errorMessage } from "$lib/api/storage";
  import { strings } from "$lib/strings";
  import { CATEGORIES, ENTRY_CATEGORIES, categoryById, entryCategory, filterOptions, selectRows } from "./catalog";
  import CatalogEntry from "./CatalogEntry.svelte";

  let { reloadKey }: { reloadKey: number } = $props();

  const t = strings.catalog;
  const PAGE_SIZE = 50;

  // Everything is in the address: ?tab=catalog&cat=…&q=…&sort=[-]column&page=…&f.<filter>=…&entry=<pack>/<index>
  const params = $derived(page.url.searchParams);
  const category = $derived(categoryById(params.get("cat")));
  const sortParam = $derived(params.get("sort") ?? "name");
  const sort = $derived(sortParam.replace(/^-/, ""));
  const desc = $derived(sortParam.startsWith("-"));
  const chosen = $derived(Object.fromEntries(category.filters.map((f) => [f.id, params.getAll(`f.${f.id}`)])));
  const entry = $derived(params.get("entry"));

  // Typed text filters at once; the address follows, so it's kept when coming back from an entry.
  let search = $state(page.url.searchParams.get("q") ?? "");
  let rows = $state<CatalogRow[]>();
  let error = $state<string>();

  $effect(() => {
    const { id, fields } = category;
    void reloadKey;
    rows = undefined;
    error = undefined;
    let stale = false;
    // The All tab loads every entry type, with no extra fields.
    const load = id === "all" ? Promise.all(ENTRY_CATEGORIES.map((cat) => catalogRows(cat.id, []))).then((lists) => lists.flat()) : catalogRows(id, fields);
    load
      .then((list) => !stale && (rows = list))
      .catch((e) => !stale && (error = errorMessage(e)));
    return () => (stale = true);
  });

  const filters = $derived(rows ? category.filters.map((f) => ({ ...f, options: filterOptions(rows!, f) })).filter((f) => f.options.length > 1) : []);
  const selected = $derived(rows ? selectRows(rows, category, { search, filters: chosen, sort, desc }) : []);
  const pages = $derived(Math.max(1, Math.ceil(selected.length / PAGE_SIZE)));
  const pageNumber = $derived(Math.min(pages, Math.max(1, Number(params.get("page")) || 1)));
  const shown = $derived(selected.slice((pageNumber - 1) * PAGE_SIZE, pageNumber * PAGE_SIZE));

  /** The address with these changes; null removes a parameter. Anything but paging starts at page 1 again. */
  function href(changes: Record<string, string | string[] | null>) {
    const next = new URLSearchParams(params);
    next.set("tab", "catalog");
    if (!("page" in changes)) next.delete("page");
    for (const [key, value] of Object.entries(changes)) {
      next.delete(key);
      for (const v of value === null ? [] : Array.isArray(value) ? value : [value]) next.append(key, v);
    }
    return `?${next}`;
  }

  // Replaces the address, so Back leaves the Library; keeps focus in the search box while typing.
  const go = (changes: Record<string, string | string[] | null>) => goto(href(changes), { replaceState: true, keepFocus: true, noScroll: true });

  function toggleOption(filterId: string, option: string) {
    const current = chosen[filterId] ?? [];
    go({ [`f.${filterId}`]: current.includes(option) ? current.filter((o) => o !== option) : [...current, option] });
  }

  function sortBy(id: string) {
    go({ sort: sort === id && !desc ? `-${id}` : id });
  }

  const ariaSort = (id: string) => (sort === id ? (desc ? "descending" : "ascending") : undefined);
  const entryHref = (row: CatalogRow) => href({ entry: `${row.pack}/${row.index}`, kind: category.id === "all" ? row.kind : null, page: String(pageNumber) });
  const anyFilter = $derived(Object.values(chosen).some((options) => options.length > 0));
  const clearFilters = () => go(Object.fromEntries(category.filters.map((f) => [`f.${f.id}`, null])));
  // A category tab keeps nothing of the last one's search, filters or sort.
  function openCategory(event: MouseEvent, id: string) {
    event.preventDefault();
    search = "";
    goto(`?tab=catalog&cat=${id}`, { replaceState: true });
  }
</script>

{#if entry}
  {@const [pack, ...rest] = entry.split("/")}
  <CatalogEntry
    category={category.id === "all" ? entryCategory(params.get("kind") ?? "") : category}
    backLabel={category.label}
    {pack}
    index={rest.join("/")}
    {reloadKey}
    backHref={href({ entry: null, kind: null, page: params.get("page") })}
  />
{:else}
  <div class="catalog">
    <nav class="categories" aria-label={t.categoriesLabel}>
      {#each CATEGORIES as cat (cat.id)}
        <a href="?tab=catalog&cat={cat.id}" onclick={(e) => openCategory(e, cat.id)} aria-current={cat.id === category.id ? "page" : undefined}>{cat.label}</a>
      {/each}
    </nav>

    <div class="tools">
      <label for="catalog-search" class="sr-only">{t.search(category.label)}</label>
      <input id="catalog-search" type="search" placeholder={t.search(category.label)} bind:value={search} oninput={() => go({ q: search || null })} />
      {#each filters as filter (filter.id)}
        <fieldset class="filter">
          <legend class="label">{filter.label}</legend>
          {#each filter.options as option (option)}
            <button class="chip" aria-pressed={chosen[filter.id]?.includes(option) ?? false} onclick={() => toggleOption(filter.id, option)}>{option}</button>
          {/each}
        </fieldset>
      {/each}
      {#if anyFilter}
        <button class="btn clear" onclick={clearFilters}>
          <i class="ti ti-x" aria-hidden="true"></i>
          {t.clearFilters}
        </button>
      {/if}
    </div>

    {#if error}
      <p class="message error" role="alert">{t.loadError(error)}</p>
    {:else if !rows}
      <p class="message" role="status">{t.loading}</p>
    {:else if rows.length === 0}
      <p class="message">{t.empty(category.label)}</p>
    {:else if selected.length === 0}
      <p class="message">{t.noMatches}</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              {#each [{ id: "name", label: t.columns.name }, ...category.columns, { id: "source", label: t.columns.source }] as col (col.id)}
                <th aria-sort={ariaSort(col.id)}>
                  <button class="sort" onclick={() => sortBy(col.id)}>
                    {col.label}
                    {#if sort === col.id}<i class="ti {desc ? 'ti-chevron-down' : 'ti-chevron-up'}" aria-hidden="true"></i>{/if}
                  </button>
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            <!-- The type is in the key: an index is only unique within its type (All has the shield spell and the shield). -->
            {#each shown as row (`${row.kind}/${row.pack}/${row.index}`)}
              <!-- The name is the link (keyboard); the whole row is clickable for the mouse. -->
              <tr onclick={() => goto(entryHref(row), { replaceState: true })}>
                <td class="name"><a href={entryHref(row)} data-sveltekit-replacestate onclick={(e) => e.stopPropagation()}>{row.name}</a></td>
                {#each category.columns as col (col.id)}
                  <td>{col.value(row)}</td>
                {/each}
                <td class="muted">{row.source}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      {#if pages > 1}
        <div class="paging">
          <a class="btn" href={href({ page: String(pageNumber - 1) })} data-sveltekit-replacestate aria-label={t.previous} title={t.previous} aria-disabled={pageNumber === 1} class:disabled={pageNumber === 1}>
            <i class="ti ti-chevron-left" aria-hidden="true"></i>
          </a>
          <span role="status">{t.range((pageNumber - 1) * PAGE_SIZE + 1, Math.min(pageNumber * PAGE_SIZE, selected.length), selected.length)}</span>
          <a class="btn" href={href({ page: String(pageNumber + 1) })} data-sveltekit-replacestate aria-label={t.next} title={t.next} aria-disabled={pageNumber === pages} class:disabled={pageNumber === pages}>
            <i class="ti ti-chevron-right" aria-hidden="true"></i>
          </a>
        </div>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .catalog {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-6) var(--corner-clearance);
  }

  .categories {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    border-bottom: var(--border-width) solid var(--color-border);
  }

  .categories a {
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--space-1) solid transparent;
    color: var(--color-text-muted);
    font-weight: var(--weight-medium);
    text-decoration: none;
  }

  .categories a[aria-current="page"] {
    border-bottom-color: var(--color-accent);
    color: var(--color-text);
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: var(--space-3) var(--space-5);
  }

  input[type="search"] {
    width: min(100%, var(--select-width));
  }

  .filter {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    border: none;
  }

  .filter legend {
    float: left;
    margin-right: var(--space-2);
  }

  .chip {
    padding: 0 var(--space-2);
    min-height: var(--control-height-sm);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-pill);
    background: var(--color-surface);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .chip:hover {
    background: var(--color-surface-sunken);
  }

  .chip[aria-pressed="true"] {
    border-color: var(--color-accent);
    background: var(--color-accent-soft);
    color: var(--color-link);
  }

  .table-wrap {
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
  }

  th {
    position: sticky;
    top: 0;
    padding: 0;
    border-bottom: var(--border-width) solid var(--color-border-strong);
    background: var(--color-bg);
    text-align: left;
  }

  .sort {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    padding: var(--space-2);
    border: none;
    background: transparent;
    font-weight: var(--weight-semibold);
    text-align: left;
    cursor: pointer;
  }

  td {
    padding: var(--space-2);
    border-bottom: var(--border-width) solid var(--color-border);
    vertical-align: top;
  }

  tbody tr {
    cursor: pointer;
  }

  tbody tr:hover {
    background: var(--color-surface-sunken);
  }

  .name a {
    font-weight: var(--weight-medium);
    text-decoration: none;
  }

  .muted {
    color: var(--color-text-muted);
  }

  .paging {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    font-size: var(--text-sm);
  }

  .paging .btn {
    color: var(--color-text);
    text-decoration: none;
  }

  .paging .disabled {
    opacity: 0.4;
    pointer-events: none;
  }

  .message {
    color: var(--color-text-muted);
  }

  .message.error {
    color: var(--color-danger);
  }

  @media (max-width: 720px) {
    .catalog {
      padding: var(--space-3) var(--space-4) var(--corner-clearance);
    }
  }
</style>
