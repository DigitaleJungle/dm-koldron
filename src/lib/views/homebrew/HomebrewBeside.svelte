<script lang="ts">
  // The Homebrew form an addon opened beside its Library tab (`homebrew.openDraft`): the Homebrew tab's
  // own form, for a new entry or chapter of a book, filled with the addon's draft and with what it sends later
  // (`homebrew.fill`). Its Book picks the homebrew book (the one last picked, else the first), or
  // makes a new one, until the first save. Once saved it's that entry's form, as in the Homebrew tab; deleting it
  // closes the pane, and so do ✕ and Back, which then goes back (both asking first when there are changes not saved).
  import { beforeNavigate } from "$app/navigation";
  import { page } from "$app/state";
  import type { BeforeNavigate } from "@sveltejs/kit";
  import { listHomebrew, saveHomebrew, type Book } from "$lib/api/homebrew";
  import { listPacks } from "$lib/api/packs";
  import { errorMessage } from "$lib/api/storage";
  import { closeBeside, moveBeside, type Beside } from "$lib/stores/homebrewDraft.svelte";
  import { strings } from "$lib/strings";
  import ChapterForm from "./ChapterForm.svelte";
  import EntryForm from "./EntryForm.svelte";
  import LeaveGuard from "./LeaveGuard.svelte";
  import { blankBook, chapterDraft, chaptersOf, entryDraft } from "./book";
  import { entryType } from "./entryTypes";

  let { open }: { open: Beside } = $props();
  const t = strings.homebrew;

  let books = $state<Book[] | null>(null);
  let rulesets: string[] = [];
  let error = $state<string | null>(null);
  /** New book picked in Book: the name typed for it, until it's made or cancelled. */
  let naming = $state<string | null>(null);
  let making = $state(false);
  let bookError = $state<string | null>(null);
  /** Book's choice for a new book (never a book's id, which is a slug). */
  const NEW = " new";
  let changed = $state(false);
  /** The saved entry's or chapter's index, once saved; before that, the draft. */
  let index = $state<string | null>(null);
  let guard: LeaveGuard | undefined = $state();

  const byName = (list: Book[]) => list.toSorted((a, b) => a.name.localeCompare(b.name));
  Promise.all([listHomebrew(), listPacks()]).then(
    ([list, packs]) => {
      books = byName(list);
      rulesets = [...new Set(packs.filter((p) => p.errors.length === 0).map((p) => p.ruleset))];
      if (!books.some((b) => b.id === open.book)) pick(books[0]?.id ?? "");
      if (!books.length) naming = "";
    },
    (e) => (error = t.loadError(errorMessage(e))),
  );
  const book = $derived(books?.find((b) => b.id === open.book) ?? null);

  function pick(id: string) {
    naming = null;
    bookError = null;
    moveBeside(open.key, id);
  }

  async function makeBook(e: SubmitEvent) {
    e.preventDefault();
    const name = naming?.trim();
    if (!name || !books) return;
    making = true;
    try {
      const made = await saveHomebrew({ ...blankBook(rulesets[0] ?? "", t.book.defaultLicense), name });
      books = byName([...books, made]);
      pick(made.id);
    } catch (err) {
      bookError = t.saveError(errorMessage(err));
    } finally {
      making = false;
    }
  }
  // Leaving the tab closes it (after the guard's question).
  $effect(() => {
    const key = open.key;
    return () => closeBeside(key);
  });

  const kind = $derived(entryType(open.kind));
  const entry = $derived(index === null ? entryDraft(open.draft) : book?.entries[open.kind]?.find((e) => e.index === index));
  const chapter = $derived(index === null ? chapterDraft(open.draft) : book && chaptersOf(book).find((c) => c.index === index));
  const what = $derived(open.kind === "rules" ? t.beside.chapter : (kind?.label ?? open.kind));

  function saved(next: Book, at: string) {
    books = books!.map((b) => (b.id === next.id ? next : b));
    index = at;
  }

  /** Back (the app's button or the mouse's, with history behind it) closes the form and goes back; with changes not
   *  saved it asks first, and Cancel stays. */
  const isBack = (navigation: BeforeNavigate) => navigation.type === "popstate" && (navigation.delta ?? -1) < 0;
  beforeNavigate((navigation) => {
    if (!isBack(navigation)) return;
    if (!changed) return closeBeside(open.key);
    navigation.cancel();
    void guard!.ask().then((leave) => {
      if (!leave) return;
      changed = false;
      closeBeside(open.key);
      history.go(navigation.delta ?? -1);
    });
  });

  async function close() {
    if (!(await guard!.ask())) return;
    changed = false;
    closeBeside(open.key);
  }
