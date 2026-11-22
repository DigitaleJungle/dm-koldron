<script lang="ts">
  // A 5e-database reference ({ index, name, url }). When it points to a Catalog type it opens a preview card
  // (Escape, ✕ or clicking outside closes it: the browser's popover does that); otherwise it's just the name.
  import { catalogEntry, type CatalogEntry } from "$lib/api/catalog";
  import { errorMessage } from "$lib/api/storage";
  import { libraryHome } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
  import { entryAddress, entryCategory, entryFacts, refKind } from "./catalog";

  let { ref, pack }: { ref: { index: string; name: string; url?: string }; pack: string } = $props();

  const t = strings.preview;
  const id = $props.id();
  const kind = $derived(refKind(ref.url));
  let button: HTMLButtonElement | undefined = $state();
  let card: HTMLElement | undefined = $state();
  let loaded = $state<CatalogEntry>();
  let error = $state<string>();
  const facts = $derived(loaded && kind ? entryFacts(entryCategory(kind), loaded).filter((f) => f.value !== "—") : []);

  // Below the name, kept inside the window.
  function place() {
    if (!button || !card) return;
    const at = button.getBoundingClientRect();
    card.style.top = `${Math.max(8, Math.min(at.bottom + 4, innerHeight - card.offsetHeight - 8))}px`;
    card.style.left = `${Math.max(8, Math.min(at.left, innerWidth - card.offsetWidth - 8))}px`;
  }

  function ontoggle(event: ToggleEvent) {
    if (event.newState !== "open" || !kind) return;
    place();
    if (loaded || error) return;
    catalogEntry(kind, pack, ref.index)
      .then((e) => (loaded = e))
      .catch((e) => (error = errorMessage(e)))
      .finally(() => requestAnimationFrame(place));
  }
</script>

{#if kind}
  <button bind:this={button} class="ref" popovertarget="ref-{id}">{ref.name}</button>
  <div bind:this={card} id="ref-{id}" popover="auto" class="preview-card" role="dialog" aria-label={ref.name} {ontoggle}>
    <header>
      <h3>{ref.name}</h3>
      <button class="icon-btn" popovertarget="ref-{id}" popovertargetaction="hide" aria-label={t.close}>
        <i class="ti ti-x" aria-hidden="true"></i>
      </button>
    </header>
    <div class="preview-body prose">
      {#if error}
        <p class="error">{error}</p>
      {:else if !loaded}
        <p class="muted" role="status">{t.loading}</p>
      {:else}
        <p class="muted">{entryCategory(kind).label} · {loaded.source}</p>
        {#if facts.length}
          <dl class="facts">
            {#each facts as fact (fact.label)}<dt>{fact.label}</dt><dd>{fact.value}</dd>{/each}
          </dl>
        {/if}
        <!-- Rendered in Rust from the pack's text, with raw HTML, links and images removed. -->
        {@html loaded.html}
      {/if}
    </div>
    <footer>
      <a href={entryAddress(kind, loaded?.pack ?? pack, ref.index, libraryHome())}>{t.open}</a>
    </footer>
  </div>
{:else}
  {ref.name}
{/if}

<style>
  .ref {
    padding: 0;
    border: none;
    background: none;
    color: var(--color-link);
    text-decoration: underline dotted;
    text-underline-offset: 2px;
    cursor: pointer;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-4);
    border-bottom: var(--border-width) solid var(--color-border);
  }

  footer {
    padding: var(--space-2) var(--space-4);
    border-top: var(--border-width) solid var(--color-border);
  }

  .muted {
    color: var(--color-text-muted);
  }

  .error {
    color: var(--color-danger);
  }
</style>
