<script lang="ts">
  // One campaign's settings: name and rules wait for Save (as in Settings); picture, archive and delete are done when
  // pressed. Opened by `campaignDialog.id` from the sidebar's switcher, a Start tile or Settings → Campaigns.
  import { tick } from "svelte";
  import { strings } from "$lib/strings";
  import { errorMessage } from "$lib/api/storage";
  import { pictureSrc } from "$lib/api/campaigns";
  import { listPacks, type PackSummary } from "$lib/api/packs";
  import {
    archiveCampaign,
    campaignDialog,
    campaigns,
    deleteCampaign,
    refreshCampaigns,
    renameCampaign,
    setCampaignPacks,
    setCampaignPicture,
  } from "$lib/stores/campaigns.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import { settingsClosed } from "$lib/components/SettingsDialog.svelte";
  import Switch from "$lib/components/Switch.svelte";

  const t = strings.settings;
  const c = strings.campaignSettings;

  let dialog: HTMLDialogElement | undefined = $state();
  const campaign = $derived(
    campaigns.list?.campaigns.find((x) => x.id === campaignDialog.id) ?? (campaigns.current?.id === campaignDialog.id ? campaigns.current : null),
  );

  let nameDraft = $state("");
  let offDraft = $state<string[]>([]);
  /** The packs switched on in Settings → Rules; null while loading. */
  let packs = $state<PackSummary[] | null>(null);
  let packsError = $state<string | null>(null);
  let saving = $state(false);
  let busy = $state(false);
  let errors = $state<string[]>([]);
  let confirmingClose = $state(false);
  let confirmingDelete = $state(false);
  /** Rules were saved while open: screens showing pack content load again on close. */
  let packsSaved = false;

  const savedOff = $derived(campaign?.packsOff ?? []);
  const sameOff = (a: string[], b: string[]) => a.length === b.length && a.every((id) => b.includes(id));
  const nameDirty = $derived(!!campaign && nameDraft.trim() !== campaign.name);
  const packsDirty = $derived(!sameOff(offDraft, savedOff));
  const dirty = $derived(nameDirty || packsDirty);

  // Opening (a new id): fresh drafts and the packs as they are now.
  $effect(() => {
    const id = campaignDialog.id;
    if (!id) return void (dialog?.open && dialog.close());
    if (dialog?.open) return;
    if (!campaigns.list) refreshCampaigns();
    nameDraft = campaign?.name ?? "";
    offDraft = [...(campaign?.packsOff ?? [])];
    [errors, confirmingClose, confirmingDelete, packs, packsError, packsSaved] = [[], false, false, null, null, false];
    listPacks().then(
      (all) => campaignDialog.id === id && (packs = all.filter((p) => p.enabled && p.errors.length === 0)),
      (e) => campaignDialog.id === id && (packsError = c.rulesError(errorMessage(e))),
    );
    dialog?.showModal();
  });

  // The list may load after opening: take the name then.
  $effect(() => {
    if (campaign && !nameDraft && !nameDirty) nameDraft = campaign.name;
  });

  const togglePack = (id: string, on: boolean) => (offDraft = on ? offDraft.filter((x) => x !== id) : [...offDraft, id]);

  async function save(): Promise<boolean> {
    if (!campaign) return false;
    saving = true;
    const failed: string[] = [];
    try {
      if (nameDirty) await renameCampaign(campaign.id, nameDraft).catch((e) => failed.push(c.renameError(campaign.name, errorMessage(e))));
      if (packsDirty) {
        await setCampaignPacks(campaign.id, offDraft).then(
          () => (packsSaved = true),
          (e) => failed.push(c.packsError(errorMessage(e))),
        );
      }
    } finally {
      saving = false;
      errors = failed;
    }
    return failed.length === 0;
  }

  function requestClose() {
    if (saving || busy) return;
    if (dirty) confirmingClose = true;
    else campaignDialog.id = null;
  }

  async function saveAndClose() {
    if (await save()) campaignDialog.id = null;
    else confirmingClose = false;
  }

  /** An action done when pressed; `closes` when the campaign leaves the active list. */
  async function run(action: () => Promise<void>, failed: (reason: string) => string, closes = false) {
    busy = true;
    errors = [];
    try {
      await action();
      if (closes) campaignDialog.id = null;
    } catch (e) {
      errors = [failed(errorMessage(e))];
    } finally {
      busy = false;
    }
  }

  const picture = (choose: boolean) => campaign && run(() => setCampaignPicture(campaign.id, choose), c.pictureError);

  function archive() {
    if (!campaign) return;
    const { id, name } = campaign;
    run(async () => (await archiveCampaign(id), notify(c.archived(name))), (r) => c.archiveError(name, r), true);
  }

  async function askDelete() {
    confirmingDelete = true;
    await tick();
    document.getElementById("campaign-cancel-delete")?.focus();
  }

  function confirmDelete() {
    if (!campaign) return;
    const { id, name } = campaign;
    run(async () => (await deleteCampaign(id), notify(c.deleted(name))), (r) => c.deleteError(name, r), true);
  }

  function onBackdropClick(event: MouseEvent) {
    if (event.target === dialog) requestClose();
  }
</script>

