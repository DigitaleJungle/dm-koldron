<script lang="ts">
  // An addon's screen in a campaign (its manifest's `campaign`): its page across the whole screen, below the campaign
  // header, told the campaign and its own place (`ui.navigate`, `?state=` in the address so Back and Forward reach
  // it). With `sidebar`, the header gets a button that hides the page's side column, remembered per addon on this
  // device. The page is left out while Settings is open, as an addon's Library tab is. Without the addon (removed or
  // disabled, or hidden in this campaign), the campaign's Home.
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import AddonFrame from "$lib/components/AddonFrame.svelte";
  import { settingsOpen } from "$lib/components/SettingsDialog.svelte";
  import { addons, campaignScreenShown, loadAddons } from "$lib/stores/addons.svelte";
  import { showHeaderLead } from "$lib/stores/header.svelte";
  import { gotoReplacing } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";

  let { campaign, addonId }: { campaign: string; addonId: string } = $props();

  const t = strings.library;
  let loaded = $state(false);
  loadAddons().then(() => (loaded = true));
  // Hidden in this campaign (`campaign.setVisible`) counts as not there.
  const addon = $derived(addons.list.find((a) => a.id === addonId && campaignScreenShown(a, campaign)));
  const base = $derived(`/campaign/${campaign}/addon/${addonId}`);
  $effect(() => {
    if (loaded && !addon) void gotoReplacing(`/campaign/${campaign}`);
  });

  function navigate(state: string | null, replace: boolean): Promise<void> {
    const address = state === null ? base : `${base}?${new URLSearchParams({ state })}`;
    const options = { keepFocus: true, noScroll: true };
    return replace ? gotoReplacing(address, options) : goto(address, options);
  }

  // The side column's button: remembered per addon on this device (storage can be unavailable: then it shows).
  const hiddenKey = $derived(`campaign.addonSidebarHidden.${addonId}`);
  let changed = $state<Record<string, boolean>>({});
  const sidebarHidden = $derived(changed[hiddenKey] ?? readHidden(hiddenKey));
  function readHidden(key: string) {
    try {
      return localStorage.getItem(key) === "1";
    } catch {
      return false;
    }
  }
  function setHidden(value: boolean) {
    changed[hiddenKey] = value;
    try {
      localStorage.setItem(hiddenKey, value ? "1" : "0");
    } catch {
      // not remembered, nothing else lost
    }
  }
  // The page can take the button away while it has no side column (`ui.setSidebarButton`); each opening starts with it.
  let sidebarButton = $state(true);
  $effect(() => {
    void addonId;
    sidebarButton = true;
  });
  showHeaderLead(() => (addon?.campaign?.sidebar && sidebarButton ? sidebarToggle : null));

  const context = $derived({
    campaign,
    state: page.url.searchParams.get("state"),
    ...(addon?.campaign?.sidebar ? { sidebarHidden } : {}),
  });
</script>

{#snippet sidebarToggle()}
  <button class="icon-btn icon-btn-lg" aria-expanded={!sidebarHidden} aria-label={sidebarHidden ? t.showSidebar : t.hideSidebar} onclick={() => setHidden(!sidebarHidden)}>
    <i class="ti {sidebarHidden ? 'ti-layout-sidebar-left-expand' : 'ti-layout-sidebar-left-collapse'}" aria-hidden="true"></i>
  </button>
{/snippet}

<svelte:head><title>{addon?.campaign?.label ?? ""}</title></svelte:head>

<!-- Keyed on the addon and campaign, so another addon's screen or campaign loads the page again; its own states don't. -->
{#if addon && settingsOpen.count === 0}
  {#key `${addon.id}/${campaign}`}
    <div class="screen">
      <AddonFrame
        {addon}
        entry={addon.campaign!.entry}
        {context}
        onnavigate={navigate}
        onsidebarbutton={addon.campaign!.sidebar ? (shown) => (sidebarButton = shown) : undefined}
      />
    </div>
  {/key}
{/if}

<style>
  /* The addon's page fills the screen and scrolls inside its own frame. */
  .screen {
    height: 100%;
  }

  .screen :global(iframe) {
    display: block;
    height: 100%;
  }
</style>
