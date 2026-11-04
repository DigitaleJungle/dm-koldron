<script lang="ts" module>
  export type SectionId = "ui" | "campaigns" | "rules" | "about";
</script>

<script lang="ts">
  import { strings } from "$lib/strings";
  import { theme, chooseTheme, chooseAnimations } from "$lib/stores/theme.svelte";
  import type { Animations, Theme } from "$lib/api/settings";
  import CampaignSettings from "$lib/components/CampaignSettings.svelte";
  import AboutSettings from "$lib/components/AboutSettings.svelte";
  import RulesSettings from "$lib/components/RulesSettings.svelte";

  let { open = $bindable(false), section }: { open?: boolean; section?: SectionId } = $props();
  let dialog: HTMLDialogElement | undefined = $state();

  const t = strings.settings;
  const sections: { id: SectionId; label: string }[] = [
    { id: "ui", label: t.ui.title },
    { id: "campaigns", label: t.campaigns.title },
    { id: "rules", label: t.rules.title },
    { id: "about", label: t.about.title },
  ];
  let active = $state<SectionId>("ui");

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

  // Native modal dialog: keeps focus inside, closes on Escape, returns focus to the opener.
  $effect(() => {
    if (open && !dialog?.open) {
      if (section) active = section;
      dialog?.showModal();
    }
    if (!open && dialog?.open) dialog.close();
  });

  function onBackdropClick(event: MouseEvent) {
    if (event.target === dialog) open = false;
  }
</script>

<dialog bind:this={dialog} aria-labelledby="settings-title" onclose={() => (open = false)} onclick={onBackdropClick}>
  <header>
    <h2 id="settings-title">{t.title}</h2>
    <button class="icon-btn icon-btn-lg" aria-label={t.close} onclick={() => (open = false)}>
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

    {#if active === "ui"}
      <div id="settings-panel-ui" class="panel" role="tabpanel" aria-labelledby="settings-tab-ui">
        <div class="field">
          <label for="theme-select">{t.ui.label}</label>
          <select id="theme-select" value={theme.choice} onchange={(e) => chooseTheme(e.currentTarget.value as Theme)}>
            {#each themes as option (option.value)}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="animations-select">{t.ui.animationsLabel}</label>
          <select id="animations-select" value={theme.animations} onchange={(e) => chooseAnimations(e.currentTarget.value as Animations)}>
            {#each animationChoices as option (option.value)}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
        </div>
        {#if theme.error}<p class="error" role="alert">{theme.error}</p>{/if}
      </div>
    {:else if active === "campaigns"}
      <div id="settings-panel-campaigns" class="panel" role="tabpanel" aria-labelledby="settings-tab-campaigns">
        <CampaignSettings />
      </div>
    {:else if active === "rules"}
      <div id="settings-panel-rules" class="panel" role="tabpanel" aria-labelledby="settings-tab-rules">
        <RulesSettings />
      </div>
    {:else if active === "about"}
      <div id="settings-panel-about" class="panel" role="tabpanel" aria-labelledby="settings-tab-about">
        <AboutSettings />
      </div>
    {/if}
  </div>
</dialog>

<style>
  dialog {
    width: min(var(--settings-width), calc(100vw - 2 * var(--space-4)));
    padding: 0;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: var(--shadow-drawer);
    overflow: hidden;
  }

  /* Fixed size, so switching sections doesn't make the dialog jump; only the section content scrolls. */
  dialog[open] {
    display: flex;
    flex-direction: column;
    height: min(var(--settings-height), calc(100vh - 2 * var(--space-4)));
  }

  dialog::backdrop {
    background: var(--color-backdrop);
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

  .field label {
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
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
