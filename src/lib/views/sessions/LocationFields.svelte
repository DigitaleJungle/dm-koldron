<script lang="ts">
  // A location's own fields, in its editor, in the scene editor's tabs (SideTabs): Atmosphere, Purpose and Secrets
  // (rich text, as in What happens; the three share the panel's height), its images (thumbnails; add, remove, and open
  // larger with previous and next), its map (the encounter map: a picture with pins, filling the panel) and its linked
  // NPCs (chips and a "+ Link" dropdown). Images and the map are stored as soon as they're chosen; the rest saves with
  // the world. The open tab is kept when another location opens.
  import { choosePictureFile, pictureSrc } from "$lib/api/campaigns";
  import { errorMessage } from "$lib/api/storage";
  import { addLocationImage, setLocationMap, type Location, type World } from "$lib/api/world";
  import RichText from "$lib/components/RichText.svelte";
  import SideTabs from "$lib/components/SideTabs.svelte";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
  import NotesPanel from "$lib/views/encounters/NotesPanel.svelte";
  import { link, NPC_DRAG, treeOrder, unlink } from "./sessions";
  import { trackStoring } from "./worldDraft.svelte";

  let { location = $bindable(), world }: { location: Location; world: World } = $props();

  const t = strings.sessions.locationFields;
  const m = strings.sessions.manage;
  const campaign = campaigns.current!.id;

  const TABS = [
    { id: "text", icon: "ti-align-justified", label: t.text },
    { id: "images", icon: "ti-library-photo", label: t.images },
    { id: "map", icon: "ti-map", label: t.map },
    { id: "npcs", icon: "ti-users", label: t.npcs },
  ] as const;
  let current = $state<string>("text");
  /** As many as Rust allows (MAX_IMAGES in src-tauri/src/commands/world.rs). */
  const MAX_IMAGES = 50;
  const indent = (depth: number, name: string) => "  ".repeat(depth) + name;

  // ---- Images ------------------------------------------------------------------------------------
  let adding = $state(false);
  let imageError = $state<string | null>(null);

  async function addImage() {
    if (adding) return;
    [adding, imageError] = [true, null];
    try {
      const path = await choosePictureFile();
      if (typeof path !== "string") return;
      await trackStoring(
        addLocationImage(campaign, location.id, path).then((stored) => {
          location.images.push(stored.name);
          (location.pictures ??= []).push(stored.picture);
        }),
      );
    } catch (e) {
      imageError = t.imageError(errorMessage(e));
    } finally {
      adding = false;
    }
  }

  function removeImage(i: number) {
    location.images.splice(i, 1);
    location.pictures?.splice(i, 1);
  }

  let viewer: HTMLDialogElement | undefined = $state();
  let viewing = $state(0);
  const pictures = $derived(location.pictures ?? []);

  function view(i: number) {
    viewing = i;
    viewer?.showModal();
  }

  /** Previous (-1) or next (+1), round. */
  const step = (by: number) => (viewing = (viewing + by + pictures.length) % pictures.length);

  // ---- Map: stored at once, as an encounter's --------------------------------------------------------
  async function setMap(path: string | null) {
    location.map = await setLocationMap(campaign, location.id, path);
  }

  // ---- Linked NPCs -------------------------------------------------------------------------------
  const npcName = (id: string) => world.npcs.find((n) => n.id === id)?.name || m.unnamedNpc;
  const npcChoices = $derived(treeOrder(world.npcs).filter((r) => !location.npcIds.includes(r.entry.id)));

  // As in the session prep: while an NPC is dragged (from the NPCs tree, or a session's NPC cards), the whole editor
  // box around this (the nearest `.editor`) is a drop target: a dashed outline easing in and out, tinted while it's
  // under the pointer. Dropping anywhere in it links the NPC and says so in Linked NPCs.
  let dragging = $state(false);
  let over = $state(false);
  let added = $state("");
  const carriesNpc = (ev: DragEvent) => !!ev.dataTransfer?.types.includes(NPC_DRAG);

  /** An NPC picked up anywhere opens Linked NPCs, so the drop shows where it lands. */
  function dragStarted(ev: DragEvent) {
    dragging = carriesNpc(ev);
    if (dragging) [added, current] = ["", "npcs"];
  }

  const dragEnded = () => ([dragging, over] = [false, false]);

  function dropNpc(ev: DragEvent) {
    if (!carriesNpc(ev)) return;
    ev.preventDefault();
    dragEnded();
    const id = ev.dataTransfer!.getData(NPC_DRAG);
    if (!world.npcs.some((n) => n.id === id)) return;
    link(location.npcIds, id);
    current = "npcs";
    added = strings.sessions.scenes.added(npcName(id), location.name || m.unnamedLocation);
  }

  let anchor: HTMLElement | undefined = $state();
  /** The editor box this sits in: the drop target. */
  const box = $derived(anchor?.closest<HTMLElement>(".editor") ?? anchor);

  $effect(() => {
    if (!box) return;
    const target = box;
    const onOver = (ev: DragEvent) => {
      if (!carriesNpc(ev)) return;
      ev.preventDefault();
      ev.dataTransfer!.dropEffect = "copy";
      over = true;
    };
    const onLeave = (ev: DragEvent) => {
      if (!target.contains(ev.relatedTarget as Node | null)) over = false;
    };
    target.addEventListener("dragover", onOver);
    target.addEventListener("dragleave", onLeave);
    target.addEventListener("drop", dropNpc);
    return () => {
      target.removeEventListener("dragover", onOver);
      target.removeEventListener("dragleave", onLeave);
      target.removeEventListener("drop", dropNpc);
      target.classList.remove("npc-box", "npc-target", "npc-over");
    };
  });

  $effect(() => {
    box?.classList.add("npc-box");
    box?.classList.toggle("npc-target", dragging);
    box?.classList.toggle("npc-over", over);
  });

  function pickNpc(ev: Event & { currentTarget: HTMLSelectElement }) {
    if (ev.currentTarget.value) link(location.npcIds, ev.currentTarget.value);
    ev.currentTarget.value = "";
  }
