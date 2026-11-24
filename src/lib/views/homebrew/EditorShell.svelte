<script module lang="ts">
  /** What the cards share with the form: its id (their element ids start with it) and which cards are open. */
  export type Editor = { id: string; open: Record<string, boolean> };
  export const EDITOR = Symbol("homebrew-editor");
</script>

<script lang="ts">
  // The layout of every Homebrew form (docs/SPEC.md → Homebrew): the save bar on top (what it is, its name, whether it's
  // saved, the eye that opens the live preview in a dialog, the form's own actions, Discard and Save), then the cards
  // (EditorCard) with "On this page" on their right (a link per card, Expand all / Collapse all, Delete). When the pane
  // is narrow (a container query: beside an addon's tab, a small window) the links drop out and Delete goes under the
  // cards.
  import { flushSync, setContext, tick, type Snippet } from "svelte";
  import { reduceMotion } from "$lib/stores/theme.svelte";
  import { strings } from "$lib/strings";

  let {
    eyebrow,
    title,
    changed,
    saving,
    saveDisabled,
    error,
    sections,
    onsubmit,
    ondiscard,
    actions,
    deleter,
    preview,
    children,
  }: {
    eyebrow: string;
    title: string;
    changed: boolean;
    saving: boolean;
    saveDisabled: boolean;
    error: string | null;
    /** The cards, in order; `missing`: a required field in it is empty. */
    sections: { id: string; label: string; missing?: boolean }[];
    onsubmit: (event: SubmitEvent) => void;
    ondiscard: () => void;
    actions?: Snippet;
    deleter?: Snippet;
    preview?: Snippet;
    children: Snippet;
  } = $props();

  const t = strings.homebrew;
  const ed = t.editor;
  const id = $props.id();
  const editor: Editor = $state({ id, open: {} });
  setContext(EDITOR, editor);
  const nav = $derived(sections.length > 1);

  // "Saved" from a save until the next change.
  let justSaved = $state(false);
  let wasSaving = false;
  $effect(() => {
    if (saving) wasSaving = true;
    else if (wasSaving) {
      wasSaving = false;
      justSaved = !changed && !error;
    }
  });
  $effect(() => {
    if (changed) justSaved = false;
  });
  const status = $derived(changed ? "unsaved" : justSaved ? "saved" : "clean");

  let barHeight = $state(0);
  let previewDialog: HTMLDialogElement | undefined = $state();
  // The preview renders only while its dialog is open, so typing doesn't keep asking Rust for it.
  let previewing = $state(false);

  function openAll(open: boolean) {
    for (const s of sections) editor.open[s.id] = open;
  }

  async function go(section: string) {
    editor.open[section] = true;
    await tick();
    const card = document.getElementById(`${id}-${section}`);
    if (!card) return;
    // Only the pane scrolls: scrollIntoView would also scroll the app shell when the card can't reach the top.
    let pane = card.parentElement;
    while (pane && !/auto|scroll/.test(getComputedStyle(pane).overflowY)) pane = pane.parentElement;
    if (pane) {
      const margin = parseFloat(getComputedStyle(card).scrollMarginTop) || 0;
      const top = card.getBoundingClientRect().top - pane.getBoundingClientRect().top + pane.scrollTop - margin;
      pane.scrollTo({ top, behavior: reduceMotion() ? "auto" : "smooth" });
    }
    card.querySelector<HTMLElement>("h2 button")?.focus({ preventScroll: true });
  }

  // A required field in a closed card: open it first, so the browser can show what's missing there.
  function oninvalid(event: Event) {
    const card = (event.target as HTMLElement).closest<HTMLElement>("[data-card]");
    if (card?.dataset.card) flushSync(() => (editor.open[card.dataset.card!] = true));
  }
</script>

