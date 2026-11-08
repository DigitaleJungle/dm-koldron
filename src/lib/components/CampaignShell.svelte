<script lang="ts">
  // The campaign screens' frame: a sidebar (app name and collapse button, campaign switcher, the screens, the
  // Settings gear) and the search bar above the screen. In a wide window the sidebar collapses to an icon strip
  // (remembered on this device); in a narrow one it's a drawer behind a menu button.
  import type { Snippet } from "svelte";
  import { onMount } from "svelte";
  import { afterNavigate, goto } from "$app/navigation";
  import { page } from "$app/state";
  import type { Campaign } from "$lib/api/campaigns";
  import Menu from "$lib/components/Menu.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import SettingsButton from "$lib/components/SettingsButton.svelte";
  import { campaigns, refreshCampaigns } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";

  let { campaign, children }: { campaign: Campaign; children: Snippet } = $props();

  const t = strings.campaign;
  const COLLAPSED_KEY = "campaign.sidebarCollapsed";

  let collapsed = $state(readCollapsed());
  function readCollapsed() {
    try {
      return localStorage.getItem(COLLAPSED_KEY) === "1";
    } catch {
      return false; // storage unavailable: start expanded
    }
  }
  function setCollapsed(value: boolean) {
    collapsed = value;
    try {
      localStorage.setItem(COLLAPSED_KEY, value ? "1" : "0");
    } catch {
      // not remembered; the sidebar still collapses
    }
  }

  let drawerOpen = $state(false);
  let toggle: HTMLButtonElement | undefined = $state();
  let settingsOpen = $state(false);

  afterNavigate(() => (drawerOpen = false));
  onMount(() => {
    if (!campaigns.list) refreshCampaigns();
  });

  function closeDrawer() {
    drawerOpen = false;
    toggle?.focus();
  }

  function onkeydown(event: KeyboardEvent) {
    // While Settings is open, Escape belongs to it.
    if (drawerOpen && !settingsOpen && event.key === "Escape") closeDrawer();
  }

  // Until the list loads, the switcher offers the open campaign only.
  const switchItems = $derived(
    (campaigns.list?.campaigns ?? [campaign]).map((c) => ({ label: c.name, current: c.id === campaign.id, onselect: () => goto(`/campaign/${c.id}`) })),
  );

  const base = $derived(`/campaign/${campaign.id}`);
  const links = $derived([
    { href: base, label: t.home, icon: "ti-home", current: page.url.pathname === base },
    { href: `${base}/library`, label: t.library, icon: "ti-book", current: page.url.pathname.startsWith(`${base}/library`) },
    { href: `${base}/characters`, label: t.characters, icon: "ti-users", current: page.url.pathname.startsWith(`${base}/characters`) },
    { href: `${base}/encounters`, label: t.encounters, icon: "ti-swords", current: page.url.pathname.startsWith(`${base}/encounters`) },
  ]);
</script>

<svelte:window {onkeydown} />