</script>

<svelte:window ondragstart={dragStarted} ondragend={dragEnded} ondrop={dragEnded} />

<!-- Dragging is one way; Linked NPCs' dropdown does the same from the keyboard. -->
<div class="fields" bind:this={anchor}>
<SideTabs tabs={TABS} bind:current id="location" label={t.parts}>
{#if current === "text"}
  <!-- Keyed: a rich text field fills itself once, so another location needs fresh ones. -->
  {#key location.id}
    {#each [["atmosphere", t.atmosphere], ["purpose", t.purpose], ["secrets", t.secrets]] as const as [key, label] (key)}
      <section class="part text">
        <h3 class="small-label">{label}</h3>
        <RichText {label} extended bind:value={location[key]} />
      </section>
    {/each}
  {/key}
{:else if current === "images"}
<section class="part" aria-labelledby="images-{location.id}">
  <h3 id="images-{location.id}" class="small-label">{t.images}</h3>
  <ul class="images">
    {#each pictures as picture, i (location.images[i])}
      <li>
        <button class="thumb" aria-label={t.openImage(i + 1)} onclick={() => view(i)}>
          <img src={pictureSrc(picture)} alt="" />
        </button>
        <button class="icon-btn remove" aria-label={t.removeImage(i + 1)} onclick={() => removeImage(i)}>
          <i class="ti ti-x" aria-hidden="true"></i>
        </button>
      </li>
    {/each}
    <li>
      <button class="thumb add" disabled={adding || location.images.length >= MAX_IMAGES} onclick={addImage}>
        <i class="ti ti-photo-plus" aria-hidden="true"></i>
        {t.addImage}
      </button>
    </li>
  </ul>
  {#if imageError}<p class="error" role="alert">{imageError}</p>{/if}
</section>
{:else if current === "map"}
<section class="part fill" aria-labelledby="map-{location.id}">
  <h3 id="map-{location.id}" class="small-label">{t.map}</h3>
  <div class="map">
    <NotesPanel
      topic="map"
      bind:notes={() => ({ description: "", pins: location.pins }), (v) => (location.pins = v.pins)}
      map={location.map ?? null}
      locked={false}
      {setMap}
    />
  </div>
</section>
{:else}
<section class="part">
  <label for="npcs-{location.id}" class="small-label">{t.npcs}</label>
  <div class="chips">
    {#each location.npcIds.filter((id) => world.npcs.some((n) => n.id === id)) as id (id)}
      <span class="chip-link">
        <strong>{npcName(id)}</strong>
        <button class="icon-btn" aria-label={strings.sessions.scenes.unlink(npcName(id))} onclick={() => unlink(location.npcIds, id)}>
          <i class="ti ti-x" aria-hidden="true"></i>
        </button>
      </span>
    {/each}
    <select id="npcs-{location.id}" class="chip-pick" value="" onchange={pickNpc}>
      <option value="">{strings.sessions.scenes.link}</option>
      {#each npcChoices as r (r.entry.id)}
        <option value={r.entry.id}>{indent(r.depth, r.entry.name || m.unnamedNpc)}</option>
      {/each}
    </select>
  </div>
  <p class="dropped" role="status">
    {#if added}<i class="ti ti-check" aria-hidden="true"></i>{added}{/if}
  </p>
</section>
{/if}
</SideTabs>
</div>

<dialog bind:this={viewer} class="dialog viewer" aria-label={t.imageOf(viewing + 1, pictures.length)} onkeydown={(e) => (e.key === "ArrowLeft" ? step(-1) : e.key === "ArrowRight" ? step(1) : null)}>
  <div class="viewer-head">
    <span class="muted">{t.imageOf(viewing + 1, pictures.length)}</span>
    <button class="icon-btn icon-btn-lg" aria-label={t.close} onclick={() => viewer?.close()}>
      <i class="ti ti-x" aria-hidden="true"></i>
    </button>
  </div>
  {#if pictures[viewing]}<img class="large" src={pictureSrc(pictures[viewing])} alt="" />{/if}
  {#if pictures.length > 1}
    <div class="viewer-nav">
      <button class="btn" aria-label={t.previous} onclick={() => step(-1)}><i class="ti ti-chevron-left" aria-hidden="true"></i></button>
      <button class="btn" aria-label={t.next} onclick={() => step(1)}><i class="ti ti-chevron-right" aria-hidden="true"></i></button>
    </div>
  {/if}
</dialog>

<style>
  .fields {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  /* The editor box while an NPC is dragged: the dashed outline and tint a scene gets in the session prep. The outline
     is a layer over the box (the tabs sit flush with its edge, over where an outline would be), letting the pointer
     through. The box's own background (.editor in Sessions.svelte and Session.svelte) is as specific as
     .editor.npc-over, so [class] makes the tint win whatever order the styles load in. */
  :global(.editor.npc-box[class]) {
    position: relative;
    transition: background-color var(--duration-hover) var(--ease-hover);
  }

  :global(.editor.npc-box[class])::after {
    content: "";
    position: absolute;
    inset: 0;
    z-index: 1;
    border: var(--focus-ring) dashed transparent;
    border-radius: inherit;
    pointer-events: none;
    transition: border-color var(--duration-hover) var(--ease-hover);
  }

  :global(.editor.npc-target[class])::after {
    border-color: var(--color-accent);
  }

  :global(.editor.npc-over[class]) {
    background: var(--color-accent-soft);
  }

  :global([data-motion="reduce"] .editor.npc-box[class]),
  :global([data-motion="reduce"] .editor.npc-box[class])::after {
    transition: none;
  }

  /* What the last drop added, as the session prep's right column says it. */
  .dropped {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--color-accent);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
  }


  .part {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  /* Atmosphere, Purpose and Secrets share the panel's height, each scrolling inside its field. */
  .part.text {
    flex: 1 1 0;
    min-height: 0;
  }

  /* No least height inside: the editing area fills its field, so it only scrolls when the text is longer. */
  .part.text :global(.rich),
  .part.text :global(.host),
  .part.text :global(.host .editor) {
    min-height: 0;
  }

  .part.text :global(.host) {
    overflow-y: auto;
  }

  .part.fill {
    flex: 1;
  }

  .small-label {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
  }

  .images {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .images li {
    position: relative;
  }

  .thumb {
    display: grid;
    place-items: center;
    width: var(--location-thumb-size);
    height: var(--location-thumb-size);
    padding: 0;
    overflow: hidden;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
    cursor: pointer;
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .thumb.add {
    gap: var(--space-1);
    align-content: center;
    border-style: dashed;
    color: var(--color-text-muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
  }

  /* On the thumbnail's corner. */
  .remove {
    position: absolute;
    top: var(--space-1);
    right: var(--space-1);
    background: var(--color-surface);
  }

  .map {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: var(--location-map-height);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .error {
    margin: 0;
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  .muted {
    color: var(--color-text-muted);
  }

  .viewer {
    width: min(100% - 2 * var(--space-4), var(--location-viewer-width));
  }

  .viewer-head,
  .viewer-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .large {
    max-width: 100%;
    max-height: 70vh;
    object-fit: contain;
    align-self: center;
  }
</style>
