<script lang="ts">
  import { strings } from "$lib/strings";
  import { cancelImageDownload, deleteImages, listPackImages, type PackImages } from "$lib/api/images";
  import { listAddons, panelIn, type AddonSummary } from "$lib/api/addons";
  import { listPacks, type PackSummary } from "$lib/api/packs";
  import AddonFrame from "$lib/components/AddonFrame.svelte";
  import { getSettings, setSettings } from "$lib/api/settings";
  import { errorMessage } from "$lib/api/storage";
  import Switch from "$lib/components/Switch.svelte";
  import { applyPending } from "./rulesPending";
  import { imageDownloads, startImageDownload } from "$lib/stores/imageDownloads.svelte";
  import { forgetPictures } from "$lib/stores/monsterPictures";

  // Pack switches and Enable images wait for the Settings dialog's Save (`save`) or Cancel (`discard`); `dirty` says
  // there's something to save.
  let { dirty = $bindable(false) }: { dirty?: boolean } = $props();

  const t = strings.settings.rules;
  const ti = t.images;
  type ImageChoice = "now" | "browsing" | "keep" | "delete";
  /** Switches changed since the last save: pack id → on. */
  let pendingPacks = $state<Record<string, boolean>>({});
  /** Enable images answered since the last save: pack id → the answer. */
  let pendingImages = $state<Record<string, ImageChoice>>({});
  $effect(() => {
    dirty = Object.keys(pendingPacks).length + Object.keys(pendingImages).length > 0;
  });
  const packOn = (pack: PackSummary) => pendingPacks[pack.id] ?? pack.enabled;
  const imagesOn = (id: string) => (id in pendingImages ? pendingImages[id] === "now" || pendingImages[id] === "browsing" : imagePacks.includes(id));
  const count = new Intl.NumberFormat();
  let packs = $state<PackSummary[]>();
  /** The packs that list images, by id. */
  let packImages = $state<Record<string, PackImages>>({});
  /** Pack ids with Enable images ticked. */
  let imagePacks = $state<string[]>([]);
  let loadError = $state<string>();
  let saveError = $state<string>();
  let saving = $state(false);
  /** Loaded addons that add a panel to Rules. */
  let panels = $state<AddonSummary[]>([]);
  let addonsError = $state<string>();

  const byPack = (images: PackImages[]) => Object.fromEntries(images.map((i) => [i.pack, i]));

  $effect(() => {
    Promise.all([listPacks(), listPackImages(), getSettings()])
      .then(([list, images, settings]) => {
        packImages = byPack(images);
        imagePacks = settings.imagePacks;
        packs = list;
      })
      .catch((e) => (loadError = errorMessage(e)));
    listAddons()
      .then((list) => (panels = list.filter((a) => a.errors.length === 0 && panelIn(a, "rules"))))
      .catch((e) => (addonsError = errorMessage(e)));
  });

  /** An addon installed or removed a pack. */
  function packsChanged() {
    listPacks()
      .then((list) => (packs = list))
      .catch((e) => (loadError = errorMessage(e)));
    recount();
  }

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

  function toggle(pack: PackSummary, on: boolean) {
    if (on === pack.enabled) delete pendingPacks[pack.id];
    else pendingPacks[pack.id] = on;
  }

  /** The dialog's Save: the switches and Enable images answers, then the downloads or deletions they ask for. Throws
   *  what went wrong; what was saved stays saved. */
  export async function save() {
    if (!dirty) return;
    saving = true;
    saveError = undefined;
    try {
      // From the saved lists, so packs not loaded right now keep their setting.
      const imagesWanted = Object.fromEntries(Object.entries(pendingImages).map(([id, choice]) => [id, choice === "now" || choice === "browsing"]));
      imagePacks = (await setSettings(applyPending(await getSettings(), pendingPacks, imagesWanted))).imagePacks;
      for (const pack of packs ?? []) if (pack.id in pendingPacks) pack.enabled = pendingPacks[pack.id];
      pendingPacks = {};
      const answered = Object.entries(pendingImages);
      pendingImages = {};
      if (answered.length) forgetPictures(); // the encounter screens ask again: pictures on, or gone
      for (const [id, choice] of answered) {
        const name = packs?.find((p) => p.id === id)?.name ?? id;
        if (choice === "now") startImageDownload(id, name);
        if (choice === "keep") await cancelImageDownload(id);
        if (choice === "delete") await deleteImages(id);
      }
      if (answered.some(([, choice]) => choice === "delete")) recount();
    } catch (e) {
      saveError = t.saveFailed(errorMessage(e));
      throw new Error(saveError);
    } finally {
      saving = false;
    }
  }

  /** The dialog's Cancel: back to what's saved. */
  export function discard() {
    pendingPacks = {};
    pendingImages = {};
  }

  // ---- Enable images: ticking asks Download all now / When browsing; unticking asks Keep / Delete. ----------------
  let asking = $state<{ pack: PackSummary; turningOn: boolean } | null>(null);
  let dialog: HTMLDialogElement | undefined = $state();

  $effect(() => {
    if (asking && dialog && !dialog.open) dialog.showModal();
  });

  /** The answer waits for Save; turning back to what's saved forgets it. */
  function answer(choice: ImageChoice | "cancel") {
    const current = asking;
    asking = null;
    if (dialog?.open) dialog.close();
    if (!current || choice === "cancel") return;
    const { pack, turningOn } = current;
    if (turningOn === imagePacks.includes(pack.id)) delete pendingImages[pack.id];
    else pendingImages[pack.id] = choice;
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
            <Switch id="pack-{pack.id}" label={pack.name} checked={packOn(pack)} disabled={saving} onchange={(on) => toggle(pack, on)} />
            {#if packImages[pack.id]}
              {@const progress = imageDownloads[pack.id]}
              <label class="check">
                <!-- Stays as it was until the question is answered. Can't be ticked again while a cancelled download
                     is still finishing its last images, or the new download wouldn't start. -->
                <input
                  type="checkbox"
                  checked={imagesOn(pack.id)}
                  disabled={saving || (!imagesOn(pack.id) && !!imageDownloads[pack.id])}
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
              {:else if imagesOn(pack.id) && packImages[pack.id].kept === packImages[pack.id].count}
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

{#if addonsError}
  <p class="error" role="alert">{t.addonsError(addonsError)}</p>
{/if}
{#each panels as addon (addon.id)}
  <AddonFrame {addon} entry={panelIn(addon, "rules")!} panel="rules" onpackschange={packsChanged} />
{/each}

<dialog bind:this={dialog} class="dialog ask" aria-labelledby="images-title" aria-describedby="images-question" onclose={() => asking && answer("cancel")}>
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

  .ask p {
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
