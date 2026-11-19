<script lang="ts">
  // The campaign screens' frame: a sidebar (app name and collapse button, campaign switcher, the screens, the
  // Settings gear) and the search bar above the screen. In a wide window the sidebar collapses to an icon strip
  // (remembered on this device); in a narrow one it's a drawer behind a menu button. While open, each screen's grip
  // (on hover or focus) moves it, by dragging or the arrow keys; one order for all campaigns, on this device.
  import type { Snippet } from "svelte";
  import { onMount } from "svelte";
  import { afterNavigate, goto } from "$app/navigation";
  import { page } from "$app/state";
  import type { Campaign } from "$lib/api/campaigns";
  import BackButton from "$lib/components/BackButton.svelte";
  import Menu from "$lib/components/Menu.svelte";
  import OrderList from "$lib/components/OrderList.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import SettingsButton from "$lib/components/SettingsButton.svelte";
  import { addons, loadAddons } from "$lib/stores/addons.svelte";
  import { campaigns, refreshCampaigns } from "$lib/stores/campaigns.svelte";
  import { header } from "$lib/stores/header.svelte";
  import { strings } from "$lib/strings";
  import { moveTo, ordered } from "./sidebarOrder";

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

  const ORDER_KEY = "campaign.sidebarOrder";
  let order = $state<string[]>(readOrder());
  function readOrder(): string[] {
    try {
      const saved: unknown = JSON.parse(localStorage.getItem(ORDER_KEY) ?? "[]");
      return Array.isArray(saved) ? saved.filter((x): x is string => typeof x === "string") : [];
    } catch {
      return []; // storage unavailable or damaged: the default order
    }
  }
  function setOrder(ids: string[]) {
    order = ids;
    try {
      localStorage.setItem(ORDER_KEY, JSON.stringify(ids));
    } catch {
      // not remembered; the order still changes until the app closes
    }
  }

  let drawerOpen = $state(false);
  let toggle: HTMLButtonElement | undefined = $state();
  let settingsOpen = $state(false);

  afterNavigate(() => (drawerOpen = false));
  onMount(() => {
    if (!campaigns.list) refreshCampaigns();
  });
  loadAddons();

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
  const links = $derived(
    ordered(
      [
        { id: "home", href: base, label: t.home, icon: "ti-home", current: page.url.pathname === base },
        { id: "library", href: `${base}/library`, label: t.library, icon: "ti-book", current: page.url.pathname.startsWith(`${base}/library`) },
        { id: "characters", href: `${base}/characters`, label: t.characters, icon: "ti-users", current: page.url.pathname.startsWith(`${base}/characters`) },
        { id: "encounters", href: `${base}/encounters`, label: t.encounters, icon: "ti-swords", current: page.url.pathname.startsWith(`${base}/encounters`) },
        { id: "sessions", href: `${base}/sessions`, label: t.sessions, icon: "ti-notebook", current: page.url.pathname.startsWith(`${base}/sessions`) },
        // Addons' campaign screens (their manifest's `campaign`), after the app's own until moved.
        ...addons.list
          .filter((a) => a.campaign)
          .map((a) => ({
            id: `addon:${a.id}`,
            href: `${base}/addon/${a.id}`,
            label: a.campaign!.label,
            icon: `ti-${a.campaign!.icon}`,
            current: page.url.pathname === `${base}/addon/${a.id}`,
          })),
      ],
      order,
    ),
  );

  function move(from: number, to: number) {
    const ids = links.map((l) => l.id);
    const next = moveTo(ids, from, to);
    if (next !== ids) setOrder(next);
  }
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
        <OrderList items={links} onmove={move}>
          {#snippet row(link)}
            <a href={link.href} aria-current={link.current ? "page" : undefined} aria-label={link.label}>
              <i class="ti {link.icon}" aria-hidden="true"></i>
              <span class="text">{link.label}</span>
            </a>
          {/snippet}
        </OrderList>
      </nav>
    </div>

    <div class="bottom">
      <SettingsButton bind:open={settingsOpen} />
    </div>
  </aside>

  <div class="main">
    <header class="top">
      <div class="lead">
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
        {#if header.back !== null}<BackButton fallback={header.back} />{/if}
        {@render header.lead?.()}
      </div>
      <div class="search"><SearchBar large /></div>
      <div class="action">{@render header.action?.()}</div>
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

  /* The list and its grips are OrderList's. */
  nav :global(ul) {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  nav :global(li) {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: var(--radius-md);
  }

  /* The grip shows on the row's hover or focus. */
  nav :global(.grip) {
    position: absolute;
    right: var(--space-1);
    opacity: 0;
  }

  nav :global(li:hover .grip),
  nav :global(li:focus-within .grip) {
    opacity: 1;
  }

  /* Not in the icon strip (the drawer shows it again, below). */
  .collapsed :global(.grip) {
    display: none;
  }

  nav a {
    flex: 1;
    min-width: 0;
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
    /* Menu and Back on the left, the screen's action on the right; the search bar centred on the header, shrinking first when there's no room. */
    display: grid;
    grid-template-columns: minmax(min-content, 1fr) minmax(0, calc(var(--search-width) * 1.5)) minmax(min-content, 1fr);
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-5);
    border-bottom: var(--border-width) solid var(--color-border);
    background: var(--color-surface);
  }

  .lead {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .action {
    justify-self: end;
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

    .collapsed aside .text,
    .collapsed :global(.grip) {
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
