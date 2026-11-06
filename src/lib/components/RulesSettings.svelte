<script lang="ts">
  import { strings } from "$lib/strings";
  import { cancelImageDownload, deleteImages, listPackImages, type PackImages } from "$lib/api/images";
  import { listPacks, type PackSummary } from "$lib/api/packs";
  import { getSettings, setSettings } from "$lib/api/settings";
  import { errorMessage } from "$lib/api/storage";
  import Switch from "$lib/components/Switch.svelte";
  import { imageDownloads, startImageDownload } from "$lib/stores/imageDownloads.svelte";

  const t = strings.settings.rules;
  const ti = t.images;
  const count = new Intl.NumberFormat();
  let packs = $state<PackSummary[]>();
  /** The packs that list images, by id. */
  let packImages = $state<Record<string, PackImages>>({});
  /** Pack ids with Enable images ticked. */
  let imagePacks = $state<string[]>([]);
  let loadError = $state<string>();
  let saveError = $state<string>();
  let saving = $state(false);

  const byPack = (images: PackImages[]) => Object.fromEntries(images.map((i) => [i.pack, i]));

  $effect(() => {
    Promise.all([listPacks(), listPackImages(), getSettings()])
      .then(([list, images, settings]) => {
        packImages = byPack(images);
        imagePacks = settings.imagePacks;
        packs = list;
      })
      .catch((e) => (loadError = errorMessage(e)));
  });

  /** Recounts the kept images: after a download ends or images are deleted. */
  function recount() {
    listPackImages()
      .then((images) => (packImages = byPack(images)))
      .catch((e) => (saveError = errorMessage(e)));
  }

  // A download ending (it leaves `imageDownloads`) changes the count.
  let running = 0;
  $effect(() => {
    const now = Object.keys(imageDownloads).length;
    if (now < running) recount();
    running = now;
  });

  async function toggle(pack: PackSummary, on: boolean) {
    saving = true;
    saveError = undefined;
    try {
      // From the saved list, so packs not loaded right now keep their setting.
      const saved = (await getSettings()).disabledPacks;
      await setSettings({ disabledPacks: on ? saved.filter((id) => id !== pack.id) : [...new Set([...saved, pack.id])] });
      pack.enabled = on;
    } catch (e) {
      saveError = t.saveError(pack.name, errorMessage(e));
    } finally {
      saving = false;
    }
  }

  // ---- Enable images: ticking asks Download all now / When browsing; unticking asks Keep / Delete. ----------------
  let asking = $state<{ pack: PackSummary; turningOn: boolean } | null>(null);
  let dialog: HTMLDialogElement | undefined = $state();

  $effect(() => {
    if (asking && dialog && !dialog.open) dialog.showModal();
  });

  async function answer(choice: "now" | "browsing" | "keep" | "delete" | "cancel") {
    const current = asking;
    asking = null;
    if (dialog?.open) dialog.close();
    if (!current || choice === "cancel") return;
    const { pack, turningOn } = current;
    saving = true;
    saveError = undefined;
    try {
      // From the saved list, so packs not loaded right now keep their setting.
      const saved = (await getSettings()).imagePacks;
      imagePacks = (await setSettings({ imagePacks: turningOn ? [...new Set([...saved, pack.id])] : saved.filter((id) => id !== pack.id) })).imagePacks;
      if (choice === "now") startImageDownload(pack.id, pack.name);
      if (!turningOn) await (choice === "delete" ? deleteImages(pack.id) : cancelImageDownload(pack.id));
      if (choice === "delete") recount();
    } catch (e) {
      saveError = ti.saveError(pack.name, errorMessage(e));
    } finally {
      saving = false;
    }
  }
</script>

