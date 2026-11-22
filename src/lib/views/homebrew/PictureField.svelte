<script lang="ts">
  // A homebrew entry's, chapter's or section's picture, as a square tile: clicking it chooses (or changes) the picture,
  // kept for the book (shrunk to 1024 px) with its name in `value`; ✕ removes it. It's part of the entry once the form
  // is saved. `small`: a tile as tall as a text field (a section's row). `what` names it for screen readers.
  import { putHomebrewPicture } from "$lib/api/homebrew";
  import { errorMessage } from "$lib/api/storage";
  import PackPicture from "$lib/components/PackPicture.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import { strings } from "$lib/strings";

  let { book, value = $bindable(), what, small = false }: { book: string; value: string | undefined; what: string; small?: boolean } = $props();
  const t = strings.homebrew.picture;
  let input: HTMLInputElement | undefined = $state();
  let busy = $state(false);

  async function pick() {
    const file = input?.files?.[0];
    if (!file) return;
    busy = true;
    try {
      value = await putHomebrewPicture(book, new Uint8Array(await file.arrayBuffer()));
    } catch (e) {
      notify(t.error(errorMessage(e)));
    } finally {
      busy = false;
      input!.value = "";
    }
  }
</script>

<!-- Positioned, so the hidden file input stays inside it (not against the page, which would make the page scroll). -->
<div class="picture-field" class:small>
  <input bind:this={input} type="file" accept="image/png,image/jpeg,image/webp,image/gif" class="sr-only" tabindex="-1" onchange={pick} />
  <button class="tile" class:empty={!value} type="button" disabled={busy} aria-label={value ? t.change(what) : t.choose(what)} onclick={() => input?.click()}>
    {#if value}
      <PackPicture pack={book} image={value} fill />
    {:else}
      <i class="ti ti-photo-plus" aria-hidden="true"></i>
      {#if !small}<span>{t.label}</span>{/if}
    {/if}
  </button>
  {#if value}
    <button class="icon-btn remove" type="button" disabled={busy} aria-label={t.remove(what)} onclick={() => (value = undefined)}>
      <i class="ti ti-x" aria-hidden="true"></i>
    </button>
  {/if}
</div>

<style>
  .picture-field {
    position: relative;
    flex: none;
    width: var(--homebrew-picture-size);
    height: var(--homebrew-picture-size);
  }

  .small {
    width: var(--control-height);
    height: var(--control-height);
  }

  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
    width: 100%;
    height: 100%;
    padding: 0;
    overflow: hidden;
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: var(--color-text-muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .tile.empty {
    border-style: dashed;
  }

  .tile:hover:not(:disabled) {
    background: var(--color-surface-sunken);
    color: var(--color-text);
  }

  .tile .ti {
    font-size: var(--icon-size);
  }

  /* ✕ on the picture's corner, readable on any picture. */
  .remove {
    position: absolute;
    top: var(--space-1);
    right: var(--space-1);
    background: var(--color-surface);
  }

  .small .remove {
    top: calc(-1 * var(--space-2));
    right: calc(-1 * var(--space-2));
    border-radius: var(--radius-pill);
  }
</style>
