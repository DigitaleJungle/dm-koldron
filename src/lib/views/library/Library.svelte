<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import BackButton from "$lib/components/BackButton.svelte";
  import { gotoReplacing } from "$lib/stores/history.svelte";
  import { showHeaderBack } from "$lib/stores/header.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import AddonFrame from "$lib/components/AddonFrame.svelte";
  import { addons, loadAddons } from "$lib/stores/addons.svelte";
  import { libraryTabList, libraryTabs, loadLibraryTabs } from "$lib/stores/libraryTabs.svelte";
  import SettingsDialog, { settingsClosed, type SectionId } from "$lib/components/SettingsDialog.svelte";
  import { libraryHome } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
  import "$lib/styles/prose.css";
  import Catalog from "./Catalog.svelte";
  import { pickTab } from "./catalog";
  import RulesReader from "./RulesReader.svelte";

  // `embedded`: inside a campaign's screens (/campaign/<id>/library), whose shell has the sidebar, the search bar and
  // Settings: then no search bar or gear of its own, and Back only on a Catalog entry (in the shell's header). Outside any campaign
  // (/library) it has all three on every page.
  let { embedded = false }: { embedded?: boolean } = $props();

  const t = strings.library;
  const base = $derived(embedded ? libraryHome() : "/library");
  const COLLAPSED_KEY = "library.contentsCollapsed";

  loadAddons();
  loadLibraryTabs();
  // In the order set in Settings → UI; addons' tabs in the app-wide Library only.
  const tabs = $derived(libraryTabList(!embedded));
  const tab = $derived(pickTab(tabs.map((item) => item.id), page.url.searchParams));
  const addonTab = $derived(embedded ? undefined : addons.list.find((a) => a.id === tab && a.library));
  function addonNavigate(state: string | null, replace: boolean): Promise<void> {
    const params = new URLSearchParams({ tab });
    if (state !== null) params.set("state", state);
    const options = { keepFocus: true, noScroll: true };
    return replace ? gotoReplacing(`${base}?${params}`, options) : goto(`${base}?${params}`, options);
  }
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

  // Rules' contents column, or an addon tab's side column (`library.sidebar`), hidden by the header's button.
  // Remembered per tab on this device only; storage can be unavailable, then the column shows.
  const collapseKey = $derived(tab === "rules" ? COLLAPSED_KEY : `library.sidebarHidden.${tab}`);
  let changed = $state<Record<string, boolean>>({});
  const collapsed = $derived(changed[collapseKey] ?? readCollapsed(collapseKey));
  function readCollapsed(key: string) {
    try {
      return localStorage.getItem(key) === "1";
    } catch {
      return false;
    }
  }
  function setCollapsed(value: boolean) {
    changed[collapseKey] = value;
    try {
      localStorage.setItem(collapseKey, value ? "1" : "0");
    } catch {
      // not remembered, nothing else lost
    }
  }
  // An addon tab with a side column; its page can take the button away while it has no column to show
  // (`ui.setSidebarButton`). Each time a tab opens, its page starts with the button.
  const sidebarTab = $derived(!!addonTab?.library?.sidebar);
  let sidebarButton = $state(true);
  $effect(() => {
    void tab;
    sidebarButton = true;
  });
  // The addon's own place in its tab (`ui.navigate`), `&state=` in the address, so Back and Forward reach it too. A
  // tab with a side column also hears whether the header's button hides it.
  const addonContext = $derived({ state: page.url.searchParams.get("state"), ...(sidebarTab ? { sidebarHidden: collapsed } : {}) });

  // The Catalog's list (not an entry page) has the Sources column.
  const catalogList = $derived(tab === "catalog" && !page.url.searchParams.has("entry"));
  // Outside a campaign the gear sits at the foot of the shown left column, else in the corner.
  const gearInContents = $derived(!embedded && ((tab === "rules" && rulesHasContents) || catalogList) && !collapsed);

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
    <!-- Inside a campaign the contents column is always shown (and there are no addon tabs). Outside, the button keeps
         its place even when a tab has none, so the tabs never move. -->
    {#if !embedded}<span class="column-slot">{#if (tab === "rules" && rulesHasContents) || catalogList || (sidebarTab && sidebarButton)}
      <button
        class="icon-btn icon-btn-lg"
        aria-expanded={!collapsed}
        aria-controls={sidebarTab ? undefined : catalogList ? "catalog-sources" : "library-contents"}
        aria-label={tab === "rules" ? (collapsed ? t.showContents : t.hideContents) : collapsed ? t.showSidebar : t.hideSidebar}
        onclick={() => setCollapsed(!collapsed)}
      >
        <i class="ti {collapsed ? 'ti-layout-sidebar-left-expand' : 'ti-layout-sidebar-left-collapse'}" aria-hidden="true"></i>
      </button>
    {/if}</span>{/if}
    <nav class="tabs" aria-label={t.tabs}>
      <!-- Switching tabs takes the current place in the history, so Back never steps between them. -->
      {#each tabs as item (item.id)}
        <a href="{base}?tab={item.id}" aria-current={tab === item.id ? "page" : undefined} onclick={switchTab}>{item.label}</a>
      {/each}
    </nav>
    {#if !embedded}<div class="search"><SearchBar /></div>{/if}
  </header>

  <!-- Until the saved order is read, so the Library doesn't open one tab and jump to another. -->
  {#if !libraryTabs.loaded}
    <!-- nothing yet: reading one small local file -->
  {:else if addonTab}
    <!-- Keyed on the addon only, so switching between addons' tabs loads the other page, and its own states don't. Left out while Settings is open: the addon's
         panel there may save, and this page must not save its older copy over it; closing Settings loads it fresh. -->
    {#key addonTab.id}
      <div class="addon-tab">{#if !settingsOpen}<AddonFrame addon={addonTab} entry={addonTab.library!.entry} context={addonContext} onnavigate={addonNavigate} onsidebarbutton={sidebarTab ? (shown) => (sidebarButton = shown) : undefined} />{/if}</div>
    {/key}
  {:else if tab === "catalog"}
    <Catalog {reloadKey} onopensettings={openRulesSettings} collapsed={collapsed && !embedded} bottom={gearInContents ? gear : undefined} />
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

  /* Always as wide as the column button, shown or not. */
  .column-slot {
    flex: none;
    display: flex;
    width: var(--control-height);
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

  /* The addon's page fills the pane and scrolls inside its own frame. */
  .addon-tab {
    flex: 1;
    min-height: 0;
  }

  .addon-tab :global(iframe) {
    display: block;
    height: 100%;
  }

  .corner {
    position: fixed;
    left: var(--space-4);
    bottom: var(--space-2);
  }
</style>
