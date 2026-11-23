<script lang="ts">
  // A rich text field on Tiptap: headings in three sizes, bold, italic, underline and lists, with a small toolbar;
  // Ctrl/Cmd+B, I and U, and Ctrl/Cmd+Z to undo, work as usual. `value` is HTML, always cleaned to the allowed tags.
  // Pasting keeps only the text. `compact` shows the toolbar only while the field has focus (for short notes in a list).
  // `extended` turns the headings into a text style dropdown (Paragraph, Heading 1 to 3), adds left, centre and right
  // alignment, boxed text (a quote, read-aloud text, a note) and a table; inside a table, its button is a dropdown
  // of the table's actions (rows, columns, header row and column, merging or splitting cells). Typing @ tags a Catalog
  // entry or one of the campaign's characters: a list of matches follows the text after the @ (arrow keys and Enter or Tab pick one, Escape closes it), and the pick becomes a tag that opens it when
  // shown. A tag is one piece: Backspace removes it whole. `markdown` (with `extended`): `value` is Markdown (a pack's
  // text), without underline or alignment, which Markdown can't hold; its tags are Catalog entries only (a pack can't
  // name a campaign's character), stored as links to them.
  import { onDestroy, onMount } from "svelte";
  import "$lib/styles/prose.css"; // the note's look (.prose.notes) wherever the field is, not only where a screen loads it
  import Menu, { type MenuItem } from "./Menu.svelte";
  import { Editor, type ChainedCommands } from "@tiptap/core";
  import type { MentionNodeAttrs } from "@tiptap/extension-mention";
  import { columnResizingPluginKey, TableMap } from "@tiptap/pm/tables";
  import type { EditorView } from "@tiptap/pm/view";
  import type { SuggestionKeyDownProps, SuggestionProps } from "@tiptap/suggestion";
  import { searchAll, type SearchHit } from "$lib/api/search";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
  import { hitMeta } from "$lib/views/search/search";
  import { editorExtensions } from "./editorSchema";
  import { cleanHtml } from "./richtext";

  let {
    value = $bindable(""),
    label,
    compact = false,
    extended = false,
    markdown = false,
  }: { value: string; label: string; compact?: boolean; extended?: boolean; markdown?: boolean } = $props();

  const t = strings.richText;
  const id = $props.id();
  let host: HTMLDivElement | undefined = $state();
  let editor = $state.raw<Editor | null>(null);
  /** Bumped on every change and cursor move, so the toolbar shows what's on where the cursor is. */
  let version = $state(0);
  const is = (name: string | Record<string, unknown>, attrs?: Record<string, unknown>) =>
    (void version, (typeof name === "string" ? editor?.isActive(name, attrs) : editor?.isActive(name)) ?? false);

  type Tool = { id: string; icon: string; label: string; run: (c: ChainedCommands) => ChainedCommands; active?: () => boolean };
  // Headings 1 to 3 are h2 to h4, under the page's own headings; h4 is what the single heading of older notes was.
  const HEADINGS: Tool[] = [
    { id: "h2", icon: "ti-h-1", label: t.heading(1), run: (c) => c.toggleHeading({ level: 2 }), active: () => is("heading", { level: 2 }) },
    { id: "h3", icon: "ti-h-2", label: t.heading(2), run: (c) => c.toggleHeading({ level: 3 }), active: () => is("heading", { level: 3 }) },
    { id: "h4", icon: "ti-h-3", label: t.heading(3), run: (c) => c.toggleHeading({ level: 4 }), active: () => is("heading", { level: 4 }) },
  ];
  /** The text style dropdown (extended): the same headings, set rather than toggled, and Paragraph to go back. */
  const STYLES: Tool[] = [
    { id: "p", icon: "ti-letter-p", label: t.paragraph, run: (c) => c.setParagraph(), active: () => is("paragraph") },
    { id: "h2", icon: "ti-h-1", label: t.heading(1), run: (c) => c.setHeading({ level: 2 }), active: () => is("heading", { level: 2 }) },
    { id: "h3", icon: "ti-h-2", label: t.heading(2), run: (c) => c.setHeading({ level: 3 }), active: () => is("heading", { level: 3 }) },
    { id: "h4", icon: "ti-h-3", label: t.heading(3), run: (c) => c.setHeading({ level: 4 }), active: () => is("heading", { level: 4 }) },
  ];
  /** Left is the default, so it clears the alignment rather than storing one. */
  const ALIGN: Tool[] = [
    { id: "left", icon: "ti-align-left", label: t.alignLeft, run: (c) => c.unsetTextAlign(), active: () => !is({ textAlign: "center" }) && !is({ textAlign: "right" }) },
    { id: "center", icon: "ti-align-center", label: t.alignCenter, run: (c) => c.setTextAlign("center"), active: () => is({ textAlign: "center" }) },
    { id: "right", icon: "ti-align-right", label: t.alignRight, run: (c) => c.setTextAlign("right"), active: () => is({ textAlign: "right" }) },
  ];
  const INLINE: Tool[] = [
    { id: "bold", icon: "ti-bold", label: t.bold, run: (c) => c.toggleBold(), active: () => is("bold") },
    { id: "italic", icon: "ti-italic", label: t.italic, run: (c) => c.toggleItalic(), active: () => is("italic") },
    { id: "underline", icon: "ti-underline", label: t.underline, run: (c) => c.toggleUnderline(), active: () => is("underline") },
    { id: "bullets", icon: "ti-list", label: t.bullets, run: (c) => c.toggleBulletList(), active: () => is("bulletList") },
    { id: "numbers", icon: "ti-list-numbers", label: t.numbers, run: (c) => c.toggleOrderedList(), active: () => is("orderedList") },
  ];
  const EXTRA: Tool[] = [
    { id: "boxed", icon: "ti-blockquote", label: t.boxed, run: (c) => c.toggleBlockquote(), active: () => is("blockquote") },
    // A header row and one row of two columns.
    { id: "table", icon: "ti-table-plus", label: t.table, run: (c) => c.insertTable({ rows: 2, cols: 2, withHeaderRow: true }) },
  ];
  /** The table button's dropdown while the cursor is in a table; removing the last row or column removes the table. */
  type Action = { label: string; icon: string; run: (c: ChainedCommands) => ChainedCommands };
  const TABLE_ACTIONS: Action[] = [
    { label: t.addRow, icon: "ti-row-insert-bottom", run: (c) => c.addRowAfter() },
    { label: t.addColumn, icon: "ti-column-insert-right", run: (c) => c.addColumnAfter() },
    { label: t.removeRow, icon: "ti-row-remove", run: (c) => c.deleteRow() },
    { label: t.removeColumn, icon: "ti-column-remove", run: (c) => c.deleteColumn() },
    { label: t.headerRow, icon: "ti-table-row", run: (c) => c.toggleHeaderRow() },
    { label: t.headerColumn, icon: "ti-table-column", run: (c) => c.toggleHeaderColumn() },
  ];
  /** One action that merges the selected cells, or splits a merged one: named for what it will do. */
  function mergeOrSplit(): Action {
    const can = editor?.can();
    return can?.splitCell() && !can.mergeCells()
      ? { label: t.splitCell, icon: "ti-arrows-split", run: (c) => c.splitCell() }
      : { label: t.mergeCells, icon: "ti-arrows-join", run: (c) => c.mergeCells() };
  }
  // svelte-ignore state_referenced_locally
  const TOOLS = markdown ? [...INLINE.filter((tool) => tool.id !== "underline"), ...EXTRA] : extended ? [...ALIGN, ...INLINE, ...EXTRA] : [...HEADINGS, ...INLINE];
  /** The style the cursor is in, if any (a table cell has none: it holds plain text). */
  const currentStyle = $derived(STYLES.find((s) => s.active?.()) ?? null);
  const styleItems = $derived<MenuItem[]>(
    STYLES.map((s) => ({ label: s.label, icon: s.icon, current: s === currentStyle, disabled: !can(s), onselect: () => run(s) })),
  );
  const inTable = $derived(extended && is("table"));

  const run = (tool: Pick<Tool, "run">) => editor && tool.run(editor.chain().focus()).run();
  /** Whether the tool can do anything where the cursor is (a heading or an alignment can't in a table cell). */
  const can = (tool: Pick<Tool, "run">) => (void version, !!editor && tool.run(editor.can().chain()).run());
  // An action that can't apply where the cursor is (merging a single cell, say) is greyed out.
  const tableItems = $derived<MenuItem[]>(
    (void version,
    [...TABLE_ACTIONS, mergeOrSplit()].map((a) => ({ label: a.label, icon: a.icon, onselect: () => run(a), disabled: !editor || !a.run(editor.can().chain()).run() }))),
  );

  // ---- Tags ------------------------------------------------------------------------------------
  /** The @ being typed: what follows it, where to show the matches, and how to pick one. */
  let tagging = $state<{ query: string; top: number; left: number; pick: (hit: SearchHit) => void } | null>(null);
  let hits = $state<SearchHit[]>([]);
  let hitsFor = $state<string | null>(null); // the query the shown hits are for
  let searchError = $state<string | null>(null);
  let current = $state(0);
  let asked = 0; // answers to older keystrokes are dropped

  async function search(query: string) {
    if (query === hitsFor) return;
    const mine = ++asked;
    if (!query.trim()) return void ([hits, hitsFor, searchError] = [[], query, null]);
    try {
      // The campaign's characters and every switched-on pack; rule sections aren't things to tag.
      const found = (await searchAll(query, 12, campaigns.scope ?? undefined)).filter((h) => h.kind !== "rule-sections" && (!markdown || (h.kind !== "characters" && !h.kind.startsWith("addon:")))).slice(0, 8);
      if (mine === asked) [hits, hitsFor, searchError, current] = [found, query, null, 0];
    } catch (e) {
      if (mine === asked) [hits, hitsFor, searchError] = [[], query, errorMessage(e)];
    }
  }

  function showTagging(props: SuggestionProps<unknown, MentionNodeAttrs>) {
    const box = props.clientRect?.();
    const pick = (hit: SearchHit) => {
      // Mention's id and label, plus a tag's own attributes (see editorSchema.ts; the id isn't kept).
      const attrs = { id: `${hit.kind}/${hit.pack}/${hit.index}`, label: hit.name, kind: hit.kind, pack: hit.pack, index: hit.index, campaign: hit.campaign ?? null };
      props.command(attrs);
    };
    tagging = { query: props.query, top: (box?.bottom ?? 0) + 4, left: box?.left ?? 0, pick };
    void search(props.query);
  }

  function tagKey({ event }: SuggestionKeyDownProps): boolean {
    if (!tagging) return false;
    const ready = hits.length > 0 && hitsFor === tagging.query;
    if (event.key === "Escape") tagging = null;
    else if (ready && (event.key === "ArrowDown" || event.key === "ArrowUp")) current = (current + (event.key === "ArrowDown" ? 1 : hits.length - 1)) % hits.length;
    else if (ready && (event.key === "Enter" || event.key === "Tab")) tagging.pick(hits[current]);
    else return false;
    return true;
  }

  /** Double-clicking a column's edge clears that column's width, so it sizes itself again; false when not on an edge. */
  function autoWidth(view: EditorView): boolean {
    const handle = columnResizingPluginKey.getState(view.state)?.activeHandle ?? -1;
    if (handle < 0) return false;
    const resolved = view.state.doc.resolve(handle);
    const table = resolved.node(-1);
    const start = resolved.start(-1);
    const map = TableMap.get(table);
    const edge = resolved.nodeAfter;
    if (!edge) return false;
    const col = map.colCount(resolved.pos - start) + edge.attrs.colspan - 1; // the column whose right edge it is
    const tr = view.state.tr;
    for (let row = 0; row < map.height; row++) {
      const index = row * map.width + col;
      if (row > 0 && map.map[index] === map.map[index - map.width]) continue; // a cell spanning rows: done already
      const pos = map.map[index];
      const cell = table.nodeAt(pos);
      const widths: number[] | null = cell?.attrs.colwidth ?? null;
      if (!cell || !widths) continue;
      const next = widths.slice();
      next[cell.attrs.colspan === 1 ? 0 : col - map.colCount(pos)] = 0; // 0: no width of its own (as prosemirror-tables counts it)
      tr.setNodeMarkup(start + pos, null, { ...cell.attrs, colwidth: next.some((w) => w > 0) ? next : null });
    }
    if (tr.docChanged) view.dispatch(tr);
    return true;
  }

  // ---- The editor --------------------------------------------------------------------------------
  let pastingText = false;

  // Filled once; after that the field owns its content and only reports it back.
  onMount(() => {
    editor = new Editor({
      element: host!,
      content: markdown ? value : cleanHtml(value),
      ...(markdown ? { contentType: "markdown" as const } : {}),
      extensions: editorExtensions(extended, {
        allowSpaces: true,
        items: () => [], // the matches come from search(), so older answers can be dropped
        render: () => ({
          onStart: showTagging,
          onUpdate: showTagging,
          onKeyDown: tagKey,
          onExit: () => (tagging = null),
        }),
      }, markdown),
      editorProps: {
        attributes: { class: "editor prose notes", role: "textbox", "aria-multiline": "true", "aria-label": label, "aria-controls": `${id}-tags` },
        handleDoubleClick: (view) => autoWidth(view),
        // Only the text: pasted HTML would bring its own look.
        handlePaste: (view, event) => {
          if (pastingText) return false;
          pastingText = true;
          view.pasteText(event.clipboardData?.getData("text/plain") ?? "");
          pastingText = false;
          return true;
        },
      },
      onUpdate: ({ editor: e }) => {
        value = e.isEmpty ? "" : markdown ? e.getMarkdown() : cleanHtml(e.getHTML());
        version++;
      },
      onSelectionUpdate: () => version++,
      onBlur: () => (tagging = null),
    });
  });

  onDestroy(() => editor?.destroy());

  // The highlighted match, for screen readers.
  $effect(() => {
    const dom = editor?.view.dom;
    if (!dom) return;
    if (tagging && hits.length) dom.setAttribute("aria-activedescendant", `${id}-tag-${current}`);
    else dom.removeAttribute("aria-activedescendant");
  });
