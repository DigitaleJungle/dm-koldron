<script lang="ts" module>
  export type SectionId = "ui" | "campaigns" | "rules" | "library" | "integrations" | "addons" | "about";

  /** Counts every time any Settings dialog closes (packs may have been switched on or off there): screens showing
   *  pack content read it to load again, whichever gear opened Settings (theirs or the campaign sidebar's). */
  export const settingsClosed = $state({ count: 0 });

  /** How many Settings dialogs are open: an addon's campaign screen closes while one is, as its Library tab does, so
   *  its page and its Settings panels never save at the same time. */
  export const settingsOpen = $state({ count: 0 });
</script>

<script lang="ts">
  import { strings } from "$lib/strings";
  import { theme, chooseTheme, chooseAnimations } from "$lib/stores/theme.svelte";
  import type { Animations, Theme } from "$lib/api/settings";
  import CampaignSettings from "$lib/components/CampaignSettings.svelte";
  import AboutSettings from "$lib/components/AboutSettings.svelte";
  import AddonSettings from "$lib/components/AddonSettings.svelte";
  import RulesSettings from "$lib/components/RulesSettings.svelte";
  import AddonFrame from "$lib/components/AddonFrame.svelte";
  import { addons, loadAddons } from "$lib/stores/addons.svelte";
  import { panelIn } from "$lib/api/addons";
  import { libraryTabList, libraryTabs, loadLibraryTabs, setLibraryTabs } from "$lib/stores/libraryTabs.svelte";
  import { settlePanels, unsavedPanels } from "$lib/stores/settingsPanels.svelte";
  import { moveTo, ordered } from "$lib/components/sidebarOrder";
  import OrderList from "$lib/components/OrderList.svelte";

  let { open = $bindable(false), section }: { open?: boolean; section?: SectionId } = $props();
  let dialog: HTMLDialogElement | undefined = $state();
  $effect(() => {
    if (!open) return;
    settingsOpen.count++;
    return () => void settingsOpen.count--;
  });

  const t = strings.settings;
  loadAddons();
  // Library and Integrations are sections only when an addon adds a panel to them.
  const libraryPanels = $derived(addons.list.filter((a) => panelIn(a, "library")));
  const integrationPanels = $derived(addons.list.filter((a) => panelIn(a, "integrations")));
  const sections: { id: SectionId; label: string }[] = $derived([
    { id: "ui", label: t.ui.title },
    { id: "campaigns", label: t.campaigns.title },
    { id: "rules", label: t.rules.title },
    ...(libraryPanels.length ? [{ id: "library" as const, label: t.library.title }] : []),
    ...(integrationPanels.length ? [{ id: "integrations" as const, label: t.integrations.title }] : []),
    { id: "addons", label: t.addons.title },
    { id: "about", label: t.about.title },
  ]);
  let active = $state<SectionId>("ui");

  // Integrations shows each addon's panel as a tab, named by the panel's label (else the addon's name).
  const integrationTabs = $derived(
    integrationPanels.map((a) => ({ addon: a, label: a.settings.find((p) => p.section === "integrations")?.label ?? a.name })),
  );
  let integration = $state<string | null>(null);
  const currentIntegration = $derived(integrationTabs.some((tab) => tab.addon.id === integration) ? integration : (integrationTabs[0]?.addon.id ?? null));

  // Horizontal tabs: Left/Right (and Home/End) move between them; only the selected one is in the Tab order.
  function onIntegrationKeydown(event: KeyboardEvent) {
    const n = integrationTabs.length;
    const at = integrationTabs.findIndex((tab) => tab.addon.id === currentIntegration);
    const to = { ArrowRight: (at + 1) % n, ArrowLeft: (at - 1 + n) % n, Home: 0, End: n - 1 }[event.key];
    if (to === undefined) return;
    event.preventDefault();
    integration = integrationTabs[to].addon.id;
    document.getElementById(`integration-tab-${integration}`)?.focus();
  }

  // Vertical tabs: Up/Down (and Home/End) move between sections; only the selected tab is in the Tab order.
  function onTabKeydown(event: KeyboardEvent) {
    const at = sections.findIndex((s) => s.id === active);
    const to =
      event.key === "ArrowDown" ? (at + 1) % sections.length
      : event.key === "ArrowUp" ? (at - 1 + sections.length) % sections.length
      : event.key === "Home" ? 0
      : event.key === "End" ? sections.length - 1
      : -1;
    if (to === -1) return;
    event.preventDefault();
    active = sections[to].id;
    document.getElementById(`settings-tab-${active}`)?.focus();
  }

  const animationChoices: { value: Animations; label: string }[] = [
    { value: "system", label: t.ui.system },
    { value: "on", label: t.ui.animationsOn },
    { value: "off", label: t.ui.animationsOff },
  ];

  const themes: { value: Theme; label: string }[] = [
    { value: "system", label: t.ui.system },
    { value: "light", label: t.ui.light },
    { value: "dark", label: t.ui.dark },
  ];

  // ---- Changes wait for Save: the UI section's drafts, Rules' switches, and the addon panels' own. ----------------
  let themeDraft = $state<Theme>(theme.choice);
  let animationsDraft = $state<Animations>(theme.animations);
  // Library tabs: one order for the app-wide Library (and Rules/Catalog inside a campaign), moved as the sidebar's
  // screens. Null while unchanged.
  loadLibraryTabs();
  const tabs = $derived(libraryTabList());
  let tabsDraft = $state<string[] | null>(null);
  const shownTabs = $derived(tabsDraft ? ordered(tabs, tabsDraft) : tabs);
  let rules: RulesSettings | undefined = $state();
  let rulesDirty = $state(false);
  let addonSettings: AddonSettings | undefined = $state();
  let addonsDirty = $state(false);

  const dirty = $derived(
    themeDraft !== theme.choice || animationsDraft !== theme.animations || tabsDraft !== null || rulesDirty || addonsDirty || Object.values(unsavedPanels).some(Boolean),
  );
  let saving = $state(false);
  let errors = $state<string[]>([]);
  /** Closing with unsaved changes asks first, in place. */
  let confirmingClose = $state(false);

  function moveTab(from: number, to: number) {
    const ids = shownTabs.map((item) => item.id);
    const next = moveTo(ids, from, to);
    if (next === ids) return;
    tabsDraft = next.join() === tabs.map((item) => item.id).join() ? null : next;
  }

  function resetDrafts() {
    themeDraft = theme.choice;
    animationsDraft = theme.animations;
    tabsDraft = null;
  }

  /** Saves every section's changes; true when all of them were saved. Work they start (scans, downloads) follows. */
  async function save(): Promise<boolean> {
    saving = true;
    const failed: string[] = [];
    try {
      // Only this Save's own theme error: one left from an earlier save isn't about this one.
      const lookChanged = themeDraft !== theme.choice || animationsDraft !== theme.animations;
      if (themeDraft !== theme.choice) await chooseTheme(themeDraft);
      if (animationsDraft !== theme.animations) await chooseAnimations(animationsDraft);
      if (lookChanged && theme.error) failed.push(theme.error);
      if (tabsDraft) {
        await setLibraryTabs(tabsDraft);
        if (libraryTabs.error) failed.push(libraryTabs.error);
        else tabsDraft = null;
      }
      if (rulesDirty) await rules?.save().catch((e: Error) => failed.push(e.message));
      if (addonsDirty) await addonSettings?.save().catch((e: Error) => failed.push(e.message));
      failed.push(...(await settlePanels("save")));
    } finally {
      saving = false;
      errors = failed;
    }
    return failed.length === 0;
  }

  async function discard() {
    resetDrafts();
    rules?.discard();
    addonSettings?.discard();
    errors = await settlePanels("discard");
  }

  /** The ✕, Escape or a click beside the dialog: asks first when something isn't saved. */
  function requestClose() {
    if (saving) return;
    if (dirty) confirmingClose = true;
    else open = false;
  }

  async function saveAndClose() {
    if (await save()) open = false;
    else confirmingClose = false;
  }

  async function discardAndClose() {
    await discard();
    open = false;
  }

  // Native modal dialog: keeps focus inside, returns focus to the opener. Escape asks first, as the ✕ does.
  $effect(() => {
    if (open && !dialog?.open) {
      if (section) active = section;
      resetDrafts();
      errors = [];
      confirmingClose = false;
      dialog?.showModal();
    }
    if (!open && dialog?.open) dialog.close();
  });

  function onBackdropClick(event: MouseEvent) {
    if (event.target === dialog) requestClose();
  }
