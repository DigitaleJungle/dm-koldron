<script lang="ts">
  // A session's Summary, written in a dialog: the text box on the left, the DM's session notes beside it to work from.
  // End session opens it (with its question), and a Played session's pencil on the overview. `open(text)` shows it
  // with that text; Save (or End session) calls `onsave`, which says whether it worked (the dialog stays open if not).
  import type { LogEntry } from "$lib/api/sessions";
  import { strings } from "$lib/strings";

  let {
    notes,
    message,
    saveLabel,
    error = null,
    onsave,
  }: {
    /** Newest first, as Run session keeps them. */
    notes: LogEntry[];
    /** The question above the summary (End session), if any. */
    message?: string;
    saveLabel: string;
    error?: string | null;
    onsave: (summary: string) => Promise<boolean> | boolean;
  } = $props();

  const t = strings.sessions.live;
  let dialog: HTMLDialogElement | undefined = $state();
  let text = $state("");
  let busy = $state(false);

  export function open(summary: string) {
    text = summary;
    dialog?.showModal();
  }

  async function save() {
    busy = true;
    try {
      if (await onsave(text)) dialog?.close();
    } finally {
      busy = false;
    }
  }
</script>

<dialog bind:this={dialog} class="dialog summary-dialog" aria-labelledby="summary-label">
  {#if message}<p>{message}</p>{/if}
  <div class="body">
    <label class="field">
      <span class="label" id="summary-label">{strings.sessions.sections.summary}</span>
      <textarea class="text-field" bind:value={text} rows="12" maxlength="20000"></textarea>
    </label>
    {#if notes.length > 0}
      <section class="notes" aria-labelledby="summary-notes">
        <span class="label" id="summary-notes">{t.notes}</span>
        <ol>
          {#each notes.toReversed() as note, i (i)}<li>{note.text}</li>{/each}
        </ol>
      </section>
    {/if}
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="actions">
    <button class="btn" onclick={() => dialog?.close()}>{t.cancel}</button>
    <button class="btn btn-primary" disabled={busy} onclick={save}>{saveLabel}</button>
  </div>
</dialog>

<style>
  .summary-dialog {
    width: min(100% - 2 * var(--space-4), calc(var(--search-width) * 2.4));
  }

  p {
    margin: 0;
  }

  /* The summary, and the notes beside it (under it in a narrow window). */
  .body {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .field {
    flex: 2 1 var(--search-width);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  textarea {
    min-height: calc(var(--control-height-xl) * 3);
    padding: var(--space-2);
    resize: vertical;
    font: var(--text-body);
  }

  .notes {
    flex: 1 1 calc(var(--search-width) / 1.5);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    max-height: calc(var(--control-height-xl) * 5);
    overflow-y: auto;
  }

  ol {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding-left: var(--space-5);
    overflow-wrap: anywhere;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>
