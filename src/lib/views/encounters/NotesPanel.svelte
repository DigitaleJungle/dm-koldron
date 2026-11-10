<script lang="ts" module>
  export const TOPICS = ["map", "description"] as const;
  export type Topic = (typeof TOPICS)[number];
  export const TOPIC_ICONS: Record<Topic, string> = { map: "ti-map", description: "ti-notes" };
</script>

<script lang="ts">
  // One of the encounter's notes, inside Prepare's details card (in the stat block's place): the Notes text, or
  // the map: a picture with numbered pins, each with its own text. A Ready encounter's notes are read-only. A session's
  // Map step uses the map the same way, with its own `setMap`.
  import { choosePictureFile, pictureSrc, type Picture } from "$lib/api/campaigns";
  import type { Notes } from "$lib/api/encounters";
  import { errorMessage } from "$lib/api/storage";
  import { untrack } from "svelte";
  import RichText from "$lib/components/RichText.svelte";
  import { cleanHtml, isEmpty } from "$lib/components/richtext";
  import { libraryHome } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";

  let {
    topic,
    notes = $bindable(),
    map,
    locked,
    setMap,
    saveFirst = strings.encounters.notes.saveFirst,
  }: {
    topic: Topic;
    notes: Notes;
    map: Picture | null;
    locked: boolean;
    /** Stores the picture file at `path` as the map, or removes it (null), and shows the result. Null until what the
     *  map belongs to is saved: only then can it have one (`saveFirst` says so). */
    setMap: ((path: string | null) => Promise<void>) | null;
    saveFirst?: string;
  } = $props();

  const t = strings.encounters.notes;
  /** How far a press moves before it's a drag (a pin, or the zoomed picture), not a click. */
  const DRAG_PX = 3;

  // ---- The map ----------------------------------------------------------------------------------
  let mapBusy = $state(false); // the file dialog is open, or the map is being changed
  // A picked picture being shrunk and stored: a progress bar. Rust does it in one step, so the bar has no percentage.
  let storing = $state(false);
  let mapError = $state<string | null>(null);
  async function changeMap(choose: boolean) {
    if (!setMap || mapBusy) return;
    [mapBusy, mapError] = [true, null];
    try {
      const path = choose ? await choosePictureFile() : null;
      if (choose && !path) return; // the DM cancelled the dialog
      storing = choose;
      await setMap(path);
    } catch (e) {
      mapError = t.mapError(errorMessage(e));
    } finally {
      [mapBusy, storing] = [false, false];
    }
  }

  let image: HTMLElement | undefined = $state();
  let pinButtons: HTMLButtonElement[] = $state([]);
  let pinTitle: HTMLInputElement | undefined = $state();
  /** The open pin: marked on the map, its text in a popup next to it. */
  let current = $state<number | null>(null);

  const clamp = (v: number) => Math.min(1, Math.max(0, v));
  /** Where a pointer is on the picture, from 0 to 1 across and down, zoomed in or not. */
  function spot(e: PointerEvent | MouseEvent) {
    const box = image!.getBoundingClientRect();
    return { x: clamp((e.clientX - box.left - view.x) / (fitW * view.zoom)), y: clamp((e.clientY - box.top - view.y) / (fitH * view.zoom)) };
  }

  // ---- Zoom: the map fills the card; at 1× the picture fits inside it, centred. The scroll wheel zooms around the
  // pointer, up to MAX_ZOOM; zoomed in, dragging the picture pans it.
  const MAX_ZOOM = 8;
  let mapW = $state(0);
  let mapH = $state(0);
  /** The picture's own size; 0 until it has loaded. */
  let naturalWidth = $state(0);
  let naturalHeight = $state(0);
  // The picture's size at 1×: as large as fits.
  const scale = $derived(naturalWidth && naturalHeight ? Math.min(mapW / naturalWidth, mapH / naturalHeight) : 0);
  const fitW = $derived(naturalWidth * scale);
  const fitH = $derived(naturalHeight * scale);
  /** The zoom, and the picture's offset in px from the map's top-left corner. */
  let view = $state({ zoom: 1, x: 0, y: 0 });
  // Smaller than the map one way, the picture is centred that way; larger, it can't be pulled away from an edge.
  const place = (size: number, room: number, at: number) => (size <= room ? (room - size) / 2 : Math.min(0, Math.max(room - size, at)));
  const fit = (v: { zoom: number; x: number; y: number }) => ({ zoom: v.zoom, x: place(fitW * v.zoom, mapW, v.x), y: place(fitH * v.zoom, mapH, v.y) });
  /** Zooms by `factor`, keeping the spot at px, py (from the map's top-left corner) where it is. */
  function zoomAt(px: number, py: number, factor: number) {
    const zoom = Math.min(MAX_ZOOM, Math.max(1, view.zoom * factor));
    const k = zoom / view.zoom;
    view = fit({ zoom, x: px - (px - view.x) * k, y: py - (py - view.y) * k });
  }
  // A resized map, or a picture that has just loaded, keeps the picture in place.
  $effect(() => {
    void [fitW, fitH, mapW, mapH];
    view = fit(untrack(() => view));
  });
  // Another picture starts at 1×.
  $effect(() => {
    void map?.version;
    view = untrack(() => fit({ zoom: 1, x: 0, y: 0 })); // not the map's size, or every resize would reset the zoom
  });
  // Not passive, so the wheel zooms the map instead of scrolling the panel.
  $effect(() => {
    const el = image;
    if (!el) return;
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      const box = el.getBoundingClientRect();
      zoomAt(e.clientX - box.left, e.clientY - box.top, Math.exp(-e.deltaY * 0.002));
    };
    el.addEventListener("wheel", wheel, { passive: false });
    return () => el.removeEventListener("wheel", wheel);
  });
  /** A pin's place on screen, in px from the map's top-left corner. */
  const onScreen = (p: { x: number; y: number }) => ({ left: view.x + p.x * fitW * view.zoom, top: view.y + p.y * fitH * view.zoom });
  const inView = (at: { left: number; top: number }) => at.left >= 0 && at.left <= mapW && at.top >= 0 && at.top <= mapH;

  // Panning: a press that moves less than DRAG_PX is a click (it adds a pin), more pans the picture.
  let pan: { from: { x: number; y: number }; start: { x: number; y: number }; moved: boolean } | null = null;
  let panned = false;
  function startPan(e: PointerEvent) {
    if (e.button !== 0) return;
    pan = { from: { x: e.clientX, y: e.clientY }, start: { x: view.x, y: view.y }, moved: false };
    panned = false;
  }
  function movePan(e: PointerEvent) {
    // The button let go off the map (before the pan was captured) ends it too.
    if (!pan || !(e.buttons & 1)) return void (pan = null);
    const [dx, dy] = [e.clientX - pan.from.x, e.clientY - pan.from.y];
    if (!pan.moved && Math.hypot(dx, dy) < DRAG_PX) return;
    // Captured only now: a captured press sends its click to the frame, and a click must reach the picture.
    if (!pan.moved) (e.currentTarget as Element).setPointerCapture(e.pointerId);
    pan.moved = true;
    view = fit({ zoom: view.zoom, x: pan.start.x + dx, y: pan.start.y + dy });
  }
  function endPan() {
    panned = !!pan?.moved;
    pan = null;
  }

  function addPin(at: { x: number; y: number }) {
    notes.pins.push({ ...at, title: "", text: "" });
    select(notes.pins.length - 1);
  }

  // Selecting a pin opens its popup, with the cursor in its title when it can be written.
  function select(i: number) {
    current = i;
    if (!locked) queueMicrotask(() => pinTitle?.focus());
  }

  /** Closes the popup; from the keyboard, focus goes back to its pin. */
  function close(refocus = false) {
    const i = current;
    current = null;
    if (refocus && i !== null) pinButtons[i]?.focus();
  }

  function removePin(i: number) {
    notes.pins.splice(i, 1);
    current = null;
  }

  // A click on the picture closes an open popup, or else adds a pin there; a click beside the picture or outside the
  // map closes it too.
  function clickMap(e: MouseEvent) {
    const target = e.target as Element;
    if (panned || !(target.tagName === "IMG" || target.classList.contains("frame"))) return void (panned = false);
    if (current !== null) close();
    else if (!locked && target.tagName === "IMG") addPin(spot(e));
  }

  // On the map (its pins and popup too): Escape closes the popup. On the map itself: Enter adds a pin in the middle
  // of what's in view, + and − zoom.
  function mapKey(e: KeyboardEvent) {
    if (e.key === "Escape" && current !== null && !e.defaultPrevented) {
      e.preventDefault();
      close(true);
    } else if (e.target !== image) {
      return;
    } else if (e.key === "Enter" && !locked) {
      e.preventDefault();
      addPin({ x: clamp((mapW / 2 - view.x) / (fitW * view.zoom)), y: clamp((mapH / 2 - view.y) / (fitH * view.zoom)) });
    } else if (e.key === "+" || e.key === "=" || e.key === "-") {
      e.preventDefault();
      zoomAt(mapW / 2, mapH / 2, e.key === "-" ? 1 / 1.25 : 1.25);
    }
  }

  // Dragging a pin moves it, keeping where on the pin it was grabbed; a press that moves less than DRAG_PX selects it.
  let drag: { index: number; from: { x: number; y: number }; grab: { x: number; y: number }; moved: boolean } | null = null;
  function startDrag(e: PointerEvent, i: number) {
    if (locked) return;
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    const at = spot(e);
    drag = { index: i, from: { x: e.clientX, y: e.clientY }, grab: { x: notes.pins[i].x - at.x, y: notes.pins[i].y - at.y }, moved: false };
  }
  function moveDrag(e: PointerEvent) {
    if (!drag || (!drag.moved && Math.hypot(e.clientX - drag.from.x, e.clientY - drag.from.y) < DRAG_PX)) return;
    const at = spot(e);
    Object.assign(notes.pins[drag.index], { x: clamp(at.x + drag.grab.x), y: clamp(at.y + drag.grab.y) });
    drag.moved = true;
  }
  function endDrag(i: number) {
    const moved = drag?.moved;
    drag = null;
    if (!moved) select(i);
  }

  // From the keyboard, the arrow keys move the focused pin (Shift for bigger steps).
  function nudge(e: KeyboardEvent, i: number) {
    const step = e.shiftKey ? 0.05 : 0.01;
    const move = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] }[e.key];
    if (locked || !move) return;
    e.preventDefault();
    const pin = notes.pins[i];
    [pin.x, pin.y] = [clamp(pin.x + move[0]), clamp(pin.y + move[1])];
  }