</script>

<aside class="beside" aria-labelledby="homebrew-beside-title">
  <header class="head">
    <h2 id="homebrew-beside-title">{what}</h2>
    <button class="icon-btn" aria-label={t.beside.close} onclick={close}><i class="ti ti-x" aria-hidden="true"></i></button>
  </header>
  <div class="scroll">
    {#if error}
      <p class="error" role="alert">{error}</p>
    {:else if !books}
      <p class="muted">{t.loading}</p>
    {:else}
      <div class="book">
        <label class="field">
          <span>{t.beside.book}</span>
          <select
            value={naming === null ? open.book : NEW}
            disabled={index !== null || making}
            onchange={(e) => (e.currentTarget.value === NEW ? (naming = "") : pick(e.currentTarget.value))}
          >
            {#each books as b (b.id)}<option value={b.id}>{b.name}</option>{/each}
            <option value={NEW}>{t.beside.newBook}</option>
          </select>
        </label>
        {#if naming !== null}
          <form class="new-book" onsubmit={makeBook}>
            <input bind:value={naming} aria-label={t.beside.bookName} required maxlength="100" autocomplete="off" />
            <button class="btn btn-primary" disabled={making || !naming.trim()}>{t.beside.makeBook}</button>
            {#if books.length}<button type="button" class="btn" onclick={() => (naming = null)}>{t.cancel}</button>{/if}
          </form>
          {#if bookError}<p class="error" role="alert">{bookError}</p>{/if}
        {/if}
      </div>
    {/if}
    {#if error || !books || !book}
      <!-- nothing more until a book is picked -->
    {:else if open.kind === "rules" && chapter}
      {#key index}
        <ChapterForm {book} {chapter} bind:changed incoming={open.fill} onsaved={saved} ondeleted={() => closeBeside(open.key)} />
      {/key}
    {:else if kind && entry}
      {#key index}
        <EntryForm {book} type={kind} {entry} bind:changed incoming={open.fill} onsaved={saved} ondeleted={() => closeBeside(open.key)} />
      {/key}
    {:else}
      <p class="muted">{t.notFound}</p>
    {/if}
  </div>
</aside>
<!-- The addon moving within its tab (another PDF, a page) keeps the form, and Back is handled above; leaving the tab
     otherwise asks. -->
<LeaveGuard
  bind:this={guard}
  bind:changed
  keeps={(n) => isBack(n) || (n.to!.url.pathname === page.url.pathname && n.to!.url.searchParams.get("tab") === page.url.searchParams.get("tab"))}
/>

<style>
  .beside {
    flex: none;
    width: min(calc(var(--homebrew-form-width) + 2 * var(--space-6)), 50%);
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-left: var(--border-width) solid var(--color-border);
    background: var(--color-bg);
  }

  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4) var(--space-3) var(--space-6);
    border-bottom: var(--border-width) solid var(--color-border);
  }

  h2 {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: var(--text-md);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-4) var(--space-6) var(--corner-clearance);
  }

  .muted {
    color: var(--color-text-muted);
  }

  .book {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
    padding-bottom: var(--space-4);
    border-bottom: var(--border-width) solid var(--color-border);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .new-book {
    display: flex;
    gap: var(--space-2);
  }

  .new-book input {
    flex: 1;
    min-width: 0;
  }

  input,
  select {
    min-height: var(--control-height);
    padding: 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    font-weight: var(--weight-regular);
  }

  .error {
    color: var(--color-danger);
  }
</style>
