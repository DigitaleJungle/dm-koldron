<script lang="ts">
  // Add character / Edit: one form in a modal dialog. Rust checks every rule again when it saves; the form shows its
  // answer. A picked picture is dragged into place in a square frame, cropped here and stored once the character is saved.
  // A saved character can be deleted here too, after a confirmation in the footer.
  import { onDestroy, tick, untrack } from "svelte";
  import { pictureSrc } from "$lib/api/campaigns";
  import { deleteCharacter, saveCharacter, setCharacterPicture, type Character } from "$lib/api/characters";
  import { errorMessage } from "$lib/api/storage";
  import { strings } from "$lib/strings";
  import { clamp01, cropPicture, defaultCrop, loadPicture, MAX_STAT, totalLevel, type Crop } from "./characters";

  let {
    campaign,
    character,
    classNames,
    onclose,
    onsaved,
    ondeleted,
  }: {
    campaign: string;
    /** The character to edit, or a blank one to add. */
    character: Character;
    /** The class names the packs offer; any other name can be typed. */
    classNames: string[];
    onclose: () => void;
    onsaved: (saved: Character) => void;
    ondeleted: (deleted: Character) => void;
  } = $props();

  const t = strings.characters.form;
  const id = $props.id();
  // A copy: Cancel leaves the character as it was.
  let draft = $state<Character>(untrack(() => $state.snapshot(character)));
  /** A newly picked picture, while it can still be moved; `removed`: the saved picture is to go. */
  let picked = $state<{ image: ImageBitmap; url: string; crop: Crop } | null>(null);
  let removed = $state(false);
  let drag: { x: number; y: number; crop: Crop } | null = $state(null);
  let pictureError = $state<string | null>(null);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let dialog: HTMLDialogElement | undefined = $state();
  let fileInput: HTMLInputElement | undefined = $state();

  const savedSrc = $derived(!removed && draft.picture ? pictureSrc(draft.picture) : null);
  const total = $derived(totalLevel(draft));

  // Native modal dialog: keeps focus inside, closes on Escape, returns focus to the opener.
  $effect(() => {
    if (dialog && !dialog.open) dialog.showModal();
  });

  async function pick(event: Event & { currentTarget: HTMLInputElement }) {
    const file = event.currentTarget.files?.[0];
    event.currentTarget.value = "";
    if (!file) return;
    pictureError = null;
    try {
      const { image, url } = await loadPicture(file);
      picked?.image.close();
      picked = { image, url, crop: defaultCrop() };
    } catch {
      pictureError = t.pictureError;
    }
  }

  function removePicture() {
    picked?.image.close();
    picked = null;
    removed = true;
  }

  onDestroy(() => picked?.image.close());

  /** How far the picture sticks out of the square frame on screen, per axis (the longer side only). */
  function overflow(frame: HTMLElement) {
    const { width: w, height: h } = picked!.image;
    const size = frame.clientWidth;
    return { x: w > h ? (size * w) / h - size : 0, y: h > w ? (size * h) / w - size : 0 };
  }

  function startDrag(event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!picked) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drag = { x: event.clientX, y: event.clientY, crop: { ...picked.crop } };
  }

  function moveDrag(event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!drag || !picked) return;
    const over = overflow(event.currentTarget);
    // Dragging the picture right shows more of its left side.
    picked.crop = {
      x: over.x ? clamp01(drag.crop.x - (event.clientX - drag.x) / over.x) : picked.crop.x,
      y: over.y ? clamp01(drag.crop.y - (event.clientY - drag.y) / over.y) : picked.crop.y,
    };
  }

  function nudge(event: KeyboardEvent) {
    const step = 0.05;
    const moves: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
    const move = moves[event.key];
    if (!move || !picked) return;
    event.preventDefault();
    picked.crop = { x: clamp01(picked.crop.x + move[0]), y: clamp01(picked.crop.y + move[1]) };
  }

  // Empty number fields are no value, not zero.
  const stat = (value: unknown) => (value === "" || value === null || value === undefined ? null : Number(value));

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (saving) return;
    saving = true;
    error = null;
    const stats = ["maxHp", "currentHp", "tempHp", "ac", "passivePerception", "passiveInsight", "passiveInvestigation"] as const;
    const toSave: Character = { ...$state.snapshot(draft), classes: draft.classes.map((c) => ({ name: c.name, level: Number(c.level) })) };
    for (const key of stats) toSave[key] = stat(draft[key]);
    let saved: Character;
    try {
      saved = await saveCharacter(campaign, toSave);
    } catch (e) {
      error = t.saveError(errorMessage(e));
      saving = false;
      return;
    }
    if (picked || (removed && character.picture)) {
      try {
        saved = await setCharacterPicture(campaign, saved.id, picked ? cropPicture(picked.image, picked.crop) : null);
      } catch (e) {
        // Saved already: keep the form open on the saved character so the picture can be tried again.
        [draft.id, draft.createdAt, error, saving] = [saved.id, saved.createdAt, t.pictureSaveError(errorMessage(e)), false];
        onsaved(saved);
        return;
      }
    }
    onsaved(saved);
    onclose();
  }

  // Delete: the footer asks first (Cancel, the default, or Delete); deleting closes the form.
  let confirming = $state(false);
  let deleting = $state(false);

  async function askDelete() {
    confirming = true;
    await tick();
    document.getElementById(`${id}-keep`)?.focus();
  }

  async function keep() {
    confirming = false;
    await tick();
    document.getElementById(`${id}-delete`)?.focus();
  }

  async function remove() {
    deleting = true;
    error = null;
    try {
      await deleteCharacter(campaign, character.id);
    } catch (e) {
      error = strings.characters.deleteError(character.name, errorMessage(e));
      deleting = false;
      return;
    }
    ondeleted(character);
    onclose();
  }

  // Only a click that both starts and ends on the backdrop closes the form: a text selection or a picture drag
  // that ends outside the dialog must not lose what was typed.
  let pressedBackdrop = false;
  function onBackdropClick(event: MouseEvent) {
    if (pressedBackdrop && event.target === dialog) onclose();
  }

  const stats: { key: "maxHp" | "currentHp" | "tempHp" | "ac" | "passivePerception" | "passiveInsight" | "passiveInvestigation"; label: string }[] = [
    { key: "maxHp", label: t.maxHp },
    { key: "currentHp", label: t.currentHp },
    { key: "tempHp", label: t.tempHp },
    { key: "ac", label: t.ac },
    { key: "passivePerception", label: t.passivePerception },
    { key: "passiveInsight", label: t.passiveInsight },
    { key: "passiveInvestigation", label: t.passiveInvestigation },
  ];