</script>

{#snippet text(value: string)}
  {#if isEmpty(value)}
    <p class="muted">{t.empty}</p>
  {:else}
    <div class="note-text prose">{@html cleanHtml(value, { links: true, base: libraryHome() })}</div>
  {/if}
{/snippet}

<svelte:document onpointerdown={(e) => current !== null && !image?.contains(e.target as Node) && close()} />

{#if topic !== "map"}
  {#if locked}
    {@render text(notes[topic])}
  {:else}
    <!-- Keyed: the field fills itself once, so another topic needs a fresh one. -->
    {#key topic}
      <RichText label={t.topics[topic]} bind:value={notes[topic]} />
    {/key}
  {/if}
{:else}
  {#if mapError}<p class="error" role="alert">{mapError}</p>{/if}
  {#if storing}<progress class="map-progress" aria-label={t.storingMap}></progress>{/if}
  {#if map}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_no_noninteractive_tabindex: from the keyboard, Enter on the focused map adds a pin, + and − zoom -->
    <div
      class="map"
      class:editable={!locked}
      bind:this={image}
      bind:clientWidth={mapW}
      bind:clientHeight={mapH}
      onclick={clickMap}
      tabindex="0"
      aria-label={locked ? t.mapLabelReadOnly : t.mapLabel}
      onkeydown={mapKey}
    >
      <!-- The picture zooms and pans inside its frame; the pins and popup stay their size, on top. -->
      <div class="frame" class:zoomed={view.zoom > 1} onpointerdown={startPan} onpointermove={movePan} onpointerup={endPan} onpointercancel={() => (pan = null)}>
        <img
          src={pictureSrc(map)}
          alt={t.topics.map}
          draggable="false"
          style:width="{fitW}px"
          style:height="{fitH}px"
          style:transform="translate({view.x}px, {view.y}px) scale({view.zoom})"
          bind:naturalWidth
          bind:naturalHeight
        />
      </div>
      {#if !locked}
        <!-- On the picture's corners: change it (top left), remove it (top right). -->
        <button class="icon-btn map-btn change" aria-label={t.changeMap} disabled={mapBusy} onclick={() => changeMap(true)}>
          <i class="ti ti-switch-horizontal" aria-hidden="true"></i>
        </button>
        <button class="icon-btn map-btn remove" aria-label={t.removeMap} disabled={mapBusy} onclick={() => changeMap(false)}>
          <i class="ti ti-trash" aria-hidden="true"></i>
        </button>
      {/if}
      {#each notes.pins as pin, i (pin)}
        {@const at = onScreen(pin)}
        <button
          class="pin"
          class:current={current === i}
          class:out={!inView(at)}
          style:left="{at.left}px"
          style:top="{at.top}px"
          aria-label={t.pin(i + 1, pin.title)}
          aria-expanded={current === i}
          bind:this={pinButtons[i]}
          onpointerdown={(e) => startDrag(e, i)}
          onpointermove={moveDrag}
          onpointerup={() => endDrag(i)}
          onpointercancel={() => (drag = null)}
          onclick={(e) => e.detail === 0 && select(i)}
          onkeydown={(e) => nudge(e, i)}
        >
          <span class="hex">{i + 1}</span>
        </button>
      {/each}
      {#if current !== null && notes.pins[current] && inView(onScreen(notes.pins[current]))}
        {@const pin = notes.pins[current]}
        {@const at = onScreen(pin)}
        <!-- Beside its pin: to the right, or to the left on the map's right half; above it on the lower part. -->
        <div
          class="pin-popup popover"
          class:left={at.left > mapW / 2}
          class:up={at.top > mapH * 0.6}
          style:left="{at.left}px"
          style:top="{at.top}px"
          role="dialog"
          aria-label={t.pin(current + 1, pin.title)}
          tabindex="-1"
        >
          <div class="popup-head">
            {#if locked}
              <h4 class="pin-title" class:muted={!pin.title}>{pin.title || t.untitled}</h4>
            {:else}
              <input
                class="pin-title text-field"
                bind:this={pinTitle}
                bind:value={notes.pins[current].title}
                maxlength="100"
                autocomplete="off"
                placeholder={t.pinTitle}
                aria-label={t.pinTitleLabel(current + 1)}
              />
              <button class="icon-btn" aria-label={t.removePin(current + 1)} onclick={() => removePin(current!)}>
                <i class="ti ti-trash" aria-hidden="true"></i>
              </button>
            {/if}
            <button class="icon-btn" aria-label={t.close} onclick={() => close(true)}>
              <i class="ti ti-x" aria-hidden="true"></i>
            </button>
          </div>
          {#if locked}
            {@render text(pin.text)}
          {:else}
            <!-- Keyed: the field fills itself once, so another pin needs a fresh one. -->
            {#key pin}
              <RichText label={t.pinText(current + 1)} compact bind:value={notes.pins[current].text} />
            {/key}
          {/if}
        </div>
      {/if}
    </div>
    {#if !locked && notes.pins.length === 0}<p class="muted">{t.addPin}</p>{/if}
  {:else if locked}
    <p class="muted">{t.noMap}</p>
  {:else if !setMap}
    <p class="muted">{saveFirst}</p>
  {:else}
    <button class="add-map" disabled={mapBusy} onclick={() => changeMap(true)}>
      <i class="ti ti-plus" aria-hidden="true"></i>
      {t.chooseMap}
    </button>
  {/if}
{/if}

<style>
  /* Without .prose's reading width and centring. */
  .note-text {
    flex: 1;
    max-width: none;
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .note-text > :global(:first-child) {
    margin-top: 0;
  }

  .note-text > :global(:last-child) {
    margin-bottom: 0;
  }

  .muted {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .error {
    margin: 0;
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  /* Fills the rest of the card (a set height when the window is narrow and the card grows with its content). */
  .map {
    position: relative;
    flex: 1;
    min-height: var(--map-empty-height);
    user-select: none;
  }

  .frame {
    position: absolute;
    inset: 0;
    overflow: hidden;
    border-radius: var(--radius-sm);
    background: var(--color-surface-sunken);
  }

  .frame.zoomed {
    cursor: grab;
  }

  .map img {
    position: absolute;
    top: 0;
    left: 0;
    display: block;
    max-width: none;
    transform-origin: 0 0;
  }

  .map.editable img {
    cursor: crosshair;
  }

  /* Zoomed in, a pin outside the view isn't shown. */
  .pin.out {
    display: none;
  }

  /* A numbered hexagon, centred on its spot. The button stays square so its focus ring isn't cut to the hexagon. */
  .pin {
    position: absolute;
    display: grid;
    place-items: center;
    width: var(--roster-badge-width);
    height: var(--roster-badge-height);
    padding: 0;
    border: none;
    background: none;
    transform: translate(-50%, -50%);
    cursor: grab;
    touch-action: none;
  }

  .hex {
    display: grid;
    place-items: center;
    width: var(--roster-badge-width);
    height: var(--roster-badge-height);
    flex: none;
    background: var(--potion-glow);
    color: var(--charcoal);
    font: var(--text-dice);
    clip-path: polygon(25% 0, 75% 0, 100% 50%, 75% 100%, 25% 100%, 0 50%);
  }

  .current > .hex {
    background: var(--ember);
    color: var(--on-fill);
  }

  /* With no value, the bar keeps moving until the map is stored. */
  .map-progress {
    width: 100%;
    accent-color: var(--color-accent);
  }

  /* No map yet: a large dashed drop-in place, like the add card in a list. */
  .add-map {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    min-height: var(--map-empty-height);
    border: var(--border-width) dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    cursor: pointer;
  }

  .add-map:hover:not(:disabled) {
    border-color: var(--color-text);
    color: var(--color-text);
  }

  .add-map .ti {
    font-size: var(--icon-size-lg);
  }

  /* Readable on any picture: a solid button with a border. */
  .map-btn {
    position: absolute;
    top: var(--space-2);
    z-index: 1;
    border: var(--border-width) solid var(--color-border-strong);
    background: var(--color-surface);
  }

  .map-btn.change {
    left: var(--space-2);
  }

  .map-btn.remove {
    right: var(--space-2);
  }

  /* A pin's text, next to it on the map. Text can be selected in it, unlike on the map itself. */
  .pin-popup {
    --dx: calc(var(--roster-badge-width) / 2 + var(--space-2));
    --dy: calc(-1 * var(--space-4));
    position: absolute;
    z-index: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    width: min(var(--pin-popup-width), 90vw);
    padding: var(--space-3);
    transform: translate(var(--dx), var(--dy));
    user-select: text;
    cursor: auto;
  }

  .pin-popup.left {
    --dx: calc(-100% - var(--roster-badge-width) / 2 - var(--space-2));
  }

  .pin-popup.up {
    --dy: calc(-100% + var(--space-4));
  }

  .popup-head {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .pin-title {
    flex: 1;
    min-width: 0;
    margin: 0;
    font: var(--text-body);
    font-weight: var(--weight-bold);
  }

  h4.pin-title {
    overflow-wrap: anywhere;
  }
</style>
