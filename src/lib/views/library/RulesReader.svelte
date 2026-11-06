<script lang="ts">
  import { tick } from "svelte";
  import { page } from "$app/state";
  import { rulesChapter, rulesContents, type Chapter, type ContentsChapter } from "$lib/api/rules";
  import { errorMessage } from "$lib/api/storage";
  import { gotoReplacing, keepScroll, reachedByBack } from "$lib/stores/history.svelte";
  import { reduceMotion } from "$lib/stores/theme.svelte";
  import { strings } from "$lib/strings";
  import RulesContents from "./RulesContents.svelte";

  let {
    collapsed,
    reloadKey,
    onopensettings,
    hasContents = $bindable(false),
  }: {
    /** The contents column is hidden. */
    collapsed: boolean;
    /** Changes when the packs may have changed (Settings closed): the contents are loaded again. */
    reloadKey: number;
    onopensettings: () => void;
    hasContents?: boolean;
  } = $props();

  const t = strings.library;

  let contents = $state<ContentsChapter[]>();
  let contentsError = $state<string>();
  let chapter = $state<Chapter>();
  let chapterError = $state<string>();
  let pane: HTMLElement | undefined = $state();

  // Where you are is in the address: ?pack=…&chapter=…&section=…
  const params = $derived(page.url.searchParams);
  const current = $derived(
    contents?.find((c) => c.pack === params.get("pack") && c.index === params.get("chapter")) ?? contents?.[0],
  );
  const section = $derived(params.get("section"));
  // Plain values, so a reloaded contents list with the same chapter doesn't reload it (and lose the scroll position).
  const currentPack = $derived(current?.pack);
  const currentIndex = $derived(current?.index);

  $effect(() => {
    void reloadKey;
    contentsError = undefined;
    rulesContents()
      .then((list) => (contents = list))
      .catch((e) => (contentsError = errorMessage(e)));
  });

  $effect(() => {
    hasContents = !!contents && contents.length > 0;
  });

  $effect(() => {
    const [pack, index] = [currentPack, currentIndex];
    chapter = undefined;
    chapterError = undefined;
    if (!pack || !index) return;
    let stale = false;
    rulesChapter(pack, index)
      .then((loaded) => !stale && (chapter = loaded))
      .catch((e) => !stale && (chapterError = errorMessage(e)));
    return () => (stale = true);
  });

  // Scrolling to a section is animated here (ease-out), since the WebView's own smooth scrolling may not animate.
  // Turning the wheel stops it; with animations off it jumps.
  const SCROLL_MS = 450;
  let scrollFrame = 0;
  const stopScroll = () => cancelAnimationFrame(scrollFrame);

  function scrollPaneTo(to: number) {
    stopScroll();
    if (!pane) return;
    const [from, start] = [pane.scrollTop, performance.now()];
    if (reduceMotion()) return void (pane.scrollTop = to);
    const step = (now: number) => {
      if (!pane) return; // the reader closed mid-animation
      const progress = Math.min(1, (now - start) / SCROLL_MS);
      pane.scrollTop = from + (to - from) * (1 - (1 - progress) ** 3);
      if (progress < 1) scrollFrame = requestAnimationFrame(step);
    };
    scrollFrame = requestAnimationFrame(step);
  }

  async function scrollToSection(id: string | null) {
    await tick();
    const target = id ? document.getElementById(`section-${id}`) : null;
    if (!target || !pane) return void pane?.scrollTo({ top: 0 });
    const margin = parseFloat(getComputedStyle(target).scrollMarginTop) || 0;
    scrollPaneTo(target.getBoundingClientRect().top - pane.getBoundingClientRect().top + pane.scrollTop - margin);
  }

  // Coming back with Back, the pane returns to where it was (keepScroll) instead.
  $effect(() => {
    if (chapter && !reachedByBack()) scrollToSection(section);
  });

  // Opening a chapter or section takes the current place in the history: Back (and the mouse's back button) leave the
  // Library instead of stepping through what was read.
  function open(c: ContentsChapter, sectionIndex?: string) {
    const query = new URLSearchParams({ pack: c.pack, chapter: c.index });
    if (sectionIndex) query.set("section", sectionIndex);
    if (`?${query}` === page.url.search) scrollToSection(sectionIndex ?? null); // same address: just scroll again
    else gotoReplacing(`?${query}`, { keepFocus: true, noScroll: true });
  }
</script>

{#if contentsError}
  <p class="message error" role="alert">{t.loadError(contentsError)}</p>
{:else if !contents}
  <p class="message" role="status">{t.loading}</p>
{:else if contents.length === 0}
  <div class="empty">
    <h1>{t.empty}</h1>
    <button class="btn btn-primary" onclick={onopensettings}>{t.openSettings}</button>
  </div>
{:else}
  <div class="body">
    {#if !collapsed}
      <aside id="library-contents">
        <RulesContents {contents} {current} {section} onopen={open} />
      </aside>
    {/if}
    <main bind:this={pane} use:keepScroll={{ id: "rules", url: page.url.href, ready: !!chapter }} onwheel={stopScroll}>
      {#if chapterError}
        <p class="message error" role="alert">{t.loadError(chapterError)}</p>
      {:else if !chapter}
        <p class="message" role="status">{t.loading}</p>
      {:else}
        <article class="prose">
          <h1>{chapter.name}</h1>
          <!-- Rendered in Rust from the pack's Markdown, with raw HTML, links and images removed. -->
          {@html chapter.html}
          {#each chapter.sections as s (s.index)}
            <section id="section-{s.index}">
              <h2>{s.name}</h2>
              {@html s.html}
            </section>
          {/each}
        </article>
      {/if}
    </main>
  </div>
{/if}

<style>
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  /* The Settings gear sits in the corner over the column's bottom, hence the clearance. */
  aside {
    flex: 0 0 var(--library-contents-width);
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-3) var(--space-3) var(--corner-clearance);
    border-right: var(--border-width) solid var(--color-border);
    background: var(--color-surface);
  }

  main {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: var(--space-6) var(--space-7) var(--corner-clearance);
  }

  .message {
    padding: var(--space-7);
    color: var(--color-text-muted);
  }

  .message.error {
    color: var(--color-danger);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-3);
    margin: var(--space-8) auto;
    padding: 0 var(--space-4);
  }

  .empty h1 {
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
  }

  /* Narrow windows: the contents above the text, in a box of its own. */
  @media (max-width: 720px) {
    .body {
      flex-direction: column;
    }

    aside {
      flex: 0 0 var(--library-stacked-height);
      padding-bottom: var(--space-3);
      border-right: none;
      border-bottom: var(--border-width) solid var(--color-border);
    }

    main {
      padding: var(--space-4) var(--space-4) var(--corner-clearance);
    }
  }
</style>