</script>

<dialog
  bind:this={dialog}
  class="dialog"
  aria-labelledby="settings-title"
  oncancel={(e) => {
    e.preventDefault();
    requestClose();
  }}
  onclose={() => {
    open = false;
    settingsClosed.count++;
  }}
  onclick={onBackdropClick}
>
  <header>
    <h2 id="settings-title">{t.title}</h2>
    <button class="icon-btn icon-btn-lg" aria-label={t.close} onclick={requestClose}>
      <i class="ti ti-x" aria-hidden="true"></i>
    </button>
  </header>

  <div class="body">
    <div class="side" role="tablist" aria-label={t.sections} aria-orientation="vertical">
      {#each sections as section (section.id)}
        <button
          id="settings-tab-{section.id}"
          class="tab"
          role="tab"
          aria-selected={active === section.id}
          aria-controls="settings-panel-{section.id}"
          tabindex={active === section.id ? 0 : -1}
          onclick={() => (active = section.id)}
          onkeydown={onTabKeydown}
        >
          {section.label}
        </button>
      {/each}
    </div>

    <!-- Every section stays loaded while Settings is open (only the chosen one shows), so changes not saved yet survive
         moving between sections. -->
    {#if open}
      <div id="settings-panel-ui" class="panel" role="tabpanel" aria-labelledby="settings-tab-ui" hidden={active !== "ui"}>
        <div class="field">
          <label for="theme-select">{t.ui.label}</label>
          <select id="theme-select" bind:value={themeDraft}>
            {#each themes as option (option.value)}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="animations-select">{t.ui.animationsLabel}</label>
          <select id="animations-select" bind:value={animationsDraft}>
            {#each animationChoices as option (option.value)}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <span class="field-name" id="library-tabs-label">{t.ui.libraryTabs}</span>
          <div class="order">
            <OrderList items={shownTabs} onmove={moveTab} aria-labelledby="library-tabs-label" aria-describedby="library-tabs-hint">
              {#snippet row(item)}<span>{item.label}</span>{/snippet}
            </OrderList>
          </div>
          <span class="hint" id="library-tabs-hint">{t.ui.libraryTabsHint}</span>
        </div>
      </div>
      <div id="settings-panel-campaigns" class="panel" role="tabpanel" aria-labelledby="settings-tab-campaigns" hidden={active !== "campaigns"}>
        <CampaignSettings />
      </div>
      <div id="settings-panel-rules" class="panel" role="tabpanel" aria-labelledby="settings-tab-rules" hidden={active !== "rules"}>
        <RulesSettings bind:this={rules} bind:dirty={rulesDirty} />
      </div>
      {#if libraryPanels.length}
        <div id="settings-panel-library" class="panel" role="tabpanel" aria-labelledby="settings-tab-library" hidden={active !== "library"}>
          {#each libraryPanels as addon (addon.id)}
            <AddonFrame {addon} entry={panelIn(addon, "library")!} panel="library" />
          {/each}
        </div>
      {/if}
      {#if integrationPanels.length}
        <div id="settings-panel-integrations" class="panel" role="tabpanel" aria-labelledby="settings-tab-integrations" hidden={active !== "integrations"}>
          <div class="integration-tabs" role="tablist" aria-label={t.integrations.tabs}>
            {#each integrationTabs as tab (tab.addon.id)}
              <button
                id="integration-tab-{tab.addon.id}"
                role="tab"
                aria-selected={currentIntegration === tab.addon.id}
                aria-controls="integration-panel-{tab.addon.id}"
                tabindex={currentIntegration === tab.addon.id ? 0 : -1}
                onclick={() => (integration = tab.addon.id)}
                onkeydown={onIntegrationKeydown}
              >
                {tab.label}
              </button>
            {/each}
          </div>
          <!-- Every tab's panel stays loaded (only the chosen one shows), so changes not saved yet survive switching. -->
          {#each integrationTabs as tab (tab.addon.id)}
            <div id="integration-panel-{tab.addon.id}" role="tabpanel" aria-labelledby="integration-tab-{tab.addon.id}" hidden={currentIntegration !== tab.addon.id}>
              <AddonFrame addon={tab.addon} entry={panelIn(tab.addon, "integrations")!} panel="integrations" />
            </div>
          {/each}
        </div>
      {/if}
      <div id="settings-panel-addons" class="panel" role="tabpanel" aria-labelledby="settings-tab-addons" hidden={active !== "addons"}>
        <AddonSettings bind:this={addonSettings} bind:dirty={addonsDirty} unsaved={dirty} />
      </div>
      <div id="settings-panel-about" class="panel" role="tabpanel" aria-labelledby="settings-tab-about" hidden={active !== "about"}>
        <AboutSettings />
      </div>
    {/if}
  </div>

  <footer>
    {#each errors as error, i (i)}<p class="error" role="alert">{error}</p>{/each}
    {#if confirmingClose}
      <div class="confirm" role="alertdialog" aria-labelledby="unsaved-question">
        <p id="unsaved-question">{t.unsaved}</p>
        <div class="actions">
          <button class="btn" onclick={() => (confirmingClose = false)}>{t.keepEditing}</button>
          <button class="btn" disabled={saving} onclick={discardAndClose}>{t.discard}</button>
          <!-- svelte-ignore a11y_autofocus: saving loses nothing -->
          <button class="btn btn-primary" autofocus disabled={saving} onclick={saveAndClose}>{t.save}</button>
        </div>
      </div>
    {:else}
      <div class="actions">
        <button class="btn" disabled={saving} onclick={requestClose}>{t.closeButton}</button>
        <button class="btn btn-primary" disabled={!dirty || saving} onclick={save}>{saving ? t.saving : t.save}</button>
      </div>
    {/if}
  </footer>
</dialog>

<style>
  dialog {
    width: min(var(--settings-width), calc(100vw - 2 * var(--space-4)));
    padding: 0;
    box-shadow: var(--shadow-drawer);
    overflow: hidden;
  }

  /* Fixed size, so switching sections doesn't make the dialog jump; only the section content scrolls. */
  dialog[open] {
    gap: 0;
    height: min(var(--settings-height), calc(100vh - 2 * var(--space-4)));
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-4) var(--space-4) var(--space-6);
    border-bottom: var(--border-width) solid var(--color-border);
  }

  h2 {
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
  }

  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .side {
    flex: 0 0 var(--settings-side-width);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3);
    background: var(--color-bg);
    border-right: var(--border-width) solid var(--color-border);
    border-bottom-left-radius: var(--radius-lg);
  }

  .tab {
    padding: var(--space-2) var(--space-3);
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    text-align: left;
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  .tab:hover {
    background: var(--color-surface-sunken);
  }

  .tab[aria-selected="true"] {
    background: var(--color-accent-soft);
    color: var(--color-link);
  }

  /* Every section is loaded while Settings is open; only the chosen one shows. */
  .panel[hidden] {
    display: none;
  }

  /* Integrations' tabs, one per addon panel, in the Library's tab look. */
  .integration-tabs {
    display: flex;
    flex-wrap: wrap;
    align-self: flex-start;
    gap: var(--space-1);
    padding: var(--space-1);
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
  }

  .integration-tabs button {
    padding: var(--space-1) var(--space-4);
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--color-text-muted);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  .integration-tabs button[aria-selected="true"] {
    background: var(--color-accent);
    color: var(--color-accent-text);
  }

  [role="tabpanel"][hidden] {
    display: none;
  }

  footer {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-6);
    border-top: var(--border-width) solid var(--color-border);
  }

  footer .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  footer .confirm {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .panel {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-6) var(--space-7);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .field label,
  .field-name {
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .hint {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  /* The list and its grips are OrderList's. */
  .order {
    width: min(100%, var(--select-width));
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .order :global(li) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-1) var(--space-1) var(--space-3);
    border-radius: var(--radius-md);
  }

  .order :global(li + li) {
    border-top: var(--border-width) solid var(--color-border);
  }

  select {
    width: min(100%, var(--select-width));
  }

  .error {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  @media (max-width: 720px) {
    .body {
      flex-direction: column;
    }

    .side {
      flex: none;
      flex-direction: row;
      border-right: none;
      border-bottom: var(--border-width) solid var(--color-border);
      border-bottom-left-radius: 0;
    }
  }
</style>