</script>

<dialog
  bind:this={dialog}
  class="dialog"
  aria-labelledby="{id}-title"
  onclose={onclose}
  onpointerdown={(e) => (pressedBackdrop = e.target === dialog)}
  onclick={onBackdropClick}
>
  <form onsubmit={save}>
    <header>
      <h2 id="{id}-title">{character.id ? t.editTitle : t.newTitle}</h2>
      <button type="button" class="icon-btn icon-btn-lg" aria-label={t.close} onclick={onclose}>
        <i class="ti ti-x" aria-hidden="true"></i>
      </button>
    </header>

    <div class="body">
      <div class="identity">
        <div class="picture">
          <div class="frame">
            {#if picked}
              <div
                class="avatar draggable"
                class:dragging={drag !== null}
                role="slider"
                tabindex="0"
                aria-label={t.picture}
                aria-valuenow={Math.round((picked.image.width >= picked.image.height ? picked.crop.x : picked.crop.y) * 100)}
                aria-valuetext={t.picturePosition(Math.round(picked.crop.x * 100), Math.round(picked.crop.y * 100))}
                style:background-image="url({picked.url})"
                style:background-position="{picked.crop.x * 100}% {picked.crop.y * 100}%"
                onpointerdown={startDrag}
                onpointermove={moveDrag}
                onpointerup={() => (drag = null)}
                onpointercancel={() => (drag = null)}
                onkeydown={nudge}
              ></div>
            {:else if savedSrc}
              <img class="avatar" src={savedSrc} alt="" />
            {:else}
              <span class="avatar" aria-hidden="true"><i class="ti ti-user"></i></span>
            {/if}
            {#if picked || savedSrc}
              <button type="button" class="remove" aria-label={t.removePicture} onclick={removePicture}>
                <i class="ti ti-x" aria-hidden="true"></i>
              </button>
            {/if}
          </div>
          <input bind:this={fileInput} id="{id}-file" type="file" accept="image/png,image/jpeg,image/webp,image/gif" class="sr-only" tabindex="-1" onchange={pick} />
          <button type="button" class="btn small" onclick={() => fileInput?.click()}>{picked || savedSrc ? t.changePicture : t.choosePicture}</button>
          {#if pictureError}<p class="error" role="alert">{pictureError}</p>{/if}
        </div>
        <div class="fields">
          <label class="field">
            <span>{t.name} <span aria-hidden="true">*</span></span>
            <!-- svelte-ignore a11y_autofocus: the dialog opens to type the name -->
            <input bind:value={draft.name} required maxlength="100" autocomplete="off" autofocus />
          </label>
          <label class="field">
            <span>{t.player}</span>
            <input bind:value={draft.player} maxlength="100" autocomplete="off" />
          </label>
        </div>
      </div>

      <fieldset>
        <legend class="label">{t.classes}</legend>
        <datalist id="{id}-classes">
          {#each classNames as name (name)}<option value={name}></option>{/each}
        </datalist>
        {#each draft.classes as row, i (i)}
          <div class="class-row">
            <input aria-label={t.className(i + 1)} list="{id}-classes" bind:value={row.name} required maxlength="100" autocomplete="off" />
            <input class="level" aria-label={t.classLevel(i + 1)} type="number" min="1" max="20" step="1" bind:value={row.level} required />
            {#if draft.classes.length > 1}
              <button type="button" class="icon-btn" aria-label={t.removeClass(i + 1)} onclick={() => draft.classes.splice(i, 1)}>
                <i class="ti ti-x" aria-hidden="true"></i>
              </button>
            {/if}
          </div>
        {/each}
        <div class="class-foot">
          <button type="button" class="btn small" onclick={() => draft.classes.push({ name: "", level: 1 })}>
            <i class="ti ti-plus" aria-hidden="true"></i>
            {t.addClass}
          </button>
          <span class:error={total > 20} role={total > 20 ? "alert" : undefined}>{total > 20 ? t.tooHigh : t.total(total)}</span>
        </div>
      </fieldset>

      <fieldset>
        <legend class="label">{t.stats}</legend>
        <div class="stats">
          {#each stats as s (s.key)}
            <label class="field">
              <span>{s.label}</span>
              <input type="number" min="0" max={MAX_STAT} step="1" bind:value={draft[s.key]} />
            </label>
          {/each}
        </div>
      </fieldset>

      <label class="check">
        <input type="checkbox" bind:checked={draft.onHome} />
        {t.addToHome}
      </label>

      {#if error}<p class="error" role="alert">{error}</p>{/if}
    </div>

    <footer>
      {#if confirming}
        <p class="confirm" role="alert">{strings.characters.confirmDelete(character.name)}</p>
        <button id="{id}-keep" type="button" class="btn" onclick={keep}>{t.cancel}</button>
        <button type="button" class="btn btn-danger" disabled={deleting} onclick={remove}>{strings.characters.delete}</button>
      {:else}
        {#if character.id}
          <button id="{id}-delete" type="button" class="btn delete" onclick={askDelete}>
            <i class="ti ti-trash" aria-hidden="true"></i>
            {strings.characters.delete}
          </button>
        {/if}
        <button type="button" class="btn" onclick={onclose}>{t.cancel}</button>
        <button type="submit" class="btn btn-primary" disabled={saving || total > 20}>{saving ? t.saving : t.save}</button>
      {/if}
    </footer>
  </form>
</dialog>

<style>
  dialog {
    width: min(var(--character-form-width), calc(100vw - 2 * var(--space-4)));
    max-height: calc(100vh - 2 * var(--space-4));
    padding: 0;
    box-shadow: var(--shadow-drawer);
  }

  form {
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 2 * var(--space-4));
  }

  header,
  footer {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-3) var(--space-3) var(--space-5);
  }

  header {
    justify-content: space-between;
    border-bottom: var(--border-width) solid var(--color-border);
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-semibold);
  }

  footer {
    justify-content: flex-end;
    padding-right: var(--space-5);
    border-top: var(--border-width) solid var(--color-border);
  }

  /* Delete on the left, away from Save; the question takes its place while it's asked. */
  .delete {
    margin-right: auto;
    color: var(--color-danger);
  }

  .confirm {
    margin: 0 auto 0 0;
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    padding: var(--space-5);
  }

  .identity {
    display: flex;
    gap: var(--space-5);
  }

  .picture {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
  }

  .avatar {
    width: var(--avatar-pick-size);
    height: var(--avatar-pick-size);
    display: grid;
    place-items: center;
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
    color: var(--color-text-muted);
    font-size: calc(var(--avatar-pick-size) / 2);
    object-fit: cover;
  }

  .frame {
    position: relative;
  }

  .draggable {
    background-repeat: no-repeat;
    background-size: cover;
    cursor: grab;
    touch-action: none;
  }

  .draggable.dragging {
    cursor: grabbing;
  }

  .draggable:focus-visible {
    outline: var(--focus-ring) solid var(--color-focus);
    outline-offset: 2px;
  }

  /* Remove: a red dot with a cross on the picture's top-right corner. */
  .remove {
    position: absolute;
    top: calc(-1 * var(--space-2));
    right: calc(-1 * var(--space-2));
    display: grid;
    place-items: center;
    width: var(--control-height-sm);
    height: var(--control-height-sm);
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--color-danger-fill);
    color: var(--color-accent-text);
    box-shadow: var(--shadow-card);
    cursor: pointer;
  }

  .remove:hover {
    filter: brightness(1.1);
  }

  .remove:focus-visible {
    outline: var(--focus-ring) solid var(--color-focus);
    outline-offset: 2px;
  }

  .fields {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  input:not([type="checkbox"]) {
    min-height: var(--control-height);
    padding: 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    font-weight: var(--weight-regular);
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  legend {
    padding: 0;
    margin-bottom: var(--space-2);
  }

  .class-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .class-row input:first-child {
    flex: 1;
    min-width: 0;
  }

  .level {
    width: var(--level-input-width);
  }

  .class-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--stat-input-width), 1fr));
    gap: var(--space-3);
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .small {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
    font-size: var(--text-sm);
  }

  .error {
    margin: 0;
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  @media (max-width: 600px) {
    .identity {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
