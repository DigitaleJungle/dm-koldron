<script lang="ts">
  // A Homebrew entry form's live preview: the draft as the Catalog shows it (its page's own rendering), worked out in
  // Rust from the book as it would be saved, a moment after the typing stops. A subclass shows on its class's page.
  import { untrack } from "svelte";
  import type { CatalogEntry } from "$lib/api/catalog";
  import { homebrewPreview, type Book, type PackEntry } from "$lib/api/homebrew";
  import { errorMessage } from "$lib/api/storage";
  import PackPicture from "$lib/components/PackPicture.svelte";
  import TagPreview from "$lib/components/TagPreview.svelte";
  import { strings } from "$lib/strings";
  import EntryBody from "$lib/views/library/EntryBody.svelte";
  import { entryCategory } from "$lib/views/library/catalog";
  import type { EntryType } from "./entryTypes";
  import { saveEntry } from "./linked";

  let { book, type, draft }: { book: Book; type: EntryType; draft: PackEntry } = $props();
  const t = strings.homebrew.editor;
  const WAIT = 250;
  let loaded = $state<CatalogEntry>();
  let error = $state<string | null>(null);

  $effect(() => {
    const entry = $state.snapshot(draft) as PackEntry;
    const pack = $state.snapshot(book) as Book;
    let stale = false;
    const timer = setTimeout(
      () => {
        const { book: next, index } = saveEntry(pack, type, entry);
        const parent = (entry.class as { index?: unknown } | undefined)?.index;
        const [kind, at] = type.id === "subclasses" ? ["classes", typeof parent === "string" ? parent : ""] : [type.id, index];
        if (!at) return void (loaded = undefined);
        homebrewPreview(next, kind, at).then(
          (shown) => {
            if (stale) return;
            loaded = shown;
            error = null;
          },
          (e) => !stale && (error = t.previewError(errorMessage(e))),
        );
      },
      untrack(() => loaded) ? WAIT : 0,
    );
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  });
</script>

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}
{#if loaded}
  <TagPreview>
    <article class="prose preview-entry">
      {#if typeof loaded.entry.image === "string" && loaded.pack}
        <PackPicture pack={loaded.pack} image={loaded.entry.image} />
      {/if}
      <p class="name">{loaded.entry.name || t.unnamed}</p>
      <EntryBody category={entryCategory(type.id === "subclasses" ? "classes" : type.id)} {loaded} />
    </article>
  </TagPreview>
{/if}

<style>
  /* The dialog is the card: its padding is the only space around the entry, the same on every side. */
  .preview-entry > :global(:first-child) {
    margin-top: 0;
  }

  .preview-entry > :global(:last-child) {
    margin-bottom: 0;
  }

  .name {
    margin: 0 0 var(--space-2);
    /* Clear of the dialog's ✕ in the corner. */
    padding-inline-end: var(--control-height);
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: var(--weight-display);
    line-height: var(--leading-tight);
  }

  .error {
    color: var(--color-danger);
  }
</style>
