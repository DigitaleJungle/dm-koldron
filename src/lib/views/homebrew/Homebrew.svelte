<script lang="ts">
  // The Library's Homebrew tab (docs/SPEC.md → Homebrew): the books on the left, each with its chapters, and the form of
  // the book, chapter or entry that's open on the right (`?tab=homebrew&book=<id|new>`, plus `&chapter=<index|new>` or
  // `&type=<type>&entry=<index|new>`). Leaving a form with changes not saved asks first.
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import type { Snippet } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { catalogEntry } from "$lib/api/catalog";
  import { listHomebrew, type Book, type PackEntry } from "$lib/api/homebrew";
  import { listPacks } from "$lib/api/packs";
  import { errorMessage } from "$lib/api/storage";
  import { gotoReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";
  import SideColumn from "../library/SideColumn.svelte";
  import Menu from "$lib/components/Menu.svelte";
  import BookForm from "./BookForm.svelte";
  import ChapterForm from "./ChapterForm.svelte";
  import EntryForm from "./EntryForm.svelte";
  import LeaveGuard from "./LeaveGuard.svelte";
  import { blankBook, chaptersOf } from "./book";
  import { copyDraft } from "./copy";
  import { ADDABLE_TYPES, ENTRY_TYPES, entryType } from "./entryTypes";

  let { reloadKey, bottom }: { reloadKey: number; bottom?: Snippet } = $props();
  const t = strings.homebrew;

  let books = $state<Book[] | null>(null);
  let off = $state(new Set<string>());
  let rulesets = $state<string[]>([]);
  let error = $state<string | null>(null);

  async function load() {
    try {
      const [list, packs] = await Promise.all([listHomebrew(), listPacks()]);
      books = byName(list);
      off = new Set(packs.filter((p) => p.homebrew && !p.enabled).map((p) => p.id));
      rulesets = [...new Set(packs.filter((p) => p.errors.length === 0).map((p) => p.ruleset))];
      error = null;
    } catch (e) {
      error = t.loadError(errorMessage(e));
    }
  }
  // Again when Settings closes: a book may have been switched off or deleted there.
  $effect(() => {
    void reloadKey;
    load();
  });

  const bookId = $derived(page.url.searchParams.get("book"));
  const chapterId = $derived(page.url.searchParams.get("chapter"));
  const book = $derived(bookId === "new" ? blankBook(rulesets[0] ?? "", t.book.defaultLicense) : books?.find((b) => b.id === bookId));
  const chapters = $derived(book ? chaptersOf(book) : []);
  const chapter = $derived(chapterId === "new" ? { index: "", name: "", desc: "", sections: [] } : chapters.find((c) => c.index === chapterId));
  const typeId = $derived(page.url.searchParams.get("type"));
  const entryId = $derived(page.url.searchParams.get("entry"));
  const kind = $derived(entryType(typeId));
  // Copy as homebrew: `&from=<pack>/<index>` fills a new entry from that Catalog entry.
  const from = $derived(entryId === "new" ? page.url.searchParams.get("from") : null);
  let copied = $state<{ from: string; entry: PackEntry } | null>(null);
  $effect(() => {
    if (!from || !kind || copied?.from === `${kind.id}:${from}`) return;
    const key = `${kind.id}:${from}`;
    const [pack, ...index] = from.split("/");
    catalogEntry(kind.id, pack, index.join("/"), null)
      .then((loaded) => (copied = { from: key, entry: copyDraft(kind, loaded) }))
      .catch((e) => (error = t.loadError(errorMessage(e))));
  });
  const copy = $derived(from && kind && copied?.from === `${kind.id}:${from}` ? copied.entry : undefined);
  const entry = $derived(
    entryId === "new" ? (from ? copy : { index: "", name: "" }) : book?.entries[typeId ?? ""]?.find((e) => e.index === entryId),
  );
  const missing = $derived(!book || (chapterId !== null && !chapter) || (typeId !== null && (!kind || !entry)));

  /** A place in the tab: a book, and maybe its `chapter`, or an entry by `type` and `entry`. */
  const href = (book: string, place: Record<string, string> = {}) => `/library?${new URLSearchParams({ tab: "homebrew", book, ...place })}`;
  const byName = <T extends { name: string }>(list: T[]) => list.toSorted((a, b) => a.name.localeCompare(b.name));

  /** A save: the book as saved replaces the listed one, and a new book, chapter or entry's address takes its place. */
  function saved(next: Book, place: Record<string, string> = {}) {
    books = byName([...(books ?? []).filter((b) => b.id !== next.id), next]);
    const here = page.url.searchParams;
    if (bookId !== next.id || Object.entries(place).some(([k, v]) => here.get(k) !== v)) void gotoReplacing(href(next.id, place));
  }

  /** After a delete: the book's own form, then the list. */
  const deleted = (next: Book) => void gotoReplacing(href(next.id)).then(() => saved(next));

  let changed = $state(false);

  /** The books whose chapters and entries show (their arrow); the open book's always does. */
  const openBooks = new SvelteSet<string>();
  $effect(() => {
    if (bookId) openBooks.add(bookId);
  });
  /** The entry groups of a book (`<book>/<type>`) whose entries show; the open entry's always does. */
  const openGroups = new SvelteSet<string>();
  $effect(() => {
    if (bookId && typeId) openGroups.add(`${bookId}/${typeId}`);
  });
  const hasContents = (b: Book) => chaptersOf(b).length > 0 || ENTRY_TYPES.some((type) => b.entries[type.id]?.length);
</script>

<div class="body">
  <SideColumn id="homebrew-books" collapsed={false} {bottom}>
    <a class="btn new" href={href("new")}><i class="ti ti-plus" aria-hidden="true"></i>{t.newBook}</a>
    {#if books?.length}
      <nav aria-label={t.books}>
        <ul class="books">
          {#each books as item (item.id)}
            {@const here = item.id === bookId}
            {@const open = openBooks.has(item.id)}
            <li>
              <div class="row">
                {#if hasContents(item)}
                  <button class="toggle" type="button" aria-expanded={open} aria-label={t.bookContents(item.name)} onclick={() => (open ? openBooks.delete(item.id) : openBooks.add(item.id))}>
                    <i class="ti {open ? 'ti-chevron-down' : 'ti-chevron-right'}" aria-hidden="true"></i>
                  </button>
                {:else}
                  <span class="toggle" aria-hidden="true"></span>
                {/if}
                <a href={href(item.id)} aria-current={here && !chapterId && !typeId ? "page" : undefined}>
                  <span class="name">{item.name}</span>
                  {#if off.has(item.id)}<span class="muted off">{t.off}</span>{/if}
                </a>
                <Menu
                  id="add-to-{item.id}"
                  label={t.addTo(item.name)}
                  items={[
                    { label: t.editor.chapter, onselect: () => goto(href(item.id, { chapter: "new" })) },
                    ...ADDABLE_TYPES.map((type) => ({ label: type.label, onselect: () => goto(href(item.id, { type: type.id, entry: "new" })) })),
                  ]}
                  triggerClass="icon-btn"
                >
                  {#snippet trigger()}<i class="ti ti-plus" aria-hidden="true"></i>{/snippet}
                </Menu>
              </div>
              {#if open && hasContents(item)}
                <ul class="chapters">
                  {#each chaptersOf(item) as c (c.index)}
                    <li><a href={href(item.id, { chapter: c.index })} aria-current={here && c.index === chapterId ? "page" : undefined}>{c.name}</a></li>
                  {/each}
                  {#each ENTRY_TYPES.filter((type) => item.entries[type.id]?.length) as type (type.id)}
                    {@const group = `${item.id}/${type.id}`}
                    {@const groupOpen = openGroups.has(group)}
                    <li class="group">
                      <div class="row">
                        <button class="toggle" type="button" aria-expanded={groupOpen} aria-label={t.groupContents(type.label, item.name)} onclick={() => (groupOpen ? openGroups.delete(group) : openGroups.add(group))}>
                          <i class="ti {groupOpen ? 'ti-chevron-down' : 'ti-chevron-right'}" aria-hidden="true"></i>
                        </button>
                        <span class="label">{type.label}</span>
                      </div>
                      {#if groupOpen}
                        <ul class="chapters">
                          {#each byName(item.entries[type.id]) as e (e.index)}
                            <li><a href={href(item.id, { type: type.id, entry: e.index })} aria-current={here && type.id === typeId && e.index === entryId ? "page" : undefined}>{e.name}</a></li>
                          {/each}
                        </ul>
                      {/if}
                    </li>
                  {/each}
                </ul>
              {/if}
            </li>
          {/each}
        </ul>
      </nav>
    {/if}
  </SideColumn>

  <div class="pane" use:keepScroll={{ id: "homebrew", url: page.url.href, ready: !!books }}>
    {#if error}
      <p class="error" role="alert">{error}</p>
    {:else if !books}
      <p class="muted">{t.loading}</p>
    {:else if from && !copy}
      <p class="muted">{t.copying}</p>
    {:else if bookId && missing}
      <p class="muted">{t.notFound}</p>
    {:else if book && chapter}
      {#key `${book.id}/${chapterId}`}
        <ChapterForm {book} {chapter} bind:changed onsaved={(next, index) => saved(next, { chapter: index })} ondeleted={deleted} />
      {/key}
    {:else if book && kind && entry}
      {#key `${book.id}/${typeId}/${entryId}/${from}`}
        <EntryForm {book} type={kind} {entry} bind:changed onsaved={(next, index) => saved(next, { type: kind.id, entry: index })} ondeleted={deleted} />
      {/key}
    {:else if book}
      {#key book.id || "new"}
        <BookForm {book} {rulesets} place={book.id ? (p) => href(book.id, p) : null} bind:changed onsaved={(next) => saved(next)} />
      {/key}
    {:else if books.length === 0}
      <p class="muted">{t.empty}</p>
    {/if}
  </div>
</div>

<LeaveGuard bind:changed />

<style>
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .new {
    align-self: flex-start;
    margin-bottom: var(--space-3);
    text-decoration: none;
    color: inherit;
  }

  .books,
  .chapters {
    display: flex;
    flex-direction: column;
    list-style: none;
    margin: 0;
    padding: 0;
  }

  /* Under its book's or type's name: past the arrow column. */
  .chapters {
    padding-left: var(--control-height-sm);
  }

  /* A book: its arrow (or the room for one), then its name. */
  .row {
    display: flex;
    align-items: center;
  }

  .row > a {
    flex: 1;
    min-width: 0;
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

  button.toggle:hover {
    background: var(--color-surface-sunken);
  }

  .group {
    margin-top: var(--space-2);
  }

  .group .label {
    padding: 0 var(--space-2);
  }

  .books a {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    color: var(--color-text);
    font-size: var(--text-sm);
    text-decoration: none;
  }

  .books a:hover {
    background: var(--color-surface-sunken);
  }

  .books a[aria-current="page"] {
    background: var(--color-accent-soft);
  }

  .name {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .off {
    margin-left: auto;
    font-size: var(--text-xs);
  }

  .pane {
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-4) var(--space-6) var(--corner-clearance);
  }

  .muted {
    color: var(--color-text-muted);
  }

  .error {
    color: var(--color-danger);
  }
</style>
