<script lang="ts">
  import { page } from "$app/state";
  import BackButton from "$lib/components/BackButton.svelte";
  import { previousUrl } from "$lib/stores/history.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
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
  // Back from a Catalog entry goes to its list (same category, filters, sort and page); from anywhere else in the
  // Library, to the Start screen. Opened from that list, it goes back through history instead, so the list's scroll
  // position comes back too.
  const backTo = $derived.by(() => {
    const params = new URLSearchParams(page.url.searchParams);
    if (tab !== "catalog" || !params.has("entry")) return "/";
    params.delete("entry");
    params.delete("kind");
    return `/library?${params}`;
  });
  const openedFromList = $derived.by(() => {
    const from = previousUrl();
    return backTo !== "/" && from?.pathname === "/library" && from.searchParams.get("tab") === "catalog" && !from.searchParams.has("entry");
  });
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

  function openRulesSettings() {
    settingsSection = "rules";
    settingsOpen = true;
  }
</script>

<svelte:head><title>{t.title}</title></svelte:head>

<div class="library">
  <header class="top">
    <BackButton always={!openedFromList} fallback={backTo} />
    {#if tab === "rules" && rulesHasContents}
      <button
        class="icon-btn icon-btn-lg"
        aria-expanded={!collapsed}
        aria-controls="library-contents"
        aria-label={collapsed ? t.showContents : t.hideContents}
        onclick={() => setCollapsed(!collapsed)}
      >
        <i class="ti {collapsed ? 'ti-layout-sidebar-left-expand' : 'ti-layout-sidebar-left-collapse'}" aria-hidden="true"></i>
      </button>
    {/if}
    <nav class="tabs" aria-label={t.tabs}>
      <a href="/library" aria-current={tab === "rules" ? "page" : undefined}>{t.rulesTab}</a>
      <a href="/library?tab=catalog" aria-current={tab === "catalog" ? "page" : undefined}>{strings.catalog.title}</a>
    </nav>
    <div class="search"><SearchBar /></div>
  </header>

  {#if tab === "catalog"}
    <Catalog {reloadKey} onopensettings={openRulesSettings} />
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

  .search {
    flex: 0 1 var(--search-width);
    margin-left: auto;
  }

  .corner {
    position: fixed;
    left: var(--space-4);
    bottom: var(--space-2);
  }
</style>
