<script lang="ts">
  // A campaign's Home: its widgets (Party, Next session, Previous session, Random encounter) in a grid of three
  // columns, each one, two or three columns wide. A widget moves by dragging its grip onto another (the others make
  // room as it goes; Escape puts it back) and resizes by dragging its right edge, which snaps to whole columns; its ⋯
  // menu does both from the keyboard, and Remove takes it off Home (a dashed slot at the grid's end puts it back). The layout is saved in the campaign's folder as soon as it changes.
  import { tick } from "svelte";
  import { flip } from "svelte/animate";
  import { listCharacters, type Character } from "$lib/api/characters";
  import { getHomeLayout, setHomeLayout } from "$lib/api/home";
  import { errorMessage } from "$lib/api/storage";
  import Menu from "$lib/components/Menu.svelte";
  import { addons } from "$lib/stores/addons.svelte";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { showHeaderAction } from "$lib/stores/header.svelte";
  import { reduceMotion } from "$lib/stores/theme.svelte";
  import { strings } from "$lib/strings";
  import EncounterWidget from "./EncounterWidget.svelte";
  import NextSessionWidget from "./NextSessionWidget.svelte";
  import PartyWidget from "./PartyWidget.svelte";
  import SummaryWidget from "./SummaryWidget.svelte";
  import { fromSaved, moveWidget, snapWidth, WIDTHS, type WidgetId, type Width } from "./widgets";

  const t = strings.home;
  const campaign = campaigns.current!.id;

  let order = $state<WidgetId[] | null>(null); // null while loading
  let widths = $state<Record<WidgetId, Width>>(fromSaved(null).widths);
  let locked = $state(false);
  /** Widgets taken off Home (⋯ → Remove); the dashed slot at the grid's end puts them back. */
  let removed = $state<WidgetId[]>([]);
  /** Shared by Party (which changes it) and Random encounter, so a party change shows in its difficulty at once. */
  let characters = $state<Character[] | null>(null);
  let charactersError = $state<string | null>(null);
  // Again when an addon saved a character (it keeps it up to date).
  $effect(() => {
    void addons.charactersChanged;
    listCharacters(campaign).then(
      (l) => (characters = l.characters),
      (e) => (charactersError = t.party.loadError(errorMessage(e))),
    );
  });
  let loadError = $state<string | null>(null);
  let saveError = $state<string | null>(null);

  getHomeLayout(campaign).then(
    (saved) => ({ order, widths, locked, removed } = fromSaved(saved)),
    (e) => {
      loadError = t.loadError(errorMessage(e));
      ({ order, widths, locked, removed } = fromSaved(null));
    },
  );

  async function save() {
    if (!order) return;
    saveError = null;
    try {
      await setHomeLayout(campaign, { widgets: order, widths, locked, removed });
    } catch (e) {
      saveError = t.saveError(errorMessage(e));
    }
  }

  // Keyed widgets move in the page, which drops the focus: put it back on the widget's menu.
  async function focusMenu(id: WidgetId) {
    await tick();
    document.getElementById(`widget-menu-${id}`)?.focus();
  }

  function move(id: WidgetId, by: number) {
    if (!order) return;
    order = moveWidget(order, id, order.indexOf(id) + by);
    void save();
    void focusMenu(id);
  }

  function resize(id: WidgetId, width: Width) {
    widths = { ...widths, [id]: width };
    void save();
    void focusMenu(id);
  }

  async function remove(id: WidgetId) {
    if (!order) return;
    order = order.filter((w) => w !== id);
    removed = [...removed, id];
    void save();
    await tick();
    document.getElementById("widget-add")?.focus();
  }

  function putBack(id: WidgetId) {
    if (!order) return;
    removed = removed.filter((w) => w !== id);
    order = [...order, id];
    void save();
    void focusMenu(id);
  }

  // ---- The lock, right of the search bar: locked, the widgets can't be moved or resized ----------------------------
  function toggleLock() {
    locked = !locked;
    void save();
  }
  showHeaderAction(() => (order ? lockButton : null));

  // ---- Moving by dragging the grip: the dragged widget takes the place of whichever one it enters ---------------
  let dragging = $state<WidgetId | null>(null);
  let beforeDrag: WidgetId[] = [];
  let dropped = false;
  let lastEntered: WidgetId | null = null;

  function onDragStart(event: DragEvent, id: WidgetId) {
    if (!order) return;
    [dragging, beforeDrag, dropped, lastEntered] = [id, order, false, id];
    const slot = (event.currentTarget as HTMLElement).closest(".slot");
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", id);
      if (slot) event.dataTransfer.setDragImage(slot, 24, 24);
    }
  }

  // dragenter fires for every element inside a widget too: only a different widget counts, or two would swap back.
  function onDragEnter(target: WidgetId) {
    if (!order || !dragging || target === lastEntered) return;
    lastEntered = target;
    if (target !== dragging) order = moveWidget(order, dragging, order.indexOf(target));
  }

  function onDragEnd() {
    // Escape, or a drop outside the grid, puts it back.
    if (dropped) void save();
    else order = beforeDrag;
    dragging = null;
  }

  // ---- Resizing by dragging the right edge: it snaps to whole columns as it goes ----------------------------------
  let grid: HTMLUListElement | undefined = $state();
  let resizing = $state<WidgetId | null>(null);

  function startResize(event: PointerEvent, id: WidgetId) {
    if (!grid) return;
    event.preventDefault();
    const handle = event.currentTarget as HTMLElement;
    handle.setPointerCapture(event.pointerId);
    const style = getComputedStyle(grid);
    const gap = parseFloat(style.columnGap) || 0;
    const column = (grid.clientWidth - 2 * gap) / 3;
    const left = (handle.closest(".slot") as HTMLElement).getBoundingClientRect().left;
    const before = widths[id];
    resizing = id;
    const onMove = (e: PointerEvent) => {
      const width = snapWidth(e.clientX, left, column, gap);
      if (width !== widths[id]) widths = { ...widths, [id]: width };
    };
    const onUp = () => {
      handle.removeEventListener("pointermove", onMove);
      handle.removeEventListener("pointerup", onUp);
      handle.removeEventListener("pointercancel", onUp);
      resizing = null;
      if (widths[id] !== before) void save();
    };
    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", onUp);
    handle.addEventListener("pointercancel", onUp);
  }
