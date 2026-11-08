<script lang="ts">
  import { onMount, tick } from "svelte";
  import { fly } from "svelte/transition";
  import { goto } from "$app/navigation";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import SettingsButton from "$lib/components/SettingsButton.svelte";
  import { pictureSrc } from "$lib/api/campaigns";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns, createCampaign, refreshCampaigns } from "$lib/stores/campaigns.svelte";
  import { reduceMotion } from "$lib/stores/theme.svelte";
  import { strings } from "$lib/strings";
  import { version } from "../../../../package.json";

  const t = strings.start;
  const formatDate = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

  let adding = $state(false);
  let name = $state("");
  let creating = $state(false);
  let createError = $state<string | null>(null);

  const list = $derived(campaigns.list);

  onMount(() => {
    campaigns.scope = null; // out of any campaign: search everything
    void refreshCampaigns();
  });

  // The name field slides in from the right, from behind the + New campaign / Cancel button.
  const slide = () => ({ x: 24, duration: reduceMotion() ? 0 : 200 });

  async function startAdding() {
    adding = true;
    await tick();
    document.getElementById("new-campaign")?.focus();
  }

  function stopAdding() {
    adding = false;
    name = "";
    createError = null;
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    if (!name.trim() || creating) return;
    creating = true;
    createError = null;
    try {
      const campaign = await createCampaign(name);
      await goto(`/campaign/${campaign.id}`);
    } catch (e) {
      createError = t.createError(errorMessage(e));
      creating = false;
    }
  }
</script>

<div class="start">
  <nav class="top-links">
    <a class="btn" href="/library">
      <i class="ti ti-book" aria-hidden="true"></i>
      {t.library}
    </a>
  </nav>

  <main>
    <h1>{strings.appName}</h1>

    <div class="search"><SearchBar large /></div>

    <section aria-labelledby="campaigns-title">
      <div class="section-head">
        <h2 id="campaigns-title" class="sr-only">{t.campaigns}</h2>
        <div class="head-actions">
          {#if adding}
            <form class="create" onsubmit={create} transition:fly={slide()}>
              <label for="new-campaign" class="sr-only">{t.newCampaign}</label>
              <input
                id="new-campaign"
                bind:value={name}
                placeholder={t.namePlaceholder}
                maxlength="100"
                autocomplete="off"
                onkeydown={(e) => e.key === "Escape" && stopAdding()}
                aria-describedby={createError ? "create-error" : undefined}
              />
              <button class="btn btn-primary small" type="submit" disabled={!name.trim() || creating}>
                {creating ? t.creating : t.create}
              </button>
            </form>
          {/if}
          <button class="btn small" onclick={() => (adding ? stopAdding() : startAdding())}>
            {adding ? t.cancel : t.addCampaign}
          </button>
        </div>
      </div>
      {#if adding && createError}<p id="create-error" class="error" role="alert">{createError}</p>{/if}

      {#if campaigns.loadError}
        <p class="error" role="alert">{campaigns.loadError}</p>
      {:else if list === null}
        <p class="muted" role="status">{t.loading}</p>
      {:else}
        {#if list.campaigns.length === 0}
          <p class="muted">{t.empty}</p>
        {:else}
          <!-- Most recently opened first (the backend sorts them). -->
          <ul class="campaigns">
            {#each list.campaigns as campaign (campaign.id)}
              <li>
                <a class="tile" class:pictured={!!campaign.picture} href="/campaign/{campaign.id}">
                  {#if campaign.picture}<img src={pictureSrc(campaign.picture)} alt="" />{/if}
                  <span class="caption">
                    <span class="name">{campaign.name}</span>
                    <span class="when">{t.lastOpened(formatDate.format(campaign.lastOpenedAt))}</span>
                  </span>
                </a>
              </li>
            {/each}
          </ul>
        {/if}
        {#if list.unreadable.length > 0}
          <p class="error" role="alert">{t.unreadable(list.unreadable.join(", "))}</p>
        {/if}
      {/if}
    </section>
  </main>

  <footer>
    <SettingsButton />
    <p class="version">{t.version(version)}</p>
  </footer>
</div>

<style>
  .start {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  .top-links {
    display: flex;
    justify-content: flex-end;
    padding: var(--space-3) var(--space-6);
  }

  .top-links .btn {
    color: var(--color-text);
    text-decoration: none;
  }

  main {
    flex: 1;
    width: 100%;
    max-width: var(--content-max-width);
    margin: 0 auto;
    padding: var(--space-1) var(--space-7) var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  h1 {
    text-align: center;
    font-size: var(--text-2xl);
    font-weight: var(--weight-bold);
    letter-spacing: var(--tracking-tight);
  }

  .search {
    width: min(100%, var(--start-width));
    margin: 0 auto;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: var(--control-height-sm);
  }


  .head-actions {
    flex: 1;
    min-width: 0;
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    padding-left: var(--space-3);
    overflow: hidden;
  }

  .small {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
    font-size: var(--text-sm);
    font-weight: var(--weight-semibold);
  }

  .create {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: var(--space-2);
  }

  .create input {
    flex: 1;
    min-width: 0;
    min-height: var(--control-height-sm);
    font-size: var(--text-sm);
    padding: 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  /* The field slides in already focused; its caret shows where typing goes. */
  .create input:focus-visible {
    outline: none;
  }

  .create input::placeholder {
    color: var(--color-text-muted);
  }

  .campaigns {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--tile-min-width), 1fr));
    gap: var(--space-4);
  }

  /* A 16:9 tile; with a picture, its name and date show over it on hover or focus. Without one they always show. */
  .tile {
    position: relative;
    aspect-ratio: 16 / 9;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
    color: var(--color-text);
    text-decoration: none;
    overflow: hidden;
  }

  .tile:focus-visible {
    outline-offset: 2px;
  }

  .tile img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .caption {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3);
  }

  .pictured .caption {
    position: absolute;
    inset: 0;
    justify-content: flex-end;
    background: var(--color-overlay);
    color: var(--color-overlay-text);
    opacity: 0;
    transition: opacity var(--duration-hover) var(--ease-hover);
  }

  .pictured:hover .caption,
  .pictured:focus-visible .caption {
    opacity: 1;
  }

  :global([data-motion="reduce"]) .pictured .caption {
    transition: none;
  }

  .name {
    font-weight: var(--weight-semibold);
    overflow-wrap: anywhere;
  }

  .when {
    font-size: var(--text-xs);
  }

  .tile:not(.pictured) .when {
    color: var(--color-text-muted);
  }

  .muted,
  .version {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .error {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  /* The gear in the corner, the version centred. */
  footer {
    display: grid;
    grid-template-columns: var(--control-height) 1fr var(--control-height);
    align-items: center;
    padding: var(--space-1) var(--space-4) var(--space-2);
  }

  .version {
    text-align: center;
    font-size: var(--text-xs);
  }

  @media (max-width: 900px) {
    main {
      padding: var(--space-1) var(--space-4) var(--space-3);
    }
  }
</style>
