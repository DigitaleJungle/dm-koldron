<script lang="ts">
  import { onMount, tick } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { rulesChapter, rulesContents, type Chapter, type ContentsChapter } from "$lib/api/rules";
  import { errorMessage } from "$lib/api/storage";
  import SettingsButton from "$lib/components/SettingsButton.svelte";
  import type { SectionId } from "$lib/components/SettingsDialog.svelte";
  import { reduceMotion } from "$lib/stores/theme.svelte";
  import { strings } from "$lib/strings";
  import RulesContents from "./RulesContents.svelte";

  const t = strings.library;
  const COLLAPSED_KEY = "library.contentsCollapsed";

  let contents = $state<ContentsChapter[]>();
  let contentsError = $state<string>();
  let chapter = $state<Chapter>();
  let chapterError = $state<string>();
  let settingsOpen = $state(false);
  let settingsSection = $state<SectionId>();
  let pane: HTMLElement | undefined = $state();

  // Remembered on this device only; storage can be unavailable, then the contents show.
  let collapsed = $state(readCollapsed());
  function readCollapsed() {
    try {
      return localStorage.getItem(COLLAPSED_KEY) === "1";
    } catch {
      return false;
    }
  }
  function setCollapsed(value: boolean) {
    collapsed = value;
    try {
      localStorage.setItem(COLLAPSED_KEY, value ? "1" : "0");
    } catch {
      // not remembered, nothing else lost
    }
  }

  // Where you are is in the address: ?pack=…&chapter=…&section=…
  const params = $derived(page.url.searchParams);
  const current = $derived(
    contents?.find((c) => c.pack === params.get("pack") && c.index === params.get("chapter")) ?? contents?.[0],
  );
  const section = $derived(params.get("section"));
  // Plain values, so a reloaded contents list with the same chapter doesn't reload it (and lose the scroll position).
  const currentPack = $derived(current?.pack);
  const currentIndex = $derived(current?.index);

  function loadContents() {
    contentsError = undefined;
    rulesContents()
      .then((list) => (contents = list))
      .catch((e) => (contentsError = errorMessage(e)));
  }
  onMount(loadContents);

  // Packs may have been switched on or off in Settings, so reload when it closes.
  let settingsWasOpen = false;
  $effect(() => {
    if (settingsWasOpen && !settingsOpen) {
      settingsSection = undefined;
      loadContents();
    }
    settingsWasOpen = settingsOpen;
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

  async function scrollToSection(id: string | null) {
    await tick();
    const target = id ? document.getElementById(`section-${id}`) : null;
    if (target) target.scrollIntoView({ behavior: reduceMotion() ? "auto" : "smooth", block: "start" });
    else pane?.scrollTo({ top: 0 });
  }

  $effect(() => {
    if (chapter) scrollToSection(section);
  });

  // Replaces the address, so Back leaves the Library instead of stepping through chapters.
  function open(c: ContentsChapter, sectionIndex?: string) {
    const query = new URLSearchParams({ pack: c.pack, chapter: c.index });
    if (sectionIndex) query.set("section", sectionIndex);
    if (`?${query}` === page.url.search) scrollToSection(sectionIndex ?? null); // same address: just scroll again
    else goto(`?${query}`, { replaceState: true, keepFocus: true, noScroll: true });
  }

  function back(event: MouseEvent) {
    if (history.length <= 1) return; // opened right here: the link goes to the Start screen
    event.preventDefault();
    history.back();
  }

  function openRulesSettings() {
    settingsSection = "rules";
    settingsOpen = true;
  }

  const hasContents = $derived(!!contents && contents.length > 0);
</script>

<svelte:head><title>{t.title}</title></svelte:head>

<div class="library">
  <header class="top">
    <a class="btn back" href="/" onclick={back}>
      <i class="ti ti-chevron-left" aria-hidden="true"></i>
      {t.back}
    </a>
    {#if hasContents}
      <button
        class="icon-btn"
        aria-expanded={!collapsed}
        aria-controls="library-contents"
        aria-label={collapsed ? t.showContents : t.hideContents}
        title={collapsed ? t.showContents : t.hideContents}
        onclick={() => setCollapsed(!collapsed)}
      >
        <i class="ti {collapsed ? 'ti-layout-sidebar-left-expand' : 'ti-layout-sidebar-left-collapse'}" aria-hidden="true"></i>
      </button>
    {/if}
  </header>

  {#if contentsError}
    <p class="message error" role="alert">{t.loadError(contentsError)}</p>
  {:else if !contents}
    <p class="message" role="status">{t.loading}</p>
  {:else if contents.length === 0}
    <div class="empty">
      <h1>{t.empty}</h1>
      <button class="btn btn-primary" onclick={openRulesSettings}>{t.openSettings}</button>
    </div>
  {:else}
    <div class="body">
      {#if !collapsed}
        <aside id="library-contents">
          <div class="tree">
            <RulesContents {contents} {current} {section} onopen={open} />
          </div>
          <div class="gear"><SettingsButton bind:open={settingsOpen} section={settingsSection} /></div>
        </aside>
      {/if}
      <main bind:this={pane}>
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

  {#if !hasContents || collapsed}
    <div class="corner"><SettingsButton bind:open={settingsOpen} section={settingsSection} /></div>
  {/if}
</div>

<style>
  .library {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .top {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-5);
    border-bottom: var(--border-width) solid var(--color-border);
    background: var(--color-surface);
  }

  .back {
    gap: var(--space-1);
    padding-left: var(--space-2);
    color: var(--color-text);
    text-decoration: none;
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--control-height);
    height: var(--control-height);
    padding: 0;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .icon-btn:hover {
    background: var(--color-surface-sunken);
    color: var(--color-text);
  }

  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  aside {
    flex: 0 0 var(--library-contents-width);
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: var(--border-width) solid var(--color-border);
    background: var(--color-surface);
  }

  .tree {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-3);
  }

  .gear {
    flex: none;
    padding: var(--space-2) var(--space-4);
  }

  .corner {
    position: fixed;
    left: var(--space-4);
    bottom: var(--space-2);
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

  .prose {
    max-width: var(--reading-width);
    margin: 0 auto;
    line-height: var(--leading-normal);
  }

  .prose h1 {
    margin-bottom: var(--space-4);
    font-size: var(--text-2xl);
    font-weight: var(--weight-bold);
    letter-spacing: var(--tracking-tight);
  }

  .prose section {
    padding-top: var(--space-6);
    scroll-margin-top: var(--space-4);
  }

  .prose h2 {
    margin-bottom: var(--space-3);
    padding-bottom: var(--space-2);
    border-bottom: var(--border-width) solid var(--color-border);
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
  }

  .prose :global(h3) {
    margin: var(--space-5) 0 var(--space-2);
    font-size: var(--text-lg);
    font-weight: var(--weight-semibold);
  }

  .prose :global(h4) {
    margin: var(--space-4) 0 var(--space-2);
    font-size: var(--text-md);
    font-weight: var(--weight-semibold);
  }

  .prose :global(p),
  .prose :global(ul),
  .prose :global(ol) {
    margin: 0 0 var(--space-3);
  }

  .prose :global(ul),
  .prose :global(ol) {
    padding-left: var(--space-6);
  }

  .prose :global(li + li) {
    margin-top: var(--space-1);
  }

  .prose :global(blockquote) {
    margin: var(--space-4) 0;
    padding: var(--space-3) var(--space-4);
    border-left: var(--space-1) solid var(--color-accent);
    border-radius: var(--radius-sm);
    background: var(--color-surface-sunken);
  }

  .prose :global(blockquote > :last-child) {
    margin-bottom: 0;
  }

  .prose :global(table) {
    width: 100%;
    margin: 0 0 var(--space-4);
    border-collapse: collapse;
    font-size: var(--text-sm);
  }

  .prose :global(th),
  .prose :global(td) {
    padding: var(--space-1) var(--space-2);
    border-bottom: var(--border-width) solid var(--color-border);
    text-align: left;
    vertical-align: top;
  }

  .prose :global(th) {
    font-weight: var(--weight-semibold);
    background: var(--color-surface-sunken);
  }

  /* Narrow windows: the contents above the text, in a box of its own. */
  @media (max-width: 720px) {
    .body {
      flex-direction: column;
    }

    aside {
      flex: 0 0 var(--library-stacked-height);
      border-right: none;
      border-bottom: var(--border-width) solid var(--color-border);
    }

    main {
      padding: var(--space-4);
    }
  }
</style>
