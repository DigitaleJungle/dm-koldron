<script lang="ts">
  import { strings } from "$lib/strings";
  import { cancelImageDownload, deleteImages, listPackImages, type PackImages } from "$lib/api/images";
  import { listAddons, panelIn, type AddonSummary } from "$lib/api/addons";
  import { checkSrdUpdate, listPacks, removePack, updateSrd, type PackSummary } from "$lib/api/packs";
  import AddonFrame from "$lib/components/AddonFrame.svelte";
  import { getSettings, setSettings } from "$lib/api/settings";
  import { errorMessage } from "$lib/api/storage";
  import Switch from "$lib/components/Switch.svelte";
  import { applyPending } from "./rulesPending";
  import { imageDownloads, startImageDownload } from "$lib/stores/imageDownloads.svelte";
  import { packProgress, type PackProgress } from "$lib/stores/packProgress.svelte";
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

  /** Packs an addon reports work on that aren't installed yet (a first import): a row each until they are. */
  const coming = $derived(Object.entries(packProgress).filter(([id]) => !packs?.some((p) => p.id === id)));

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
      .then((list) => (panels = list.filter((a) => a.loadedVersion !== null && !a.removed && panelIn(a, "rules"))))
      .catch((e) => (addonsError = errorMessage(e)));
  });

  /** An addon installed or removed a pack. */
  function packsChanged() {
    listPacks()
      .then((list) => (packs = list))
      .catch((e) => (loadError = errorMessage(e)));
    recount();
  }

  // ---- Removing an installed pack: its row asks first, then it goes at once (not on Save). ----------------------
  /** The pack whose row asks "Remove … and its images?". */
  let removing = $state<string | null>(null);

  /** The question takes the trash button's place, so focus moves to its Cancel. */
  const focus = (node: HTMLElement) => node.focus();

  async function remove(pack: PackSummary) {
    saveError = undefined;
    try {
      await removePack(pack.id);
      delete pendingPacks[pack.id];
      delete pendingImages[pack.id];
      forgetPictures();
      packsChanged();
    } catch (e) {
      saveError = t.removeError(errorMessage(e));
    } finally {
      removing = null;
    }
  }

  // ---- Update rules: the SRD pack from the newest 5e-database release; actions, not waiting for Save. ------------
  const tu = t.update;
  const SRD = "srd";
  let srdBusy = $state<"checking" | "updating" | null>(null);
  /** The newer release Check for updates found, which Update installs. */
  let srdLatest = $state<string | null>(null);
  let srdStatus = $state("");
  let srdError = $state<string>();

  async function checkSrd() {
    srdBusy = "checking";
    [srdLatest, srdStatus, srdError] = [null, "", undefined];
    try {
      const found = await checkSrdUpdate();
      srdLatest = found.latest;
      srdStatus = found.latest ? tu.available(found.latest) : tu.upToDate(found.installed);
    } catch (e) {
      srdError = tu.checkError(errorMessage(e));
    } finally {
      srdBusy = null;
    }
  }

  async function updateSrdData(version: string) {
    srdBusy = "updating";
    [srdStatus, srdError] = ["", undefined];
    try {
      await updateSrd(version);
      [srdLatest, srdStatus] = [null, tu.updated(version)];
      forgetPictures();
      packsChanged();
    } catch (e) {
      [srdStatus, srdError] = [tu.available(version), tu.updateError(errorMessage(e))];
    } finally {
      srdBusy = null;
    }
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
  <!-- One row per pack, its columns lined up with the other packs' (CSS subgrid). -->
  <ul class="packs">
    {#each packs as pack (pack.id)}
      <li>
        {#if removing === pack.id}
          <span class="confirm">
            {pack.homebrew ? t.deleteQuestion(pack.name) : t.removeQuestion(pack.name)}
            <button class="btn small" use:focus onclick={() => (removing = null)}>{t.cancel}</button>
            <button class="btn small btn-danger" onclick={() => remove(pack)}>{pack.homebrew ? t.delete : t.remove}</button>
          </span>
        {:else if pack.errors.length === 0}
          {@const progress = imageDownloads[pack.id]}
          {@const work = packProgress[pack.id] ?? (pack.id === SRD && srdBusy === "updating" && srdLatest ? { name: pack.name, fraction: null, text: tu.updating(srdLatest) } : null)}
          <Switch id="pack-{pack.id}" label={pack.name} checked={packOn(pack)} disabled={saving} onchange={(on) => toggle(pack, on)} />
          {#if work}
            {@render working(work)}
          {:else}
            <span class="muted">{pack.version}</span>
            <span class="muted number">{t.entries(pack.entries)}</span>
            <span class="muted">{pack.license}</span>
          {/if}
          <span class="images">
            {#if packImages[pack.id]}
              <label class="check">
                <!-- Stays as it was until the question is answered. Can't be ticked again while a cancelled download
                     is still finishing its last images, or the new download wouldn't start. -->
                <input
                  type="checkbox"
                  checked={imagesOn(pack.id)}
                  disabled={saving || (!imagesOn(pack.id) && !!progress)}
                  onchange={(e) => {
                    asking = { pack, turningOn: e.currentTarget.checked };
                    e.currentTarget.checked = !e.currentTarget.checked;
                  }}
                />
                {ti.label}
              </label>
              {#if !progress && imagesOn(pack.id) && packImages[pack.id].kept === packImages[pack.id].count}
                <span class="muted">{ti.allKept}</span>
              {/if}
            {/if}
          </span>
          {@render trash(pack)}
          {#if progress}
            <div class="progress">
              <progress max={progress.total || 1} value={progress.done + progress.failed} aria-label={ti.enable}></progress>
              <span class="muted">{ti.progress(count.format(progress.done + progress.failed), count.format(progress.total))}</span>
              <button class="btn small" onclick={() => cancelImageDownload(pack.id)}>{ti.cancelDownload}</button>
            </div>
          {/if}
        {:else}
          <p class="name">{pack.name}</p>
          <p class="invalid">{t.invalid}</p>
          {@render trash(pack)}
          <ul class="errors">
            {#each pack.errors as error, i (i)}
              <li>{error}</li>
            {/each}
          </ul>
        {/if}
      </li>
    {/each}
    {#each coming as [id, progress] (id)}
      <li>
        <span class="name coming">{progress.name}</span>
        {@render working(progress)}
      </li>
    {/each}
  </ul>
  {#if packs.some((p) => p.id === SRD && p.errors.length === 0)}
    <div class="update">
      <button class="btn small" disabled={!!srdBusy} onclick={checkSrd}>{tu.check}</button>
      {#if srdLatest}
        <button class="btn small btn-primary" disabled={!!srdBusy} onclick={() => updateSrdData(srdLatest!)}>{tu.update}</button>
      {/if}
      <span class="muted" role="status">{srdBusy === "checking" ? tu.checking : srdStatus}</span>
    </div>
    {#if srdError}<p class="error" role="alert">{srdError}</p>{/if}
  {/if}
{/if}

<!-- An installed pack's remove button, in the last column; a bundled pack has none. -->
{#snippet trash(pack: PackSummary)}
  <span class="trash">
    {#if pack.installed}
      <button class="icon-btn" aria-label={t.removeName(pack.name)} disabled={saving || !!packProgress[pack.id] || !!imageDownloads[pack.id] || (pack.id === SRD && !!srdBusy)} onclick={() => (removing = pack.id)}>
        <i class="ti ti-trash" aria-hidden="true"></i>
      </button>
    {/if}
  </span>
{/snippet}

<!-- An addon's work on a pack, in place of its version, entries and license. -->
{#snippet working(progress: Pick<PackProgress, "name" | "fraction" | "text">)}
  <span class="working">
    <progress max="1" value={progress.fraction ?? undefined} aria-label={progress.name}></progress>
    <span class="muted">{progress.text}</span>
  </span>
{/snippet}

{#if addonsError}
  <p class="error" role="alert">{t.addonsError(addonsError)}</p>
{/if}
{#if panels.length}
  <div class="panels">
    {#each panels as addon (addon.id)}
      <AddonFrame {addon} entry={panelIn(addon, "rules")!} panel="rules" onpackschange={packsChanged} />
    {/each}
  </div>
{/if}

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
    list-style: none;
  }

  p {
    font-size: var(--text-sm);
  }

  .packs {
    display: grid;
    /* The name takes what's left and wraps when long; the other columns fit their widest value. */
    grid-template-columns: minmax(0, 1fr) repeat(5, auto);
    column-gap: var(--space-5);
    font-size: var(--text-sm);
  }

  .packs > li {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: subgrid;
    align-items: center;
    row-gap: var(--space-2);
    padding: var(--space-3) 0;
  }

  .packs > li:first-child {
    padding-top: 0;
  }

  .packs > li + li {
    border-top: var(--border-width) solid var(--color-border);
  }

  .number {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .name {
    font-size: var(--text-md);
    font-weight: var(--weight-medium);
  }

  .muted {
    color: var(--color-text-muted);
  }

  .images {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  /* An addon's work on a pack: across the version, entries and license columns. */
  .working {
    grid-column: 2 / 5;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
  }

  .working progress {
    flex: 1;
    min-width: var(--space-10);
    accent-color: var(--color-accent);
  }

  .working .muted {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* A pack not installed yet: its name where the switch's label would be. */
  .coming {
    padding-left: calc(2 * var(--control-height-sm) - 2 * var(--space-1) + var(--space-3));
    font-size: var(--text-sm);
  }

  .trash {
    grid-column: -2;
    display: flex;
    justify-content: flex-end;
  }

  /* Asking before a pack is removed: across the whole row. */
  .confirm {
    grid-column: 1 / -1;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }

  /* While downloading: its own line under the pack's row, the full width. */
  .progress {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .progress progress {
    flex: 1;
    min-width: 0;
    accent-color: var(--color-accent);
  }

  .small {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
  }

  /* An invalid pack: its name, "Not loaded" in the next columns, its errors under them. */
  .invalid {
    grid-column: 2 / -2;
    color: var(--color-danger);
    font-weight: var(--weight-medium);
  }

  .errors {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding-left: var(--space-4);
    list-style: disc;
    color: var(--color-text-muted);
  }

  /* Update rules, under the packs. */
  .update {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    padding-top: var(--space-3);
    border-top: var(--border-width) solid var(--color-border);
    font-size: var(--text-sm);
  }

  /* The addon panels under the packs, set apart from them. */
  .panels {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding-top: var(--space-4);
    border-top: var(--border-width) solid var(--color-border);
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