{#if loadError}
  <p class="error" role="alert">{t.loadError(loadError)}</p>
{:else if !packs}
  <p class="muted" role="status">{t.loading}</p>
{:else if packs.length === 0}
  <p class="muted">{t.empty}</p>
{:else}
  {#if saveError}<p class="error" role="alert">{saveError}</p>{/if}
  <ul>
    {#each packs as pack (pack.id)}
      <li>
        {#if pack.errors.length === 0}
          <div class="row">
            <Switch id="pack-{pack.id}" label={pack.name} checked={pack.enabled} disabled={saving} onchange={(on) => toggle(pack, on)} />
            {#if packImages[pack.id]}
              {@const progress = imageDownloads[pack.id]}
              <label class="check">
                <!-- Stays as saved until the dialog is answered. Can't be ticked again while a cancelled download
                     is still finishing its last images, or the new download wouldn't start. -->
                <input
                  type="checkbox"
                  checked={imagePacks.includes(pack.id)}
                  disabled={saving || (!imagePacks.includes(pack.id) && !!imageDownloads[pack.id])}
                  onchange={(e) => {
                    asking = { pack, turningOn: e.currentTarget.checked };
                    e.currentTarget.checked = !e.currentTarget.checked;
                  }}
                />
                {ti.enable}
              </label>
              {#if progress}
                <div class="progress">
                  <progress max={progress.total || 1} value={progress.done + progress.failed} aria-label={ti.enable}></progress>
                  <span class="muted">{ti.progress(count.format(progress.done + progress.failed), count.format(progress.total))}</span>
                  <button class="btn small" onclick={() => cancelImageDownload(pack.id)}>{ti.cancelDownload}</button>
                </div>
              {:else if imagePacks.includes(pack.id) && packImages[pack.id].kept === packImages[pack.id].count}
                <span class="muted all-kept">{ti.allKept(count.format(packImages[pack.id].count))}</span>
              {/if}
            {/if}
          </div>
          <p class="muted">{t.details(pack.version, pack.license, pack.entries)}</p>
        {:else}
          <p class="name">{pack.name}</p>
          <p class="invalid">{t.invalid}</p>
          <ul class="errors">
            {#each pack.errors as error, i (i)}
              <li>{error}</li>
            {/each}
          </ul>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<dialog bind:this={dialog} class="ask" aria-labelledby="images-title" aria-describedby="images-question" onclose={() => asking && answer("cancel")}>
  {#if asking?.turningOn}
    {@const info = packImages[asking.pack.id]}
    <h2 id="images-title">{ti.enable}</h2>
    <p id="images-question">{ti.enableQuestion(asking.pack.name, count.format(info.count), ti.size(info.bytes))}</p>
    <div class="actions">
      <button class="btn" onclick={() => answer("cancel")}>{ti.cancel}</button>
      <button class="btn" onclick={() => answer("browsing")}>{ti.browsing}</button>
      <!-- svelte-ignore a11y_autofocus: downloading all is the suggested choice -->
      <button class="btn btn-primary" autofocus onclick={() => answer("now")}>{ti.now(ti.size(info.bytes))}</button>
    </div>
  {:else if asking}
    <h2 id="images-title">{ti.disableTitle}</h2>
    <p id="images-question">{ti.disableQuestion(asking.pack.name)}</p>
    <div class="actions">
      <button class="btn" onclick={() => answer("cancel")}>{ti.cancel}</button>
      <button class="btn" onclick={() => answer("delete")}>{ti.delete}</button>
      <!-- svelte-ignore a11y_autofocus: keeping loses nothing -->
      <button class="btn btn-primary" autofocus onclick={() => answer("keep")}>{ti.keep}</button>
    </div>
  {/if}
</dialog>

<style>
  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    list-style: none;
  }

  li {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  p {
    font-size: var(--text-sm);
  }

  .name {
    font-size: var(--text-md);
    font-weight: var(--weight-medium);
  }

  .muted {
    color: var(--color-text-muted);
  }

  /* The pack's switch, then its image checkbox and download progress on the same line (wrapping when narrow). */
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-6);
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .progress {
    flex: 1 1 var(--select-width);
    display: flex;
    align-items: center;
    gap: var(--space-3);
    font-size: var(--text-sm);
  }

  .progress progress {
    flex: 1;
    min-width: 0;
    accent-color: var(--color-accent);
  }

  .all-kept {
    font-size: var(--text-sm);
  }

  .small {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
  }

  .invalid {
    color: var(--color-danger);
    font-weight: var(--weight-medium);
  }

  .errors {
    gap: var(--space-1);
    padding-left: var(--space-4);
    list-style: disc;
    font-size: var(--text-sm);
    color: var(--color-text-muted);
  }

  .error {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
  }

  .ask {
    width: min(100% - 2 * var(--space-4), var(--dialog-width));
    padding: var(--space-5);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    color: var(--color-text);
  }

  .ask[open] {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .ask::backdrop {
    background: var(--color-backdrop);
  }

  .ask h2 {
    font-size: var(--text-lg);
  }

  .ask p {
    margin: 0;
    font-size: var(--text-md);
  }

  /* Equal buttons on one row; the primary one only differs in colour here. */
  .ask .actions {
    display: grid;
    grid-auto-columns: 1fr;
    grid-auto-flow: column;
    gap: var(--space-2);
  }

  .ask .actions .btn {
    min-height: var(--control-height-lg);
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-sm);
    text-align: center;
  }
</style>