<form class="editor" class:nav style:--bar-height="{barHeight}px" {onsubmit} oninvalidcapture={oninvalid}>
  <header class="bar" bind:clientHeight={barHeight}>
    <div class="heading">
      <span class="label">{eyebrow}</span>
      <span class="title-row">
        <strong class="title">{title}</strong>
        <span class="status {status}">{status === "unsaved" ? ed.unsaved : status === "saved" ? ed.saved : ed.clean}</span>
      </span>
    </div>
    <div class="buttons">
      {#if preview}
        <button
          class="btn"
          type="button"
          aria-label={ed.preview}
          aria-haspopup="dialog"
          onclick={() => {
            previewing = true;
            previewDialog?.showModal();
          }}
        >
          <i class="ti ti-eye" aria-hidden="true"></i>
        </button>
      {/if}
      {@render actions?.()}
      <button class="btn" type="button" disabled={saving || !changed} onclick={ondiscard}>{t.discard}</button>
      <button class="btn btn-primary" type="submit" disabled={saveDisabled}>{t.save}</button>
    </div>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  </header>

  <div class="layout">
    <div class="cards">
      {@render children()}
      {#if deleter && !nav}<div class="delete">{@render deleter()}</div>{/if}
    </div>
    {#if nav}
      <div class="side">
        <nav class="links" aria-labelledby="{id}-nav">
          <h2 id="{id}-nav" class="label">{ed.onThisPage}</h2>
          <div class="all">
            <button class="link" type="button" onclick={() => openAll(true)}>{ed.expandAll}</button>
            <button class="link" type="button" onclick={() => openAll(false)}>{ed.collapseAll}</button>
          </div>
          <ul>
            {#each sections as section (section.id)}
              <li>
                <button class="go" type="button" onclick={() => go(section.id)}>
                  {section.label}
                  {#if section.missing}<span class="missing">{t.required}</span>{/if}
                </button>
              </li>
            {/each}
          </ul>
        </nav>
        {#if deleter}<div class="delete">{@render deleter()}</div>{/if}
      </div>
    {/if}
  </div>
</form>

{#if preview}
  <dialog bind:this={previewDialog} class="dialog preview-dialog" aria-label={ed.preview} onclose={() => (previewing = false)}>
    <button class="icon-btn icon-btn-lg close" type="button" aria-label={ed.closePreview} onclick={() => previewDialog?.close()}>
      <i class="ti ti-x" aria-hidden="true"></i>
    </button>
    {#if previewing}{@render preview()}{/if}
  </dialog>
{/if}

<style>
  /* The pane's width decides the layout (below), not the window's. */
  .editor {
    container-type: inline-size;
    /* Absolutely placed parts inside (the ability tiles' hidden labels) stay in the scrolling pane, not the page:
       without it they make the page taller than the window, and it scrolls. */
    position: relative;
  }

  /* Over the pane's top padding (both panes use --space-4), so nothing scrolls by above it. */
  .bar {
    position: sticky;
    top: calc(-1 * var(--space-4));
    z-index: 1;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-4);
    margin-top: calc(-1 * var(--space-4));
    margin-bottom: var(--space-4);
    padding: calc(var(--space-3) + var(--space-4)) 0 var(--space-3);
    border-bottom: var(--border-width) solid var(--color-border);
    background: var(--color-bg);
  }

  .heading {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .title-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1) var(--space-3);
  }

  .title {
    min-width: 0;
    overflow-wrap: anywhere;
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: var(--weight-display);
    line-height: var(--leading-tight);
  }

  .status {
    padding: 0 var(--space-2);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .status.unsaved {
    border-color: var(--color-inspiration);
    color: var(--color-inspiration);
  }

  .status.saved {
    border-color: var(--color-success);
    color: var(--color-success);
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .bar .error {
    flex-basis: 100%;
    margin: 0;
  }

  /* Narrow: the cards only, Delete under them. */
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-5);
    align-items: start;
  }

  .side {
    display: contents;
  }

  .links {
    display: none;
  }

  .cards {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 0;
  }

  /* Wider: the links on the cards' right. */
  @container (min-width: 760px) {
    .editor.nav .layout {
      grid-template-columns: minmax(0, 1fr) var(--homebrew-nav-width);
    }

    .editor.nav .side,
    .editor.nav .links {
      display: flex;
    }
  }

  .side {
    position: sticky;
    /* The bar's height counts the pane's padding it covers. */
    top: var(--bar-height);
    flex-direction: column;
    gap: var(--space-4);
  }

  .links {
    flex-direction: column;
    gap: var(--space-1);
  }

  .links h2 {
    margin: 0;
    padding: 0 var(--space-2);
  }

  .all {
    display: flex;
    gap: var(--space-1);
  }

  .link {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-2);
    border: none;
    background: transparent;
    color: var(--color-link);
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
    cursor: pointer;
  }

  .links ul {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .go {
    width: 100%;
    min-height: var(--control-height);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: 0 var(--space-2);
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text);
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
  }

  .go:hover {
    background: var(--color-surface-sunken);
  }

  .missing {
    color: var(--color-danger);
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
  }

  .side .delete {
    padding-top: var(--space-3);
    border-top: var(--border-width) solid var(--color-border);
  }

  .preview-dialog {
    width: min(100% - 2 * var(--space-4), var(--homebrew-preview-width));
    max-height: var(--homebrew-preview-height);
    overflow-y: auto;
    /* The same room on every side of the entry. */
    padding: var(--space-7);
  }

  /* ✕ in the dialog's top right corner, a small step in from both edges (in the padding), staying there while it
     scrolls; the negative margins take back its row and move it out of the content box. */
  .close {
    position: sticky;
    top: calc(var(--space-2) - var(--space-7));
    z-index: 1;
    align-self: flex-end;
    margin-top: calc(var(--space-2) - var(--space-7));
    margin-right: calc(var(--space-2) - var(--space-7));
    margin-bottom: calc(-1 * (var(--control-height) + var(--space-4)) - var(--space-2) + var(--space-7));
    background: var(--color-surface);
  }

  .error {
    color: var(--color-danger);
  }
</style>
