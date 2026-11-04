<script lang="ts">
  import { page } from "$app/state";
  import SettingsButton from "$lib/components/SettingsButton.svelte";
  import type { SectionId } from "$lib/components/SettingsDialog.svelte";
  import { strings } from "$lib/strings";
  import "$lib/styles/prose.css";
  import Catalog from "./Catalog.svelte";
  import RulesReader from "./RulesReader.svelte";

  const t = strings.library;
  const COLLAPSED_KEY = "library.contentsCollapsed";

  // ?tab=catalog, else the Rules tab.
  const tab = $derived(page.url.searchParams.get("tab") === "catalog" ? "catalog" : "rules");
  let settingsOpen = $state(false);
  let settingsSection = $state<SectionId>();
  let reloadKey = $state(0);
  let rulesHasContents = $state(false);

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

  // Packs may have been switched on or off in Settings, so the tabs load again when it closes.
  let settingsWasOpen = false;
  $effect(() => {
    if (settingsWasOpen && !settingsOpen) {
      settingsSection = undefined;
      reloadKey++;
    }
    settingsWasOpen = settingsOpen;
  });

  function back(event: MouseEvent) {
    if (history.length <= 1) return; // opened right here: the link goes to the Start screen
    event.preventDefault();
    history.back();
  }

  function openRulesSettings() {
    settingsSection = "rules";
    settingsOpen = true;
  }
</script>

<svelte:head><title>{t.title}</title></svelte:head>

<div class="library">
  <header class="top">
    <a class="btn back" href="/" onclick={back}>
      <i class="ti ti-chevron-left" aria-hidden="true"></i>
      {t.back}
    </a>
    {#if tab === "rules" && rulesHasContents}
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
    <!-- Switching tabs replaces the address, so Back leaves the Library. -->
    <nav class="tabs" aria-label={t.tabs}>
      <a href="/library" data-sveltekit-replacestate aria-current={tab === "rules" ? "page" : undefined}>{t.rulesTab}</a>
      <a href="/library?tab=catalog" data-sveltekit-replacestate aria-current={tab === "catalog" ? "page" : undefined}>{strings.catalog.title}</a>
    </nav>
  </header>

  {#if tab === "catalog"}
    <Catalog {reloadKey} />
  {:else}
    <RulesReader {collapsed} {reloadKey} onopensettings={openRulesSettings} bind:hasContents={rulesHasContents} />
  {/if}

  <div class="corner"><SettingsButton bind:open={settingsOpen} section={settingsSection} /></div>
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

  .tabs {
    display: flex;
    gap: var(--space-1);
    padding: var(--space-1);
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
  }

  .tabs a {
    padding: var(--space-1) var(--space-4);
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    font-weight: var(--weight-medium);
    text-decoration: none;
  }

  .tabs a[aria-current="page"] {
    background: var(--color-accent);
    color: var(--color-accent-text);
  }

  .corner {
    position: fixed;
    left: var(--space-4);
    bottom: var(--space-2);
  }
</style>
