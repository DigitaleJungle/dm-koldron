<script lang="ts">
  // A rich text field: headings in three sizes, bold, italic, underline and lists, typed in place (the window's own editing) with a small
  // toolbar; Ctrl/Cmd+B, I and U work as usual. `value` is HTML, always cleaned to the allowed tags. Pasting keeps
  // only the text. `compact` shows the toolbar only while the field has focus (for short notes in a list). `extended`
  // adds a highlighted section and a table (with tools to add and remove rows and columns while the cursor is in one).
  // Typing @ tags a Catalog entry or one of the campaign's characters: a list of matches follows the text after the
  // @ (arrow keys and Enter or Tab pick one, Escape closes it), and the pick becomes a tag that opens it when shown.
  import { onMount } from "svelte";
  import { searchAll, type SearchHit } from "$lib/api/search";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
  import { hitMeta } from "$lib/views/search/search";
  import { cleanHtml, isEmpty } from "./richtext";

  let {
    value = $bindable(""),
    label,
    compact = false,
    extended = false,
  }: { value: string; label: string; compact?: boolean; extended?: boolean } = $props();

  const t = strings.richText;
  const id = $props.id();
  let editor: HTMLDivElement | undefined = $state();
  let active = $state<Record<string, boolean>>({});

  // A tool with a `block` turns the line into that block (again: back to a paragraph). Headings 1 to 3 are h2 to h4,
  // under the page's own headings; h4 is what the single heading of older notes was, so those read as Heading 3.
  type Tool = { command: string; icon: string; label: string; block?: string };
  const BASIC: Tool[] = [
    { command: "h2", icon: "ti-h-1", label: t.heading(1), block: "h2" },
    { command: "h3", icon: "ti-h-2", label: t.heading(2), block: "h3" },
    { command: "h4", icon: "ti-h-3", label: t.heading(3), block: "h4" },
    { command: "bold", icon: "ti-bold", label: t.bold },
    { command: "italic", icon: "ti-italic", label: t.italic },
    { command: "underline", icon: "ti-underline", label: t.underline },
    { command: "insertUnorderedList", icon: "ti-list", label: t.bullets },
    { command: "insertOrderedList", icon: "ti-list-numbers", label: t.numbers },
  ];
  const EXTRA: Tool[] = [
    { command: "highlight", icon: "ti-highlight", label: t.highlight, block: "blockquote" },
    { command: "table", icon: "ti-table-plus", label: t.table },
  ];
  // svelte-ignore state_referenced_locally
  const TOOLS = extended ? [...BASIC, ...EXTRA] : BASIC;
  /** Shown while the cursor is in a table. */
  const TABLE_TOOLS = [
    { id: "row", icon: "ti-row-insert-bottom", label: t.addRow },
    { id: "column", icon: "ti-column-insert-right", label: t.addColumn },
    { id: "removeRow", icon: "ti-row-remove", label: t.removeRow },
    { id: "removeColumn", icon: "ti-column-remove", label: t.removeColumn },
  ] as const;
  let cell = $state<HTMLTableCellElement | null>(null);

  // Tags can't be edited letter by letter: they're one piece, removed as a whole.
  const lockTags = () => editor?.querySelectorAll("a[data-kind]").forEach((a) => ((a as HTMLElement).contentEditable = "false"));

  // Filled once; after that the field owns its content and only reports it back.
  onMount(() => {
    if (!editor) return;
    editor.innerHTML = cleanHtml(value);
    lockTags();
  });

  export const focus = () => editor?.focus();

  function changed() {
    // An emptied field leaves a stray <br>; it counts as empty, so clearing a note isn't a change.
    if (editor) value = isEmpty(editor.innerHTML) ? "" : cleanHtml(editor.innerHTML);
    update();
    findTag();
  }

  // shortcut: document.execCommand is deprecated but the only built-in rich editing; WebView2 and WebKit both support it.
  function run(tool: Tool) {
    editor?.focus();
    if (tool.command === "table") {
      // A header row and one row of two columns, then a line to go on typing after it.
      document.execCommand("insertHTML", false, "<table><tbody><tr><th><br></th><th><br></th></tr><tr><td><br></td><td><br></td></tr></tbody></table><p><br></p>");
    } else if (tool.block) document.execCommand("formatBlock", false, active[tool.command] ? "p" : tool.block);
    else document.execCommand(tool.command);
    changed();
  }

  /** The table cell the cursor is in, if any. */
  function cellAtCursor(): HTMLTableCellElement | null {
    const node = document.getSelection()?.anchorNode ?? null;
    const found = (node instanceof Element ? node : node?.parentElement)?.closest("td, th");
    return found && editor?.contains(found) ? (found as HTMLTableCellElement) : null;
  }

  const blankCell = (tag: "td" | "th") => Object.assign(document.createElement(tag), { innerHTML: "<br>" });

  function tableTool(id: (typeof TABLE_TOOLS)[number]["id"]) {
    const at = cell;
    const row = at?.parentElement as HTMLTableRowElement | null;
    const table = at?.closest("table");
    if (!at || !row || !table) return;
    const rows = [...table.rows];
    if (id === "row") row.after(newRow(row));
    else if (id === "column") rows.forEach((r) => r.insertBefore(blankCell(r.cells[0]?.tagName === "TH" ? "th" : "td"), r.cells[at.cellIndex + 1] ?? null));
    else if (id === "removeRow") row.remove();
    else rows.forEach((r) => r.cells[at.cellIndex]?.remove());
    // A table with no rows or no columns left goes.
    if (!table.rows.length || ![...table.rows].some((r) => r.cells.length)) table.remove();
    editor?.focus();
    changed();
  }

  /** An empty row like `row` (body cells, also under the header row). */
  function newRow(row: HTMLTableRowElement): HTMLTableRowElement {
    const next = document.createElement("tr");
    for (let i = 0; i < row.cells.length; i++) next.appendChild(blankCell("td"));
    return next;
  }

  // Which tools are on where the cursor is.
  function update() {
    if (!editor || !editor.contains(document.getSelection()?.anchorNode ?? null)) return;
    const block = String(document.queryCommandValue("formatBlock")).toLowerCase();
    const anchor = document.getSelection()?.anchorNode ?? null;
    const inQuote = !!(anchor instanceof Element ? anchor : anchor?.parentElement)?.closest("blockquote");
    active = Object.fromEntries(
      TOOLS.map((tool) => [
        tool.command,
        tool.command === "highlight" ? inQuote : tool.command === "table" ? false : tool.block ? block === tool.block : document.queryCommandState(tool.command),
      ]),
    );
    cell = extended ? cellAtCursor() : null;
  }

  function paste(e: ClipboardEvent) {
    e.preventDefault();
    document.execCommand("insertText", false, e.clipboardData?.getData("text/plain") ?? "");
  }

  // ---- Tags ------------------------------------------------------------------------------------
  /** The @ being typed: its text node and place, and where to show the matches. */
  let tagging = $state<{ node: Text; at: number; query: string; top: number; left: number } | null>(null);
  let hits = $state<SearchHit[]>([]);
  let hitsFor = $state<string | null>(null); // the query the shown hits are for
  let searchError = $state<string | null>(null);
  let current = $state(0);
  let asked = 0; // answers to older keystrokes are dropped

  // An @ at the start or after a space, then up to 40 letters without another @, right before the cursor.
  function findTag() {
    const selection = document.getSelection();
    const node = selection?.anchorNode;
    if (!selection?.isCollapsed || !(node instanceof Text) || !editor?.contains(node)) return void (tagging = null);
    const before = node.data.slice(0, selection.anchorOffset);
    const match = /(?:^|\s)@([^@\n]{0,40})$/.exec(before);
    if (!match) return void (tagging = null);
    const at = before.length - match[1].length - 1;
    const range = document.createRange();
    range.setStart(node, at);
    range.setEnd(node, at + 1);
    const box = range.getBoundingClientRect();
    tagging = { node, at, query: match[1], top: box.bottom + 4, left: box.left };
    void search(match[1]);
  }

  async function search(query: string) {
    if (query === hitsFor) return;
    const mine = ++asked;
    if (!query.trim()) return void ([hits, hitsFor, searchError] = [[], query, null]);
    try {
      // The campaign's characters and every switched-on pack; rule sections aren't things to tag.
      const found = (await searchAll(query, 12, campaigns.scope ?? undefined)).filter((h) => h.kind !== "rule-sections").slice(0, 8);
      if (mine === asked) [hits, hitsFor, searchError, current] = [found, query, null, 0];
    } catch (e) {
      if (mine === asked) [hits, hitsFor, searchError] = [[], query, errorMessage(e)];
    }
  }

  function insertTag(hit: SearchHit) {
    const selection = document.getSelection();
    if (!tagging || !selection?.anchorNode) return;
    const range = document.createRange();
    range.setStart(tagging.node, tagging.at);
    range.setEnd(selection.anchorNode, selection.anchorOffset);
    range.deleteContents();
    const tag = document.createElement("a");
    Object.entries({ kind: hit.kind, pack: hit.pack, index: hit.index, campaign: hit.campaign ?? "" }).forEach(([k, v]) => v && (tag.dataset[k] = v));
    tag.textContent = hit.name;
    tag.contentEditable = "false";
    const space = document.createTextNode(" "); // so typing goes on after the tag, not inside it
    range.insertNode(space);
    range.insertNode(tag);
    range.setStartAfter(space);
    range.collapse(true);
    selection.removeAllRanges();
    selection.addRange(range);
    tagging = null;
    changed();
  }

  function keydown(e: KeyboardEvent) {
    if (!tagging) return;
    if (e.key === "Escape") {
      e.preventDefault();
      tagging = null;
    } else if (hits.length && hitsFor === tagging.query && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
      e.preventDefault();
      current = (current + (e.key === "ArrowDown" ? 1 : hits.length - 1)) % hits.length;
    } else if (hits.length && hitsFor === tagging.query && (e.key === "Enter" || e.key === "Tab")) {
      e.preventDefault();
      insertTag(hits[current]);
    }
  }