<dialog
  bind:this={dialog}
  class="dialog"
  aria-labelledby="campaign-dialog-title"
  oncancel={(e) => {
    e.preventDefault();
    if (confirmingDelete) confirmingDelete = false;
    else requestClose();
  }}
  onclose={() => {
    campaignDialog.id = null;
    if (packsSaved) settingsClosed.count++;
  }}
  onclick={onBackdropClick}
>
  <header>
    <h2 id="campaign-dialog-title">{campaign?.name ?? ""}</h2>
    <button class="icon-btn icon-btn-lg" aria-label={c.close} onclick={requestClose}>
      <i class="ti ti-x" aria-hidden="true"></i>
    </button>
  </header>

  {#if campaignDialog.id}
    <div class="body">
      <div class="field">
        <label for="campaign-dialog-name">{c.name}</label>
        <input id="campaign-dialog-name" class="text-input" bind:value={nameDraft} maxlength="100" autocomplete="off" required />
      </div>

      <div class="field">
        <span class="field-name">{c.picture}</span>
        <div class="picture">
          {#if campaign?.picture}
            <img class="thumb" src={pictureSrc(campaign.picture)} alt="" />
          {:else}
            <span class="thumb" aria-hidden="true"></span>
          {/if}
          <button class="btn small" disabled={busy || !campaign} onclick={() => picture(true)}>{campaign?.picture ? c.changePicture : c.choosePicture}</button>
          {#if campaign?.picture}<button class="btn small" disabled={busy} onclick={() => picture(false)}>{c.removePicture}</button>{/if}
        </div>
      </div>

      <div class="field">
        <span class="field-name" id="campaign-dialog-rules">{c.rules}</span>
        {#if packsError}
          <p class="error" role="alert">{packsError}</p>
        {:else if packs === null}
          <p class="muted" role="status">{t.rules.loading}</p>
        {:else if packs.length === 0}
          <p class="muted">{c.noRules}</p>
        {:else}
          <ul class="packs" aria-labelledby="campaign-dialog-rules">
            {#each packs as pack (pack.id)}
              <li>
                <Switch id="campaign-pack-{pack.id}" label={pack.name} checked={!offDraft.includes(pack.id)} onchange={(on) => togglePack(pack.id, on)} />
                <span class="muted">{pack.version}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      {#if confirmingDelete && campaign}
        <div class="confirm-delete" role="alertdialog" aria-labelledby="campaign-delete-text">
          <p id="campaign-delete-text">{c.confirmDelete(campaign.name)}</p>
          <div class="actions">
            <button id="campaign-cancel-delete" class="btn small" onclick={() => (confirmingDelete = false)}>{c.cancel}</button>
            <button class="btn btn-danger small" disabled={busy} onclick={confirmDelete}>{c.confirm}</button>
          </div>
        </div>
      {:else}
        <div class="danger">
          <button class="btn small" disabled={busy || !campaign} onclick={archive}>{c.archive}</button>
          <button class="btn btn-danger small" disabled={busy || !campaign} onclick={askDelete}>{c.delete}</button>
        </div>
      {/if}
    </div>
  {/if}

  <footer>
    {#each errors as error, i (i)}<p class="error" role="alert">{error}</p>{/each}
    {#if confirmingClose}
      <div class="confirm" role="alertdialog" aria-labelledby="campaign-unsaved-question">
        <p id="campaign-unsaved-question">{t.unsaved}</p>
        <div class="actions">
          <button class="btn" onclick={() => (confirmingClose = false)}>{t.keepEditing}</button>
          <button class="btn" disabled={saving} onclick={() => (campaignDialog.id = null)}>{t.discard}</button>
          <!-- svelte-ignore a11y_autofocus: saving loses nothing -->
          <button class="btn btn-primary" autofocus disabled={saving} onclick={saveAndClose}>{t.save}</button>
        </div>
      </div>
    {:else}
      <div class="actions">
        <button class="btn" disabled={saving} onclick={requestClose}>{t.closeButton}</button>
        <button class="btn btn-primary" disabled={!dirty || !nameDraft.trim() || saving} onclick={save}>{saving ? t.saving : t.save}</button>
      </div>
    {/if}
  </footer>
</dialog>

<style>
  dialog {
    width: min(var(--campaign-dialog-width), calc(100vw - 2 * var(--space-4)));
    padding: 0;
    box-shadow: var(--shadow-drawer);
    overflow: hidden;
  }

  dialog[open] {
    gap: 0;
    max-height: calc(100vh - 2 * var(--space-4));
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
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    padding: var(--space-6);
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

  .text-input {
    min-height: var(--control-height);
    padding: 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .picture {
    display: flex;
    flex-wrap: wrap;
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

  .packs {
    list-style: none;
    margin: 0;
    padding: 0;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .packs li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
  }

  .packs li + li {
    border-top: var(--border-width) solid var(--color-border);
  }

  .danger {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    padding-top: var(--space-4);
    border-top: var(--border-width) solid var(--color-border);
  }

  .confirm-delete {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: var(--border-width) solid var(--color-danger);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    font-size: var(--text-sm);
  }

  .small {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
  }

  footer {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-6);
    border-top: var(--border-width) solid var(--color-border);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .confirm {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .muted {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .error {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-size: var(--text-sm);
  }
</style>
