<script lang="ts">
  // A homebrew book's own fields (a new book before its first save), in the Homebrew layout (EditorShell) with one card.
  // Save writes the whole book; Discard goes back to what was saved.
  import { untrack } from "svelte";
  import { goto } from "$app/navigation";
  import { exportHomebrew, saveHomebrew, type Book } from "$lib/api/homebrew";
  import { notify } from "$lib/stores/notifications.svelte";
  import Menu from "$lib/components/Menu.svelte";
  import EditorCard from "./EditorCard.svelte";
  import EditorShell from "./EditorShell.svelte";
  import { ADDABLE_TYPES } from "./entryTypes";
  import { errorMessage } from "$lib/api/storage";
  import { strings } from "$lib/strings";

  let {
    book,
    rulesets,
    place,
    changed = $bindable(false),
    onsaved,
  }: {
    book: Book;
    rulesets: string[];
    /** The address of a place in this book (a new chapter or entry); null before its first save. */
    place: ((at: Record<string, string>) => string) | null;
    changed?: boolean;
    onsaved: (book: Book) => void;
  } = $props();

  const t = strings.homebrew;
  // The loaded packs' rulesets, and the book's own when no loaded pack has it any more.
  const choices = $derived(book.ruleset && !rulesets.includes(book.ruleset) ? [book.ruleset, ...rulesets] : rulesets);
  const fieldsOf = (b: Book) => ({ name: b.name, version: b.version, license: b.license, attribution: b.attribution, ruleset: b.ruleset });
  let draft = $state(untrack(() => fieldsOf(book)));
  let saved = $state(untrack(() => JSON.stringify(fieldsOf(book))));
  let saving = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    changed = JSON.stringify(draft) !== saved;
  });

  async function exportBook() {
    try {
      if (await exportHomebrew(book.id, t.packFile)) notify(t.exported(book.name));
    } catch (e) {
      error = t.exportError(errorMessage(e));
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    try {
      const next = await saveHomebrew({ ...book, ...draft, name: draft.name.trim() });
      draft = fieldsOf(next);
      saved = JSON.stringify(draft);
      changed = false;
      error = null;
      onsaved(next);
    } catch (e) {
      error = t.saveError(errorMessage(e));
    } finally {
      saving = false;
    }
  }
</script>

<EditorShell
  eyebrow={book.id ? t.editor.eyebrowEdit(t.editor.book) : t.editor.eyebrowNew(t.editor.book)}
  title={draft.name.trim() || t.editor.unnamed}
  {changed}
  {saving}
  saveDisabled={saving || (!changed && !!book.id)}
  {error}
  sections={[{ id: "details", label: t.sections.details, missing: !draft.name.trim() }]}
  onsubmit={save}
  ondiscard={() => (draft = JSON.parse(saved))}
>
  {#snippet actions()}
    {#if place}
      <a class="btn add" href={place({ chapter: "new" })}><i class="ti ti-plus" aria-hidden="true"></i>{t.book.addChapter}</a>
      <button class="btn" type="button" onclick={exportBook}><i class="ti ti-file-export" aria-hidden="true"></i>{t.export}</button>
      <Menu id="add-entry" label={t.addEntry} items={ADDABLE_TYPES.map((type) => ({ label: type.label, onselect: () => goto(place({ type: type.id, entry: "new" })) }))} align="end" triggerClass="btn">
        {#snippet trigger()}
          <i class="ti ti-plus" aria-hidden="true"></i>{t.addEntry}<i class="ti ti-chevron-down" aria-hidden="true"></i>
        {/snippet}
      </Menu>
    {/if}
  {/snippet}
  <EditorCard id="details" label={t.sections.details}>
    <label class="field">
      <span>{t.book.name} <span class="muted">({t.required})</span></span>
      <!-- svelte-ignore a11y_autofocus: a new book starts at its name -->
      <input bind:value={draft.name} required maxlength="100" autocomplete="off" autofocus={!book.id} />
    </label>
    <div class="row">
      <label class="field">
        <span>{t.book.version}</span>
        <input bind:value={draft.version} required maxlength="20" autocomplete="off" />
      </label>
      <label class="field">
        <span>{t.book.ruleset}</span>
        <select bind:value={draft.ruleset} required>
          {#each choices as ruleset (ruleset)}<option value={ruleset}>{ruleset}</option>{/each}
        </select>
      </label>
    </div>
    <label class="field">
      <span>{t.book.license}</span>
      <input bind:value={draft.license} required maxlength="100" autocomplete="off" />
    </label>
    <label class="field">
      <span>{t.book.attribution}</span>
      <textarea bind:value={draft.attribution} rows="2" maxlength="1000"></textarea>
    </label>
  </EditorCard>
</EditorShell>

<style>
  .row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(var(--homebrew-number-width), 1fr));
    gap: var(--space-4);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  input,
  select,
  textarea {
    min-height: var(--control-height);
    padding: 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    font-weight: var(--weight-regular);
  }

  textarea {
    padding: var(--space-2) var(--space-3);
    resize: vertical;
  }

  .add {
    color: inherit;
    text-decoration: none;
  }

  .muted {
    color: var(--color-text-muted);
    font-weight: var(--weight-regular);
  }
</style>