</script>

<svelte:document onselectionchange={update} />

<div class="rich" class:compact>
  <div class="toolbar" role="toolbar" aria-label={t.toolbar(label)}>
    {#each TOOLS as tool (tool.command)}
      <!-- mousedown is cancelled so the field keeps its selection -->
      <button class="icon-btn tool" aria-label={tool.label} aria-pressed={tool.command === "table" ? undefined : !!active[tool.command]} onmousedown={(e) => e.preventDefault()} onclick={() => run(tool)}>
        <i class="ti {tool.icon}" aria-hidden="true"></i>
      </button>
    {/each}
    {#if cell}
      <span class="divider" aria-hidden="true"></span>
      {#each TABLE_TOOLS as tool (tool.id)}
        <button class="icon-btn tool" aria-label={tool.label} onmousedown={(e) => e.preventDefault()} onclick={() => tableTool(tool.id)}>
          <i class="ti {tool.icon}" aria-hidden="true"></i>
        </button>
      {/each}
    {/if}
  </div>
  <div
    bind:this={editor}
    class="editor prose"
    contenteditable="true"
    role="textbox"
    aria-multiline="true"
    aria-label={label}
    aria-controls="{id}-tags"
    aria-activedescendant={tagging && hits.length ? `${id}-tag-${current}` : undefined}
    tabindex="0"
    oninput={changed}
    onpaste={paste}
    onkeydown={keydown}
    onfocusout={() => (tagging = null)}
  ></div>
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
        <li id="{id}-tag-{i}" role="option" aria-selected={i === current} onmousedown={(e) => e.preventDefault()} onclick={() => insertTag(hit)} onmousemove={() => (current = i)}>
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

  .divider {
    align-self: stretch;
    width: var(--border-width);
    margin: var(--space-1);
    background: var(--color-border);
  }

  /* A highlighted section stands out from the field it's in (.prose gives it the field's own colour). */
  .editor :global(blockquote) {
    background: var(--color-accent-soft);
  }

  /* Every cell outlined while editing, so an empty table can be seen and clicked into. */
  .editor :global(th),
  .editor :global(td) {
    min-width: var(--space-8);
    border: var(--border-width) solid var(--color-border);
  }

  /* Short notes in a list show their toolbar only while being written. */
  .compact:not(:focus-within) .toolbar {
    display: none;
  }

  /* .prose gives the text its look; its reading width and centring (margin: auto) don't belong in a field. */
  .editor {
    flex: 1;
    max-width: none;
    margin: 0;
    min-height: calc(3 * var(--text-md) * var(--leading-normal));
    padding: var(--space-2) var(--space-3);
    overflow-wrap: anywhere;
    outline: none;
  }

  .rich:not(.compact) .editor {
    min-height: calc(8 * var(--text-md) * var(--leading-normal));
  }

  .compact .editor {
    min-height: calc(2 * var(--text-md) * var(--leading-normal));
  }

  .editor :global(:first-child) {
    margin-top: 0;
  }

  .editor :global(:last-child) {
    margin-bottom: 0;
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
