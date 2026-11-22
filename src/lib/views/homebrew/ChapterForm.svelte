<script lang="ts">
  // A homebrew chapter: its name and intro, then its sections in order (moved by their grip), each with a name and
  // text. Texts are Markdown, as a pack has them. Save writes the whole book; Delete asks first.
  import { untrack } from "svelte";
  import { saveHomebrew, type Book } from "$lib/api/homebrew";
  import { errorMessage } from "$lib/api/storage";
  import OrderList from "$lib/components/OrderList.svelte";
  import RichText from "$lib/components/RichText.svelte";
  import { moveTo } from "$lib/components/sidebarOrder";
  import PictureField from "./PictureField.svelte";
  import { strings } from "$lib/strings";
  import { withChapter, withoutChapter, type Chapter } from "./book";

  let {
    book,
    chapter,
    changed = $bindable(false),
    onsaved,
    ondeleted,
  }: { book: Book; chapter: Chapter; changed?: boolean; onsaved: (book: Book, chapter: string) => void; ondeleted: (book: Book) => void } = $props();

  const t = strings.homebrew;
  const c = t.chapter;
  /** A section row; `id` tells rows apart before a new section has an index. */
  type Row = { id: string; index: string; name: string; desc: string; image?: string };
  let next = 0;
  const rowsOf = (ch: Chapter): Row[] => ch.sections.map((s) => ({ ...s, id: `s${next++}` }));
  const fingerprint = (d: { name: string; desc: string; image?: string; sections: Row[] }) =>
    JSON.stringify([d.name, d.desc, d.image ?? null, d.sections.map((s) => [s.index, s.name, s.desc, s.image ?? null])]);

  let draft = $state<{ name: string; desc: string; image?: string; sections: Row[] }>(
    untrack(() => ({ name: chapter.name, desc: chapter.desc, image: chapter.image, sections: rowsOf(chapter) })),
  );
  let saved = $state(untrack(() => fingerprint(draft)));
  let savedDraft = untrack(() => $state.snapshot(draft));
  /** Bumped by Discard, so the text fields (which own their content once filled) start again from the saved text. */
  let resets = $state(0);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let deleting = $state<string | null>(null); // "chapter", or a section row's id

  $effect(() => {
    changed = fingerprint(draft) !== saved;
  });

  function discard() {
    draft = structuredClone(savedDraft);
    resets++;
  }

  function addSection() {
    draft.sections.push({ id: `s${next++}`, index: "", name: "", desc: "" });
  }

  async function write(nextBook: Book): Promise<Book | null> {
    saving = true;
    try {
      const result = await saveHomebrew(nextBook);
      error = null;
      return result;
    } catch (e) {
      error = t.saveError(errorMessage(e));
      return null;
    } finally {
      saving = false;
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    const { book: nextBook, index } = withChapter(book, { index: chapter.index, name: draft.name, desc: draft.desc, image: draft.image, sections: draft.sections });
    const result = await write(nextBook);
    if (!result) return;
    savedDraft = $state.snapshot(draft);
    saved = fingerprint(draft);
    changed = false;
    onsaved(result, index);
  }

  async function remove() {
    deleting = null;
    const result = await write(withoutChapter(book, chapter.index));
    if (!result) return;
    changed = false;
    ondeleted(result);
  }
</script>

<form class="form" onsubmit={save}>
  <div class="head">
    <label class="field">
      <span>{c.name} <span class="muted">({t.required})</span></span>
      <!-- svelte-ignore a11y_autofocus: a new chapter starts at its name -->
      <input bind:value={draft.name} required maxlength="200" autocomplete="off" autofocus={!chapter.index} />
    </label>
    <PictureField book={book.id} what={t.picture.it} bind:value={draft.image} />
  </div>
  {#key resets}
    <div class="field">
      <span>{c.intro}</span>
      <RichText bind:value={draft.desc} label={c.intro} extended markdown />
    </div>

    <h2 class="label">{c.sections}</h2>
    <OrderList class="sections" items={draft.sections.map((s) => ({ ...s, label: s.name || c.untitledSection }))} onmove={(from, to) => (draft.sections = moveTo(draft.sections, from, to))}>
      {#snippet row(item)}
        {@const i = draft.sections.findIndex((s) => s.id === item.id)}
        <div class="section">
          <div class="section-head">
            <input class="section-name" aria-label={c.sectionName} placeholder={c.sectionName} bind:value={draft.sections[i].name} required maxlength="200" autocomplete="off" />
            <PictureField book={book.id} what={t.picture.of(item.label)} bind:value={draft.sections[i].image} small />
            {#if deleting === item.id}
              <span class="confirm">
                {c.deleteSectionQuestion(item.label)}
                <button class="btn small" type="button" onclick={() => (deleting = null)}>{t.cancel}</button>
                <button class="btn small btn-danger" type="button" onclick={() => { draft.sections.splice(i, 1); deleting = null; }}>{t.delete}</button>
              </span>
            {:else}
              <button class="icon-btn" type="button" aria-label={c.deleteSection(item.label)} onclick={() => (deleting = item.id)}>
                <i class="ti ti-trash" aria-hidden="true"></i>
              </button>
            {/if}
          </div>
          <RichText bind:value={draft.sections[i].desc} label={c.sectionText(item.label)} extended markdown />
        </div>
      {/snippet}
    </OrderList>
  {/key}
  <button class="btn add" type="button" onclick={addSection}><i class="ti ti-plus" aria-hidden="true"></i>{c.addSection}</button>

  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="actions">
    <button class="btn btn-primary" type="submit" disabled={saving || (!changed && !!chapter.index)}>{t.save}</button>
    <button class="btn" type="button" disabled={saving || !changed} onclick={discard}>{t.discard}</button>
    {#if chapter.index}
      {#if deleting === "chapter"}
        <span class="confirm end">
          {c.deleteQuestion(chapter.name)}
          <button class="btn small" type="button" onclick={() => (deleting = null)}>{t.cancel}</button>
          <button class="btn small btn-danger" type="button" disabled={saving} onclick={remove}>{t.delete}</button>
        </span>
      {:else}
        <button class="btn btn-danger end" type="button" disabled={saving} onclick={() => (deleting = "chapter")}>{t.delete}</button>
      {/if}
    {/if}
  </div>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: var(--homebrew-form-width);
    margin-inline: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  input {
    min-height: var(--control-height);
    padding: 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    font-weight: var(--weight-regular);
  }

  /* A section: its fields beside the grip (OrderList puts the grip last in the row). */
  .form :global(.sections) {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .form :global(.sections > li) {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    padding: var(--space-3);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .form :global(.sections > li > .grip) {
    margin-top: calc((var(--control-height) - var(--control-height-sm)) / 2);
  }

  .section {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  /* The name, with the picture's tile beside it. */
  .head {
    display: flex;
    align-items: flex-end;
    gap: var(--space-4);
  }

  .head > .field {
    flex: 1;
    min-width: 0;
  }

  .section-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .section-name {
    flex: 1;
    min-width: 0;
  }

  .confirm {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .add {
    align-self: flex-start;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .end {
    margin-left: auto;
  }

  .small {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
  }

  .muted {
    color: var(--color-text-muted);
    font-weight: var(--weight-regular);
  }

  .error {
    color: var(--color-danger);
  }
</style>
