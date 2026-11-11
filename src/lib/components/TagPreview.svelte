<script lang="ts">
  // Shown rich text with a preview card for its tags (@ links to a Catalog entry or a character): hovering or focusing a
  // tag shows the entry's facts and text under it; leaving it (and the card) or Escape hides it. The pointer can move
  // into the card to scroll it. Previews are loaded once per tag.
  import type { Snippet } from "svelte";
  import { errorMessage } from "$lib/api/storage";
  import { strings } from "$lib/strings";
  import { loadPreview, type Preview } from "$lib/views/search/preview";
  import { kindLabel } from "$lib/views/search/search";

  let { children }: { children: Snippet } = $props();

  const SHOW_MS = 300;
  const HIDE_MS = 150;
  const id = $props.id();
  let card: HTMLElement | undefined = $state();
  let shown = $state<{ name: string; kind: string; preview: Preview | null; error: string | null } | null>(null);
  let anchor: HTMLAnchorElement | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const cache = new Map<string, Promise<Preview>>();

  const tagOf = (target: EventTarget | null) => (target instanceof Element ? target.closest<HTMLAnchorElement>("a[data-kind]") : null);

  function show(a: HTMLAnchorElement) {
    clearTimeout(timer);
    if (a === anchor) return;
    timer = setTimeout(() => {
      anchor = a;
      const { kind = "", pack = "", index = "", campaign } = a.dataset;
      const key = `${kind}/${pack}/${index}`;
      if (!cache.has(key)) cache.set(key, loadPreview({ kind, pack, index, campaign: campaign ?? null }));
      shown = { name: a.textContent ?? "", kind, preview: null, error: null };
      const current = shown; // the state's proxy, to tell later whether this tag is still the one shown
      cache.get(key)!.then(
        (p) => shown === current && (shown.preview = p),
        (e) => {
          cache.delete(key); // tried again next time
          if (shown === current) shown.error = errorMessage(e);
        },
      );
      a.setAttribute("aria-describedby", `${id}-card`);
    }, anchor ? 0 : SHOW_MS);
  }

  function hide(now = false) {
    clearTimeout(timer);
    const go = () => {
      anchor?.removeAttribute("aria-describedby");
      anchor = null;
      shown = null;
    };
    if (now) go();
    else timer = setTimeout(go, HIDE_MS);
  }

  // Under the tag, kept inside the window; placed again once the preview has loaded (its size changes).
  $effect(() => {
    void shown?.preview;
    void shown?.error;
    if (!shown || !card || !anchor) return;
    if (!card.matches(":popover-open")) card.showPopover();
    const at = anchor.getBoundingClientRect();
    card.style.top = `${Math.max(8, Math.min(at.bottom + 4, innerHeight - card.offsetHeight - 8))}px`;
    card.style.left = `${Math.max(8, Math.min(at.left, innerWidth - card.offsetWidth - 8))}px`;
  });

  $effect(() => () => clearTimeout(timer));
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && shown && hide(true)} onscrollcapture={(e) => shown && !card?.contains(e.target as Node) && hide(true)} />

<!-- svelte-ignore a11y_no_static_element_interactions: the tags inside are links; this only watches them -->
<div
  class="tagged"
  onmouseover={(e) => {
    const a = tagOf(e.target);
    if (a) show(a);
  }}
  onmouseout={(e) => tagOf(e.target) && hide()}
  onfocusin={(e) => {
    const a = tagOf(e.target);
    if (a) show(a);
  }}
  onfocusout={() => hide(true)}
>
  {@render children()}
</div>

{#if shown}
  <!-- svelte-ignore a11y_no_static_element_interactions: hovering keeps it open, nothing to press -->
  <div bind:this={card} id="{id}-card" popover="manual" class="preview-card" role="tooltip" onmouseenter={() => clearTimeout(timer)} onmouseleave={() => hide()}>
    <header>
      <h3>{shown.name}</h3>
      <p class="muted">{[kindLabel(shown.kind), shown.preview?.source].filter(Boolean).join(" · ")}</p>
    </header>
    <div class="preview-body prose">
      {#if shown.error}
        <p class="error">{shown.error}</p>
      {:else if !shown.preview}
        <p class="muted" role="status">{strings.preview.loading}</p>
      {:else}
        {#if shown.preview.facts.length}
          <dl class="facts">
            {#each shown.preview.facts as fact (fact.label)}<dt>{fact.label}</dt><dd>{fact.value}</dd>{/each}
          </dl>
        {/if}
        <!-- Rendered in Rust from the pack's text, with raw HTML, links and images removed. -->
        {@html shown.preview.html}
      {/if}
    </div>
  </div>
{/if}

<style>
  header {
    padding: var(--space-2) var(--space-4);
    border-bottom: var(--border-width) solid var(--color-border);
  }

  header p {
    margin: 0;
  }

  .muted {
    color: var(--color-text-muted);
  }

  .error {
    color: var(--color-danger);
  }
</style>
