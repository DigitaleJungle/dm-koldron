<script lang="ts">
  import { tick } from "svelte";
  import { dev } from "$app/environment";
  import { strings } from "$lib/strings";
  import {
    discardStagedAddon,
    installStagedAddon,
    listAddons,
    restartApp,
    setAddonEnabled,
    stageAddon,
    uninstallAddon,
    type AddonSummary,
    type StagedAddon,
  } from "$lib/api/addons";
  import { errorMessage } from "$lib/api/storage";
  import Switch from "$lib/components/Switch.svelte";
  import { addonStatus, restartPending } from "$lib/components/addonStatus";
  import { addons as running } from "$lib/stores/addons.svelte";

  // Settings → Addons: the addons in the data folder, installing one from a .zip, enabling, disabling and
  // uninstalling. Changes wait for the dialog's Save (`save`) or are thrown away (`discard`); `dirty` says there's
  // something to save. Saved changes take effect after a restart, as addons are read once per run.
  // `unsaved`: the dialog has changes not saved yet, which a restart would lose.
  let { dirty = $bindable(false), unsaved }: { dirty?: boolean; unsaved: boolean } = $props();
  const t = strings.settings.addons;
  let addons = $state<AddonSummary[]>();
  let loadError = $state<string>();
  /** A picked .zip being confirmed. */
  let staged = $state<StagedAddon | null>(null);
  /** Waiting for Save: a confirmed install (still unzipped in the staging folder), switches changed (id → on), and
   *  uninstalls (id → also delete its data). */
  let pendingInstall = $state<StagedAddon | null>(null);
  let pendingEnabled = $state<Record<string, boolean>>({});
  let pendingUninstall = $state<Record<string, boolean>>({});
  $effect(() => {
    dirty = !!pendingInstall || Object.keys(pendingEnabled).length + Object.keys(pendingUninstall).length > 0;
  });
  /** The addon whose uninstall is being confirmed. */
  let removing = $state<string | null>(null);
  let deleteData = $state(false);
  let busy = $state(false);
  let error = $state<string>();
  let confirmButton: HTMLButtonElement | undefined = $state();
  let installButton: HTMLButtonElement | undefined = $state();

  const reload = () =>
    listAddons()
      .then((list) => (addons = list))
      .catch((e) => (loadError = errorMessage(e)));

  $effect(() => void reload());

  // Settings closed with a .zip unzipped but not installed: it goes.
  $effect(() => () => {
    if (staged || pendingInstall) discardStagedAddon().catch(() => {});
  });

  const permission = (p: string) => (p.startsWith("net:") ? t.net(p.slice(4)) : (t.permissions[p] ?? p));
  const enabledOf = (a: AddonSummary) => pendingEnabled[a.id] ?? a.enabled;

  const restartNeeded = $derived(addons?.some(restartPending) ?? false);

  function status(a: AddonSummary): string | null {
    const s = addonStatus(a, { enabled: pendingEnabled, uninstall: pendingUninstall });
    return s === "pendingUpdate" ? t.pendingUpdate(a.version) : s && t[s];
  }

  function toggle(a: AddonSummary, on: boolean) {
    if (on === a.enabled) delete pendingEnabled[a.id];
    else pendingEnabled[a.id] = on;
  }

  /** Back to the row's Uninstall (or Undo) button, or Install when the row has none. */
  async function focusRow(id: string) {
    await tick();
    (document.getElementById(`addon-uninstall-${id}`) ?? installButton)?.focus();
  }

  async function askUninstall(id: string) {
    removing = id;
    deleteData = false;
    await tick();
    document.getElementById(`addon-uninstall-cancel-${id}`)?.focus();
  }

  function cancelUninstall(id: string) {
    removing = null;
    focusRow(id);
  }

  function uninstall(id: string) {
    pendingUninstall[id] = deleteData;
    delete pendingEnabled[id];
    removing = null;
    focusRow(id);
  }

  function undoUninstall(id: string) {
    delete pendingUninstall[id];
    focusRow(id);
  }

  function onUninstallKeydown(event: KeyboardEvent, id: string) {
    if (event.key !== "Escape") return;
    event.preventDefault(); // cancel the uninstall, not the dialog
    cancelUninstall(id);
  }

  async function pick() {
    error = undefined;
    busy = true;
    try {
      // A new pick replaces the unzipped one, so it also replaces an install waiting for Save.
      const picked = await stageAddon();
      if (picked) {
        pendingInstall = null;
        staged = picked;
        await tick();
        confirmButton?.focus();
      }
    } catch (e) {
      error = t.installError(errorMessage(e));
      pendingInstall = null; // the pick emptied the unzipped folder that install was waiting in
    } finally {
      busy = false;
    }
  }

  async function confirm() {
    pendingInstall = staged;
    staged = null;
    await tick();
    installButton?.focus();
  }

  async function cancel() {
    staged = null;
    await tick();
    installButton?.focus();
    discardStagedAddon().catch(() => {}); // what's left is cleared by the next install
  }

  async function undoInstall() {
    pendingInstall = null;
    await tick();
    installButton?.focus();
    discardStagedAddon().catch(() => {});
  }

  function onConfirmKeydown(event: KeyboardEvent) {
    if (event.key !== "Escape") return;
    event.preventDefault(); // cancel the install, not the dialog
    cancel();
  }

  /** The dialog's Save: uninstalls, then switches, then the install. Throws what went wrong; what was saved stays
   *  saved, what failed stays waiting. */
  export async function save() {
    if (!dirty) return;
    busy = true;
    error = undefined;
    const failed: string[] = [];
    try {
      for (const [id, withData] of Object.entries(pendingUninstall)) {
        try {
          await uninstallAddon(id, withData);
          delete pendingUninstall[id];
          // Its pages are refused from now on: its tab, panels and background page go at once.
          running.list = running.list.filter((a) => a.id !== id);
        } catch (e) {
          failed.push(t.uninstallError(errorMessage(e)));
        }
      }
      for (const [id, on] of Object.entries(pendingEnabled)) {
        try {
          await setAddonEnabled(id, on);
          delete pendingEnabled[id];
        } catch (e) {
          failed.push(t.changeError(errorMessage(e)));
        }
      }
      if (pendingInstall) {
        try {
          await installStagedAddon();
          pendingInstall = null;
        } catch (e) {
          failed.push(t.installError(errorMessage(e)));
        }
      }
    } finally {
      busy = false;
      await reload();
    }
    if (failed.length) {
      error = failed.join(" ");
      throw new Error(error);
    }
  }

  /** Discarding the dialog's changes: back to what's saved. */
  export function discard() {
    pendingEnabled = {};
    pendingUninstall = {};
    if (pendingInstall) discardStagedAddon().catch(() => {});
    pendingInstall = null;
  }
