<script lang="ts">
  // A card of a Homebrew form (EditorShell): its title opens and closes it. A closed card keeps its fields (hidden), so
  // what's typed stays and the form still checks them.
  import { getContext, type Snippet } from "svelte";
  import { EDITOR, type Editor } from "./EditorShell.svelte";

  let { id, label, children }: { id: string; label: string; children: Snippet } = $props();
  const editor = getContext<Editor>(EDITOR);
  const open = $derived(editor.open[id] ?? true);
  const at = $derived(`${editor.id}-${id}`);
</script>

<section class="card" id={at} data-card={id} aria-labelledby="{at}-title">
  <h2 id="{at}-title">
    <button type="button" aria-expanded={open} aria-controls="{at}-body" onclick={() => (editor.open[id] = !open)}>
      <span>{label}</span>
      <i class="ti ti-chevron-down chevron" aria-hidden="true"></i>
    </button>
  </h2>
  <div class="body" id="{at}-body" hidden={!open}>
    {@render children()}
  </div>
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    /* Scrolled to from "On this page": below the sticky save bar. */
    scroll-margin-top: calc(var(--bar-height, 0px) + var(--space-4));
  }

  h2 {
    margin: 0;
  }

  h2 button {
    width: 100%;
    min-height: var(--control-height-sm);
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-text);
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-display);
    text-align: left;
    cursor: pointer;
  }

  h2 span {
    flex: 1;
  }

  .chevron {
    font-size: var(--icon-size);
    transition: transform var(--duration-hover) var(--ease-hover);
  }

  [aria-expanded="true"] .chevron {
    transform: rotate(180deg);
  }

  :global([data-motion="reduce"]) .chevron {
    transition: none;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .body[hidden] {
    display: none;
  }
</style>
