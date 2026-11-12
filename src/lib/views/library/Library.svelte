<script lang="ts">
  import { page } from "$app/state";
  import BackButton from "$lib/components/BackButton.svelte";
  import { gotoReplacing } from "$lib/stores/history.svelte";
  import { showHeaderBack } from "$lib/stores/header.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import SettingsDialog, { settingsClosed, type SectionId } from "$lib/components/SettingsDialog.svelte";
  import { libraryHome } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
  import "$lib/styles/prose.css";
  import Catalog from "./Catalog.svelte";
  import RulesReader from "./RulesReader.svelte";

  // `embedded`: inside a campaign's screens (/campaign/<id>/library), whose shell has the sidebar, the search bar and
  // Settings: then no search bar or gear of its own, and Back only on a Catalog entry (in the shell's header). Outside any campaign
  // (/library) it has all three on every page.
  let { embedded = false }: { embedded?: boolean } = $props();

  const t = strings.library;
  const base = $derived(embedded ? libraryHome() : "/library");
  const COLLAPSED_KEY = "library.contentsCollapsed";

  // ?tab=catalog, else the Rules tab.
  const tab = $derived(page.url.searchParams.get("tab") === "catalog" ? "catalog" : "rules");
  // Back goes back through history, like the mouse's back button (tabs and chapters take the current place, so they
  // aren't steps). Where there's nothing to go back to (the app was opened here): from a Catalog entry its list,
  // else the Start screen.
  const backTo = $derived.by(() => {
    const params = new URLSearchParams(page.url.searchParams);
    if (tab !== "catalog" || !params.has("entry")) return "/";
    params.delete("entry");
    params.delete("kind");
    return `${base}?${params}`;
  });
  // Inside a campaign, a Catalog entry's Back sits in the shell's header.
  showHeaderBack(() => (embedded && tab === "catalog" && page.url.searchParams.has("entry") ? backTo : null));
  function switchTab(event: MouseEvent & { currentTarget: HTMLAnchorElement }) {
    event.preventDefault();
    gotoReplacing(event.currentTarget.href);
  }
  let settingsOpen = $state(false);
  let settingsSection = $state<SectionId>();
  // Packs may have been switched on or off in Settings (whichever gear opened it), so the tabs load again when it closes.
  const reloadKey = $derived(settingsClosed.count);
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

  // Outside a campaign the gear sits at the foot of the shown contents column, else in the corner.
  const gearInContents = $derived(!embedded && tab === "rules" && rulesHasContents && !collapsed);

  // A hint opens Settings at Rules; once closed, the gear opens it where it was.
  $effect(() => {
    if (!settingsOpen) settingsSection = undefined;
  });

  function openRulesSettings() {
    settingsSection = "rules";
    settingsOpen = true;
  }
</script>

<svelte:head><title>{t.title}</title></svelte:head>

<div class="library" class:embedded>
  <header class="top">
    {#if !embedded}<BackButton fallback={backTo} />{/if}
    <!-- Inside a campaign the contents column is always shown. -->
    {#if !embedded && tab === "rules" && rulesHasContents}
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
      <!-- Switching tabs takes the current place in the history, so Back never steps between them. -->
      <a href={base} aria-current={tab === "rules" ? "page" : undefined} onclick={switchTab}>{t.rulesTab}</a>
      <a href="{base}?tab=catalog" aria-current={tab === "catalog" ? "page" : undefined} onclick={switchTab}>{strings.catalog.title}</a>
    </nav>
    {#if !embedded}<div class="search"><SearchBar /></div>{/if}
  </header>

  {#if tab === "catalog"}
    <Catalog {reloadKey} onopensettings={openRulesSettings} />
  {:else}
    <RulesReader collapsed={collapsed && !embedded} {reloadKey} onopensettings={openRulesSettings} bind:hasContents={rulesHasContents} bottom={gearInContents ? gear : undefined} />
  {/if}

  <!-- One dialog: the gear opens it where it was, the "turn a pack on" hints at Rules. Inside a campaign the shell has
       the gear; outside, it sits at the foot of the contents column while that's shown (clear of the chapters), else
       in the corner. -->
  <SettingsDialog bind:open={settingsOpen} section={settingsSection} />
  {#if !embedded && !gearInContents}<div class="corner">{@render gear()}</div>{/if}
</div>

{#snippet gear()}
  <button class="icon-btn icon-btn-lg" aria-label={strings.settings.open} aria-haspopup="dialog" onclick={() => (settingsOpen = true)}>
    <i class="ti ti-settings" aria-hidden="true"></i>
  </button>
{/snippet}

<style>
  .library {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  /* Inside the campaign shell: its screen's height, under the shell's search bar. */
  .library.embedded {
    height: 100%;
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