<div class="shell" class:collapsed>
  {#if drawerOpen}
    <button class="backdrop" tabindex="-1" aria-label={t.closeMenu} onclick={closeDrawer}></button>
  {/if}

  <aside id="campaign-sidebar" class:open={drawerOpen}>
    <!-- The app's name, and the collapse button at the top (under it in the icon strip). -->
    <div class="head">
      <!-- In the icon strip the text is hidden, so the label names the link. -->
      <a class="app-name" href="/" aria-label={strings.appName}>
        <i class="ti ti-flask" aria-hidden="true"></i>
        <span class="text">{strings.appName}</span>
      </a>
      <button
        class="icon-btn collapse"
        aria-expanded={!collapsed}
        aria-controls="campaign-sidebar"
        aria-label={collapsed ? t.expand : t.collapse}
        onclick={() => setCollapsed(!collapsed)}
      >
        <i class="ti {collapsed ? 'ti-layout-sidebar-left-expand' : 'ti-layout-sidebar-left-collapse'}" aria-hidden="true"></i>
      </button>
    </div>

    <div class="switcher">
      <Menu id="campaign-switcher" label={t.switchCampaign(campaign.name)} items={switchItems} align="start" triggerClass="switch">
        {#snippet trigger()}
          <!-- In the icon strip only the first letter shows; the menu's label names the campaign. -->
          <span class="letter" aria-hidden="true">{campaign.name.charAt(0).toUpperCase()}</span>
          <span class="text">{campaign.name}</span>
          <i class="ti ti-selector chevron" aria-hidden="true"></i>
        {/snippet}
      </Menu>
    </div>

    <!-- The sidebar's items, between the switcher and the buttons at the bottom: a layout container only. -->
    <div class="items">
      <nav aria-label={t.navigation}>
        <ul>
          {#each links as link (link.href)}
            <li>
              <a href={link.href} aria-current={link.current ? "page" : undefined} aria-label={link.label}>
                <i class="ti {link.icon}" aria-hidden="true"></i>
                <span class="text">{link.label}</span>
              </a>
            </li>
          {/each}
        </ul>
      </nav>
    </div>

    <div class="bottom">
      <SettingsButton bind:open={settingsOpen} />
    </div>
  </aside>

  <div class="main">
    <header class="top">
      <button
        bind:this={toggle}
        class="icon-btn icon-btn-lg menu-toggle"
        aria-expanded={drawerOpen}
        aria-controls="campaign-sidebar"
        aria-label={drawerOpen ? t.closeMenu : t.openMenu}
        onclick={() => (drawerOpen = !drawerOpen)}
      >
        <i class="ti ti-menu-2" aria-hidden="true"></i>
      </button>
      <div class="search"><SearchBar /></div>
    </header>
    <div class="screen">{@render children()}</div>
  </div>
</div>

<style>
  /* Collapsing and expanding, the sidebar's column eases to its new width (instant with Animations off). */
  .shell {
    display: grid;
    grid-template-columns: var(--sidebar-width) minmax(0, 1fr);
    height: 100vh;
    transition: grid-template-columns var(--duration-drawer) ease-in-out;
  }

  :global([data-motion="reduce"]) .shell {
    transition: none;
  }

  /* While the column is narrower than its content, labels stay on one line (the sidebar clips them). */
  aside .text {
    white-space: nowrap;
  }

  .shell.collapsed {
    grid-template-columns: var(--sidebar-collapsed-width) minmax(0, 1fr);
  }

  aside {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-width: 0;
    padding: var(--space-3) var(--space-2);
    border-right: var(--border-width) solid var(--color-border);
    background: var(--color-surface);
    overflow-x: hidden; /* no scrollbar flashing while it widens */
    overflow-y: auto;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-1);
  }

  .app-name {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
    color: var(--color-text);
    font-weight: var(--weight-bold);
    text-decoration: none;
  }

  .switcher {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .switcher :global(.menu-root) {
    flex: 1;
    min-width: 0;
  }

  .switcher :global(.switch) {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    min-height: var(--control-height);
    padding: 0 var(--space-2) 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: var(--color-text);
    font-weight: var(--weight-medium);
    text-align: left;
    cursor: pointer;
  }

  .switcher .letter {
    display: none;
  }

  .switcher .text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  nav ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  nav a {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: var(--control-height);
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    color: var(--color-text);
    text-decoration: none;
  }

  nav a:hover {
    background: var(--color-surface-sunken);
  }

  nav a[aria-current="page"] {
    background: var(--color-accent);
    color: var(--color-accent-text);
  }


  .ti {
    font-size: var(--icon-size);
  }

  /* Takes the room between the switcher and the bottom buttons; nothing of it shows. */
  .items {
    flex: 1;
    min-height: 0;
  }

  .bottom {
    margin-top: auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  /* The icon strip: icons only, the switcher as the campaign's first letter and its arrows. The strip doesn't
     scroll, so the switcher's list can reach out over the screen instead of being cut at its edge. */
  .collapsed aside {
    position: relative;
    z-index: var(--z-menu); /* above the screen, for the switcher's list */
    align-items: center;
    overflow: visible;
  }

  .collapsed .text {
    display: none;
  }

  /* The letter as an accent avatar, the arrows small and muted beside it; no box around them. */
  .collapsed .switcher .letter {
    display: grid;
    place-items: center;
    width: var(--control-height-sm);
    height: var(--control-height-sm);
    border-radius: var(--radius-md);
    background: var(--color-accent);
    color: var(--color-accent-text);
    font-weight: var(--weight-bold);
  }

  .collapsed .switcher :global(.switch) {
    justify-content: center;
    gap: 2px;
    width: auto;
    padding: 0 var(--space-1);
    border-color: transparent;
    background: none;
  }

  .collapsed .switcher :global(.switch:hover) {
    background: var(--color-surface-sunken);
  }

  .collapsed .switcher .chevron {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .collapsed nav a {
    justify-content: center;
    padding: 0;
    width: var(--control-height);
  }

  .collapsed .head {
    flex-direction: column;
  }

  .collapsed .bottom {
    flex-direction: column;
    gap: var(--space-2);
  }

  .main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
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

  .search {
    width: min(100%, var(--search-width));
  }

  .menu-toggle,
  .backdrop {
    display: none;
  }

  .screen {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  /* A narrow window: the full sidebar as a drawer over the screen. */
  @media (max-width: 900px) {
    .shell,
    .shell.collapsed {
      grid-template-columns: minmax(0, 1fr);
    }

    aside {
      position: fixed;
      z-index: var(--z-drawer);
      inset: 0 auto 0 0;
      width: var(--drawer-width);
      align-items: stretch;
      box-shadow: var(--shadow-drawer);
      transform: translateX(-100%);
      visibility: hidden;
      transition: transform 200ms, visibility 200ms;
    }

    aside.open {
      transform: none;
      visibility: visible;
    }

    :global([data-motion="reduce"]) aside {
      transition: none;
    }

    .collapsed aside .text {
      display: revert;
    }

    .collapsed .switcher .letter {
      display: none;
    }

    .collapsed .switcher :global(.switch) {
      justify-content: space-between;
      gap: var(--space-2);
      width: 100%;
      padding: 0 var(--space-2) 0 var(--space-3);
      border-color: var(--color-border-strong);
      background: var(--color-surface);
    }

    .collapsed .switcher .chevron {
      color: inherit;
      font-size: inherit;
    }

    /* A drawer here, collapsed or not: it keeps its place, layer and scrolling. */
    .collapsed aside {
      position: fixed;
      z-index: var(--z-drawer);
      align-items: stretch;
      overflow-y: auto;
    }

    .collapsed nav a {
      justify-content: flex-start;
      padding: 0 var(--space-3);
      width: auto;
    }

    .collapse {
      display: none;
    }

    .menu-toggle {
      display: inline-flex;
    }

    .backdrop {
      display: block;
      position: fixed;
      z-index: var(--z-drawer);
      inset: 0;
      border: none;
      background: var(--color-backdrop);
    }
  }
</style>