</script>

<section>
  {#if loadError}
    <p class="error" role="alert">{t.loadError(loadError)}</p>
  {:else if addons?.length === 0 && !pendingInstall}
    <p class="muted">{t.none}</p>
  {:else if addons}
    <ul class="addons">
      {#each addons as addon (addon.id)}
        {@const note = status(addon)}
        {@const uninstalling = addon.id in pendingUninstall}
        {@const switchable = addon.errors.length === 0 && !addon.removed && !uninstalling}
        <li>
          {#if removing === addon.id}
            <div class="confirm danger" role="alertdialog" aria-labelledby="addon-uninstall-text-{addon.id}" tabindex="-1" onkeydown={(e) => onUninstallKeydown(e, addon.id)}>
              <p id="addon-uninstall-text-{addon.id}">{t.confirmUninstall(addon.name)}</p>
              <label class="check">
                <input type="checkbox" bind:checked={deleteData} />
                {t.deleteData}
              </label>
              <div class="actions">
                <button id="addon-uninstall-cancel-{addon.id}" class="btn small" onclick={() => cancelUninstall(addon.id)}>{t.cancel}</button>
                <button class="btn btn-danger small" onclick={() => uninstall(addon.id)}>{t.uninstall}</button>
              </div>
            </div>
          {:else}
            <div class="row">
              <div class="info" class:switchable>
                {#if switchable}
                  <Switch id="addon-{addon.id}" label={addon.name} checked={enabledOf(addon)} disabled={busy} onchange={(on) => toggle(addon, on)} />
                {:else}
                  <p class="name">{addon.name}</p>
                {/if}
                {#if addon.errors.length === 0}
                  <p class="muted">{enabledOf(addon) ? t.details(addon.version, addon.license) : `${t.details(addon.version, addon.license)} · ${t.disabled}`}</p>
                {:else}
                  <p class="invalid">{t.invalid}</p>
                  <ul class="errors">
                    {#each addon.errors as problem, i (i)}
                      <li>{problem}</li>
                    {/each}
                  </ul>
                {/if}
                {#if note}<p class="note">{note}</p>{/if}
              </div>
              {#if uninstalling}
                <button id="addon-uninstall-{addon.id}" class="btn small" disabled={busy} onclick={() => undoUninstall(addon.id)}>{t.undo}</button>
              {:else if !addon.removed}
                <button id="addon-uninstall-{addon.id}" class="icon-btn" aria-label={t.uninstallName(addon.name)} disabled={busy} onclick={() => askUninstall(addon.id)}>
                  <i class="ti ti-trash" aria-hidden="true"></i>
                </button>
              {/if}
            </div>
          {/if}
        </li>
      {/each}
      {#if pendingInstall}
        <li>
          <div class="row">
            <div class="info">
              <p class="name">{pendingInstall.name}</p>
              <p class="muted">{pendingInstall.version}</p>
              <p class="note">{pendingInstall.replaces === null ? t.onSaveInstall : t.onSaveUpdate}</p>
            </div>
            <button class="btn small" disabled={busy} onclick={undoInstall}>{t.undo}</button>
          </div>
        </li>
      {/if}
    </ul>
  {/if}
</section>

<section>
  {#if staged}
    <div class="confirm" role="alertdialog" aria-labelledby="addon-install-text" tabindex="-1" onkeydown={onConfirmKeydown}>
      <p id="addon-install-text" class="name">
        {staged.replaces === null ? t.confirmInstall(staged.name, staged.version) : t.confirmUpdate(staged.name, staged.replaces, staged.version)}
      </p>
      {#if staged.permissions.length}
        <p>{t.permissionsIntro}</p>
        <ul class="permissions">
          {#each staged.permissions as p (p)}<li>{permission(p)}</li>{/each}
        </ul>
      {:else}
        <p class="muted">{t.noPermissions}</p>
      {/if}
      <div class="actions">
        <button class="btn small" disabled={busy} onclick={cancel}>{t.cancel}</button>
        <button bind:this={confirmButton} class="btn btn-primary small" disabled={busy} onclick={confirm}>{t.confirm}</button>
      </div>
    </div>
  {:else}
    <div>
      <button bind:this={installButton} class="btn" disabled={busy} onclick={pick}>{t.install}</button>
    </div>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if restartNeeded && dev}
    <!-- A restart would leave `tauri dev`: it stops the dev server when this process quits, so the new one can't load. -->
    <p class="muted" role="status">{t.restartDev}</p>
  {:else if restartNeeded}
    <div>
      <button class="btn btn-primary" disabled={unsaved} aria-describedby={unsaved ? "addon-restart-unsaved" : undefined} onclick={() => restartApp()}>{t.restart}</button>
    </div>
    {#if unsaved}<p id="addon-restart-unsaved" class="muted">{t.restartUnsaved}</p>{/if}
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  p {
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
    overflow-wrap: anywhere;
  }

  .muted {
    color: var(--color-text-muted);
  }

  ul {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .addons > li {
    padding: var(--space-3) 0;
    border-bottom: var(--border-width) solid var(--color-border);
  }

  .addons > li:first-child {
    padding-top: 0;
  }

  .row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
  }

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  /* Under the switch's label, past the switch. */
  .switchable > p {
    padding-left: calc(2 * var(--control-height-sm) - 2 * var(--space-1) + var(--space-3));
  }

  .note {
    color: var(--color-accent);
  }

  .name {
    font-weight: var(--weight-medium);
  }

  .invalid {
    color: var(--color-danger);
    font-weight: var(--weight-medium);
  }

  .errors,
  .permissions {
    gap: var(--space-1);
    padding-left: var(--space-5);
    list-style: disc;
    font-size: var(--text-sm);
  }

  .errors {
    color: var(--color-text-muted);
  }

  .confirm {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
  }

  .confirm.danger {
    border-color: var(--color-danger);
    background: var(--color-danger-soft);
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
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

  .error {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
  }
</style>