</script>

{#snippet lockButton()}
  <button class="icon-btn" aria-pressed={locked} aria-label={t.lock} onclick={toggleLock}>
    <i class="ti {locked ? 'ti-lock' : 'ti-lock-open'}" aria-hidden="true"></i>
  </button>
{/snippet}

<svelte:head><title>{campaigns.current?.name}</title></svelte:head>

<div class="home">
  {#if loadError}<p class="error" role="alert">{loadError}</p>{/if}
  {#if saveError}<p class="error" role="alert">{saveError}</p>{/if}

  {#if order === null}
    <p class="muted" role="status">{t.loading}</p>
  {:else}
    <ul class="grid" class:locked bind:this={grid}>
      {#each order as id, i (id)}
        <li
          class="slot"
          class:dragging={dragging === id}
          class:resizing={resizing === id}
          style:--span={widths[id]}
          animate:flip={{ duration: reduceMotion() ? 0 : 200 }}
          ondragenter={() => !locked && onDragEnter(id)}
          ondragover={(e) => dragging && e.preventDefault()}
          ondrop={(e) => {
            e.preventDefault();
            dropped = true;
          }}
        >
          <!-- No visible title: the content says what it is; screen readers hear its name. -->
          <section class="widget" aria-label={t.titles[id]}>
            {#if !locked}
              <header>
                <span class="grip" draggable="true" role="img" aria-label={t.move(t.titles[id])} ondragstart={(e) => onDragStart(e, id)} ondragend={onDragEnd}>
                  <i class="ti ti-grip-vertical" aria-hidden="true"></i>
                </span>
                <span class="spacer"></span>
                <Menu
                  id="widget-menu-{id}"
                  label={t.options(t.titles[id])}
                  triggerClass="icon-btn"
                  items={[
                    { label: t.moveLeft, disabled: i === 0, onselect: () => move(id, -1) },
                    { label: t.moveRight, disabled: i === order.length - 1, onselect: () => move(id, 1) },
                    { label: t.width, items: WIDTHS.map((w) => ({ label: t.widths[w], current: widths[id] === w, onselect: () => resize(id, w) })) },
                    { label: t.remove, onselect: () => remove(id) },
                  ]}
                >
                  {#snippet trigger()}<i class="ti ti-dots" aria-hidden="true"></i>{/snippet}
                </Menu>
              </header>
            {/if}
            <div class="body">
              {#if id === "party"}
                <PartyWidget bind:characters loadError={charactersError} />
              {:else if id === "nextSession"}
                <NextSessionWidget />
              {:else if id === "summary"}
                <SummaryWidget />
              {:else}
                <EncounterWidget {characters} width={widths[id]} />
              {/if}
            </div>
            <!-- The right edge: drag it to 1, 2 or 3 columns (the ⋯ menu's Width does the same from the keyboard). -->
            {#if !locked}<span class="edge" role="img" aria-label={t.resize(t.titles[id])} onpointerdown={(e) => startResize(e, id)}></span>{/if}
          </section>
        </li>
      {/each}
      <!-- Unlocked, with widgets taken off: a dashed slot that lists them to put back at the end. -->
      {#if !locked && removed.length > 0}
        <li class="slot add-slot">
          <Menu id="widget-add" label={t.addWidget} align="start" triggerClass="add-widget" items={removed.map((w) => ({ label: t.titles[w], onselect: () => putBack(w) }))}>
            {#snippet trigger()}<i class="ti ti-plus" aria-hidden="true"></i>{t.addWidget}{/snippet}
          </Menu>
        </li>
      {/if}
    </ul>
  {/if}
</div>

<style>
  .home {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    width: min(100%, var(--content-max-width));
    margin: 0 auto;
    padding: var(--space-5);
  }

  .error {
    margin: 0;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-4);
    align-items: start;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .slot {
    grid-column: span var(--span);
    min-width: 0;
  }

  .add-slot {
    --span: 1;
    align-self: stretch;
    display: flex;
  }

  .add-slot :global(.menu-root) {
    flex: 1;
    display: flex;
    position: relative;
  }

  .add-slot :global(.add-widget) {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    width: 100%;
    min-height: var(--control-height-lg);
    padding: var(--space-4);
    border: var(--border-width) dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    cursor: pointer;
  }

  .add-slot :global(.add-widget:hover) {
    border-color: var(--color-text);
    color: var(--color-text);
  }

  .widget {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4) var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  /* Locked: no card, the widgets sit on the page (the border stays transparent so nothing shifts). */
  .locked .widget {
    border-color: transparent;
    background: none;
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .spacer {
    flex: 1;
  }

  .grip {
    display: grid;
    place-items: center;
    margin-left: calc(-1 * var(--space-2));
    color: var(--color-text-muted);
    cursor: grab;
  }

  .grip:hover {
    color: var(--color-text);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-width: 0;
  }

  /* The dragged widget stays as an outlined gap while the others move round it. */
  .dragging .widget {
    border-style: dashed;
    border-color: var(--color-accent);
    background: var(--color-accent-soft);
  }

  .dragging .widget > * {
    opacity: 0.4;
  }

  /* The right edge: a strip that shows a green line on hover, and while it's dragged. */
  .edge {
    position: absolute;
    top: var(--space-3);
    right: calc(-1 * var(--space-2));
    bottom: var(--space-3);
    width: var(--space-4);
    cursor: ew-resize;
    touch-action: none;
  }

  .edge::after {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 3px;
    border-radius: var(--radius-sm);
    background: transparent;
    transform: translateX(-50%);
  }

  .edge:hover::after,
  .resizing .edge::after {
    background: var(--color-accent);
  }

  .resizing .widget {
    border-color: var(--color-accent);
  }

  /* Too narrow for three columns: every widget takes the full width, and there's nothing to resize. */
  @media (max-width: 900px) {
    .grid {
      grid-template-columns: minmax(0, 1fr);
    }

    .slot {
      grid-column: auto;
    }

    .edge {
      display: none;
    }
  }
</style>
