<script lang="ts">
  import { onMount, tick } from "svelte";
  import { strings } from "$lib/strings";
  import { errorMessage } from "$lib/api/storage";
  import { pictureSrc, type Campaign } from "$lib/api/campaigns";
  import { campaignDialog, campaigns, createCampaign, deleteCampaign, refreshCampaigns, restoreCampaign } from "$lib/stores/campaigns.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import Menu, { type MenuItem } from "$lib/components/Menu.svelte";

  const t = strings.settings.campaigns;
  const c = strings.campaignSettings;

  let addName = $state("");
  let adding = $state(false);
  let addError = $state<string | null>(null);
  let deletingId = $state<string | null>(null);
  let busy = $state(false);
  let error = $state<{ id: string; text: string } | null>(null);

  onMount(refreshCampaigns);

  // Adding from Settings stays here (unlike the Start screen, which opens the new campaign).
  async function add(event: SubmitEvent) {
    event.preventDefault();
    adding = true;
    addError = null;
    try {
      await createCampaign(addName);
      addName = "";
    } catch (e) {
      addError = t.addError(errorMessage(e));
    } finally {
      adding = false;
    }
  }

  async function run(campaign: Campaign, failed: (name: string, reason: string) => string, action: () => Promise<void>) {
    busy = true;
    error = null;
    try {
      await action();
    } catch (e) {
      error = { id: campaign.id, text: failed(campaign.name, errorMessage(e)) };
    } finally {
      busy = false;
    }
  }

  async function focusMenu(id: string) {
    await tick();
    document.getElementById(`campaign-menu-${id}`)?.focus();
  }

  async function askDelete(campaign: Campaign) {
    error = null;
    deletingId = campaign.id;
    await tick();
    document.getElementById(`cancel-delete-${campaign.id}`)?.focus();
  }

  function cancelDelete(campaign: Campaign) {
    deletingId = null;
    focusMenu(campaign.id);
  }

  function onConfirmKeydown(event: KeyboardEvent, campaign: Campaign) {
    if (event.key !== "Escape") return;
    event.preventDefault(); // cancel the confirmation, not the dialog
    cancelDelete(campaign);
  }

  /** Restore, then say so; the name is read first, as the campaign moves lists. */
  const restore = (campaign: Campaign) =>
    run(campaign, c.restoreError, async () => {
      await restoreCampaign(campaign.id);
      notify(c.restored(campaign.name));
    });

  const confirmDelete = (campaign: Campaign) =>
    run(campaign, c.deleteError, async () => {
      await deleteCampaign(campaign.id);
      notify(c.deleted(campaign.name));
    });

  // An active campaign's settings are its own dialog; an archived one can only be restored or deleted.
  const archivedItems = (campaign: Campaign): MenuItem[] => [
    { label: c.restore, onselect: () => restore(campaign) },
    { label: c.delete, onselect: () => askDelete(campaign) },
  ];
</script>

{#snippet rows(list: Campaign[], archived: boolean)}
  <ul>
    {#each list as campaign (campaign.id)}
      <li>
        {#if deletingId === campaign.id}
          <div class="confirm" role="alertdialog" aria-labelledby="delete-text-{campaign.id}" tabindex="-1" onkeydown={(e) => onConfirmKeydown(e, campaign)}>
            <p id="delete-text-{campaign.id}">{c.confirmDelete(campaign.name)}</p>
            <div class="actions">
              <button id="cancel-delete-{campaign.id}" class="btn small" onclick={() => cancelDelete(campaign)}>{c.cancel}</button>
              <button class="btn btn-danger small" disabled={busy} onclick={() => confirmDelete(campaign)}>{c.confirm}</button>
            </div>
          </div>
        {:else}
          <div class="row">
            {#if campaign.picture}
              <img class="thumb" src={pictureSrc(campaign.picture)} alt="" />
            {:else}
              <span class="thumb" aria-hidden="true"></span>
            {/if}
            <span class="name" class:archived>{campaign.name}</span>
            {#if archived}
              <Menu id="campaign-menu-{campaign.id}" label={c.actionsFor(campaign.name)} items={archivedItems(campaign)} triggerClass="icon-btn">
                {#snippet trigger()}<i class="ti ti-dots" aria-hidden="true"></i>{/snippet}
              </Menu>
            {:else}
              <button class="icon-btn" aria-label={c.open(campaign.name)} onclick={() => (campaignDialog.id = campaign.id)}>
                <i class="ti ti-settings" aria-hidden="true"></i>
              </button>
            {/if}
          </div>
        {/if}
        {#if error?.id === campaign.id}<p class="error" role="alert">{error.text}</p>{/if}
      </li>
    {/each}
  </ul>
{/snippet}

<div class="campaign-settings">
  {#if campaigns.loadError}
    <p class="error" role="alert">{campaigns.loadError}</p>
  {:else if campaigns.list === null}
    <p class="muted" role="status">{strings.start.loading}</p>
  {:else}
    <section>
      {#if campaigns.list.campaigns.length === 0}
        <p class="muted">{t.noActive}</p>
      {:else}
        {@render rows(campaigns.list.campaigns, false)}
      {/if}
      <form class="add" onsubmit={add}>
        <label for="add-campaign" class="sr-only">{t.addLabel}</label>
        <input
          id="add-campaign"
          class="name-input"
          bind:value={addName}
          placeholder={t.addPlaceholder}
          maxlength="100"
          autocomplete="off"
          aria-describedby={addError ? "add-error" : undefined}
        />
        <button class="btn" type="submit" disabled={adding || !addName.trim()}>{t.add}</button>
      </form>
      {#if addError}<p id="add-error" class="error" role="alert">{addError}</p>{/if}
    </section>

    {#if campaigns.list.archived.length > 0}
      <section aria-labelledby="archived-title">
        <h4 id="archived-title" class="subheading">{t.archived}</h4>
        {@render rows(campaigns.list.archived, true)}
      </section>
    {/if}
  {/if}
</div>

<style>
  .campaign-settings {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .subheading {
    font-size: var(--text-md);
    font-weight: var(--weight-semibold);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
  }

  li {
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-3);
  }

  li + li {
    border-top: var(--border-width) solid var(--color-border);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .thumb {
    flex: none;
    width: var(--campaign-thumb-width);
    aspect-ratio: 16 / 9;
    object-fit: cover;
    border-radius: var(--radius-sm);
    background: var(--color-surface-sunken);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: var(--weight-medium);
  }

  .name.archived {
    color: var(--color-text-muted);
  }

  .confirm {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: var(--border-width) solid var(--color-danger);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .small {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
  }

  .add {
    display: flex;
    gap: var(--space-2);
  }

  .name-input {
    flex: 1;
    min-width: 0;
    min-height: var(--control-height);
    padding: 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .name-input::placeholder {
    color: var(--color-text-placeholder);
  }

  .muted {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .error {
    margin-top: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-size: var(--text-sm);
  }
</style>