</script>

<div class="rich" class:compact>
  <div class="toolbar" role="toolbar" aria-label={t.toolbar(label)}>
    {#if extended}
      <Menu id="{id}-style" label={t.textStyle} items={styleItems} align="start" triggerClass="icon-btn tool dropdown">
        {#snippet trigger()}
          <i class="ti {(currentStyle ?? STYLES[0]).icon}" aria-hidden="true"></i>
          <i class="ti ti-chevron-down caret" aria-hidden="true"></i>
        {/snippet}
      </Menu>
    {/if}
    {#each TOOLS as tool (tool.id)}
      {#if tool.id === "table" && inTable}
        <!-- In a table, the table button is a dropdown of what can be done to it. -->
        <Menu id="{id}-table" label={t.tableActions} items={tableItems} align="start" triggerClass="icon-btn tool dropdown">
          {#snippet trigger()}
            <i class="ti ti-table" aria-hidden="true"></i>
            <i class="ti ti-chevron-down caret" aria-hidden="true"></i>
          {/snippet}
        </Menu>
      {:else}
        <!-- mousedown is cancelled so the field keeps its selection -->
        <!-- greyed out (and never shown pressed) where it can't apply -->
        {@const usable = can(tool)}
        <button class="icon-btn tool" aria-label={tool.label} aria-pressed={tool.active ? usable && tool.active() : undefined} disabled={!usable} onmousedown={(e) => e.preventDefault()} onclick={() => run(tool)}>
          <i class="ti {tool.icon}" aria-hidden="true"></i>
        </button>
      {/if}
    {/each}
  </div>
  <div class="host" bind:this={host}></div>
</div>

{#if tagging}
  <div class="tags popover" style:top="{tagging.top}px" style:left="{tagging.left}px">
    {#if searchError}
      <p class="message" role="alert">{searchError}</p>
    {:else if !tagging.query.trim()}
      <p class="message">{t.tagHint}</p>
    {:else if hitsFor === tagging.query && hits.length === 0}
      <p class="message" role="status">{t.noTags}</p>
    {/if}
    <ul id="{id}-tags" role="listbox" aria-label={t.tags}>
      {#each hits as hit, i (`${hit.kind}/${hit.pack}/${hit.index}`)}
        <!-- svelte-ignore a11y_click_events_have_key_events: the field's arrow keys and Enter drive the options -->
        <li id="{id}-tag-{i}" role="option" aria-selected={i === current} onmousedown={(e) => e.preventDefault()} onclick={() => tagging?.pick(hit)} onmousemove={() => (current = i)}>
          <span class="name">{hit.name}</span>
          <span class="meta">{hitMeta(hit)}</span>
        </li>
      {/each}
    </ul>
  </div>
{/if}

<style>
  .rich {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md); /* like every other field */
    background: var(--color-surface);
  }

  .rich:focus-within {
    outline: var(--focus-ring) solid var(--color-focus);
    outline-offset: 2px;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-1);
    border-bottom: var(--border-width) solid var(--color-border);
  }

  /* A tool that can't apply where the cursor is (a heading or an alignment in a table cell). */
  .toolbar :global(.tool:disabled) {
    color: var(--color-border-strong);
    cursor: not-allowed;
  }

  /* A dropdown button (text style, table) with the small arrow that marks it: as wide as both. */
  .toolbar :global(.dropdown) {
    width: auto;
    padding: 0 var(--space-1);
  }

  .toolbar :global(.caret) {
    margin-left: calc(-1 * var(--space-1));
    font-size: var(--icon-size-sm);
  }

  /* Short notes in a list show their toolbar only while being written. */
  .compact:not(:focus-within) .toolbar {
    display: none;
  }

  .host {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  /* The editable area Tiptap makes. .prose gives the text its look; its reading width and centring (margin: auto)
     don't belong in a field. */
  .host :global(.editor) {
    flex: 1;
    max-width: none;
    margin: 0;
    min-height: calc(3 * var(--text-md) * var(--leading-normal));
    padding: var(--space-2) var(--space-3);
    overflow-wrap: anywhere;
    outline: none;
  }

  .rich:not(.compact) .host :global(.editor) {
    min-height: calc(8 * var(--text-md) * var(--leading-normal));
  }

  .compact .host :global(.editor) {
    min-height: calc(2 * var(--text-md) * var(--leading-normal));
  }

  .host :global(.editor > :first-child) {
    margin-top: 0;
  }

  .host :global(.editor > :last-child) {
    margin-bottom: 0;
  }

  /* Boxed text: set apart from the text around it by a bar on the left, for a quote, read-aloud text or a note. */
  .host :global(blockquote) {
    margin: var(--space-3) 0;
    padding: var(--space-3) var(--space-4);
    border: 0;
    border-left: var(--border-width-thick) solid var(--color-border-strong);
    border-radius: 0;
    background: var(--color-bg);
  }

  /* Every cell outlined while editing, so an empty table can be seen and clicked into; cells picked together tinted. */
  .host :global(th),
  .host :global(td) {
    min-width: var(--space-8);
    border: var(--border-width) solid var(--color-border);
  }

  /* Columns are resized by dragging a cell's right edge (Tiptap's own handle; prosemirror-tables' base styles, here
     with the app's colours). A table wider than the field scrolls sideways. */
  .host :global(.tableWrapper) {
    overflow-x: auto;
  }

  .host :global(.editor table) {
    table-layout: fixed;
  }

  .host :global(.editor td),
  .host :global(.editor th) {
    position: relative;
    box-sizing: border-box;
  }

  .host :global(.column-resize-handle) {
    position: absolute;
    top: 0;
    right: calc(-1 * var(--border-width-thick));
    bottom: 0;
    z-index: 1;
    width: calc(2 * var(--border-width-thick));
    background: var(--color-accent);
    pointer-events: none;
  }

  .host :global(.resize-cursor) {
    cursor: col-resize;
  }

  /* Cells touch: one shared border between them, no spacing. */
  .host :global(table) {
    border-collapse: collapse;
    border-spacing: 0;
  }


  .host :global(.selectedCell) {
    background: var(--color-accent-soft);
  }

  /* The matches for an @ (a .popover), under it; fixed, so a scrolling panel doesn't cut it off. */
  .tags {
    position: fixed;
    z-index: var(--z-menu);
    width: var(--select-width);
    max-width: calc(100vw - 2 * var(--space-4));
  }

  .tags ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .tags li {
    display: flex;
    flex-direction: column;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .tags li[aria-selected="true"] {
    background: var(--color-accent-soft);
  }

  .meta,
  .message {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .message {
    margin: 0;
    padding: var(--space-2) var(--space-3);
  }
</style>
