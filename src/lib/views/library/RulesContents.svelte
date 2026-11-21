<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";
  import { strings } from "$lib/strings";
  import type { ContentsChapter } from "$lib/api/rules";
  import { booksOf } from "./rulesContents";

  let {
    contents,
    current,
    section,
    onopen,
  }: {
    contents: ContentsChapter[];
    current: ContentsChapter | undefined;
    section: string | null;
    onopen: (chapter: ContentsChapter, section?: string) => void;
  } = $props();

  const t = strings.library;
  const key = (c: ContentsChapter) => `${c.pack}/${c.index}`;
  let search = $state("");
  const books = $derived(booksOf(contents, search));
  // Book rows only help when there's more than one book; with one, its chapters are the list.
  const manyBooks = $derived(new Set(contents.map((c) => c.pack)).size > 1);
  const expanded = new SvelteSet<string>();
  const openBooks = new SvelteSet<string>();

  // The open chapter shows its sections, and its book its chapters.
  $effect(() => {
    if (!current) return;
    expanded.add(key(current));
    openBooks.add(current.pack);
  });

  const toggle = (set: SvelteSet<string>, id: string) => (set.has(id) ? set.delete(id) : set.add(id));
  // While searching, every book that matches shows its chapters.
  const bookOpen = (pack: string) => !manyBooks || !!search.trim() || openBooks.has(pack);
</script>

<nav aria-label={t.contents}>
  <input
    class="search"
    type="search"
    placeholder={t.searchContents}
    aria-label={t.searchContents}
    bind:value={search}
    onkeydown={(e) => e.key === "Escape" && search && (e.preventDefault(), (search = ""))}
  />
  {#if books.length === 0}
    <p class="none">{t.noContentsMatches}</p>
  {/if}
  {#each books as book (book.pack)}
    {@const open = bookOpen(book.pack)}
    {#if manyBooks}
      <button class="item book" aria-expanded={open} disabled={!!search.trim()} onclick={() => toggle(openBooks, book.pack)}>
        <i class="ti {open ? 'ti-chevron-down' : 'ti-chevron-right'}" aria-hidden="true"></i>
        {book.name}
      </button>
    {/if}
    {#if open}
      <ul class:chapters={manyBooks}>
        {#each book.chapters as chapter (key(chapter))}
          {@const chapterOpen = expanded.has(key(chapter))}
          {@const isCurrent = current && key(current) === key(chapter)}
          <li>
            <div class="row">
              {#if chapter.sections.length > 0}
                <button class="toggle" aria-expanded={chapterOpen} aria-label={t.showSections(chapter.name)} onclick={() => toggle(expanded, key(chapter))}>
                  <i class="ti {chapterOpen ? 'ti-chevron-down' : 'ti-chevron-right'}" aria-hidden="true"></i>
                </button>
              {:else}
                <span class="toggle" aria-hidden="true"></span>
              {/if}
              <button class="item chapter" aria-current={isCurrent && !section ? "location" : undefined} onclick={() => onopen(chapter)}>
                {chapter.name}
              </button>
            </div>
            {#if chapterOpen}
              <ul class="sections">
                {#each chapter.sections as s (s.index)}
                  <li>
                    <button class="item" aria-current={isCurrent && section === s.index ? "location" : undefined} onclick={() => onopen(chapter, s.index)}>
                      {s.name}
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/each}
</nav>

<style>
  nav {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .search {
    width: 100%;
    margin-bottom: var(--space-1);
  }

  .none {
    padding: var(--space-1) var(--space-2);
    font-size: var(--text-sm);
    color: var(--color-text-muted);
  }

  /* A book: its chevron in the chapters' toggle column, then its name. */
  .book {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding-left: 0;
    font-weight: var(--weight-bold);
  }

  .book .ti {
    flex: none;
    width: var(--control-height-sm);
    text-align: center;
    color: var(--color-text-muted);
  }

  /* While searching, the books stay open: the button does nothing then, but looks as usual. */
  .book:disabled {
    color: inherit;
    cursor: default;
  }

  .book:disabled:hover {
    background: transparent;
  }

  .chapters {
    padding-left: var(--space-3);
  }

  .row {
    display: flex;
    align-items: center;
  }

  .toggle {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--control-height-sm);
    height: var(--control-height-sm);
    padding: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .item {
    flex: 1;
    min-width: 0;
    width: 100%;
    padding: var(--space-1) var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .chapter {
    font-weight: var(--weight-semibold);
  }

  .toggle:hover,
  .item:hover {
    background: var(--color-surface-sunken);
  }

  .item[aria-current="location"] {
    background: var(--color-accent-soft);
    color: var(--color-link);
  }

  .sections {
    padding-left: var(--control-height-sm);
  }
</style>
