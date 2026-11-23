// What a rich text field may hold (RichText.svelte), as Tiptap extensions: headings 1 to 3 (h2 to h4), bold, italic,
// underline, lists, boxed text (blockquote), tables whose cells hold text and line breaks, and tags; an extended field
// also left, centre and right alignment of paragraphs and headings. Kept apart from the component so a test can load
// and save notes with exactly the same rules.
import type { AnyExtension } from "@tiptap/core";
import Mention, { type MentionOptions } from "@tiptap/extension-mention";
import { TableCell, TableHeader, TableKit } from "@tiptap/extension-table";
import TextAlign from "@tiptap/extension-text-align";
import StarterKit from "@tiptap/starter-kit";
import { Markdown } from "@tiptap/markdown";

/** A tag attribute stored as `data-<name>` on the tag's <a>. */
const data = (name: string) => ({
  default: null,
  parseHTML: (el: HTMLElement) => el.getAttribute(`data-${name}`),
  renderHTML: (attrs: Record<string, string | null>) => (attrs[name] ? { [`data-${name}`]: attrs[name] } : {}),
});

/** A tag in Markdown (a pack's text): a link to `koldron:<kind>/<pack>/<index>`, as Rust's `render` reads it. */
const TAG_URL = /^koldron:([A-Za-z0-9_.-]{1,100})\/([A-Za-z0-9_.-]{1,100})\/([A-Za-z0-9_.-]{1,100})$/;
/** Brackets and backslashes in a tag's text, escaped so they can't end its link. */
const escapeLabel = (label: string) => label.replace(/[\\[\]]/g, (c) => `\\${c}`);

/** A tag: an <a> naming a Catalog entry or a character by data attributes only (richtext.ts gives it its address when shown). */
const Tag = Mention.extend({
  // In Markdown a tag is a link to its entry; any other link is kept as its text (a pack's text has no links).
  markdownTokenName: "link",
  parseMarkdown: (token, h) => {
    const m = TAG_URL.exec(typeof token.href === "string" ? token.href : "");
    // The label from its inline tokens, so escaped brackets come back as they were typed.
    const label = (token.tokens ?? []).map((t) => t.text ?? "").join("");
    return m ? { type: "mention", attrs: { kind: m[1], pack: m[2], index: m[3], label } } : h.parseInline(token.tokens ?? []);
  },
  renderMarkdown: (node) => {
    const { kind, pack, index, label } = node.attrs ?? {};
    return `[${escapeLabel(String(label ?? ""))}](koldron:${kind}/${pack}/${index})`;
  },
  addAttributes: () => ({
    kind: data("kind"),
    pack: data("pack"),
    index: data("index"),
    campaign: data("campaign"),
    label: { default: "", parseHTML: (el: HTMLElement) => el.textContent ?? "", renderHTML: () => ({}) },
    mentionSuggestionChar: { default: "@", renderHTML: () => ({}) },
  }),
  parseHTML: () => [{ tag: "a[data-kind]" }],
  renderHTML: ({ node, HTMLAttributes }) => ["a", HTMLAttributes, node.attrs.label],
});

/** A cell holds text and line breaks, not paragraphs (<td>text<br>new line</td>); Enter in a cell adds a line break. */
const inlineCell = <T extends typeof TableCell | typeof TableHeader>(cell: T) =>
  cell.extend({
    content: "inline*",
    addKeyboardShortcuts() {
      return { Enter: () => this.editor.isActive(this.name) && this.editor.commands.setHardBreak() };
    },
  });

/** The extensions; `extended` adds alignment, `suggestion` is how typing @ offers what to tag (the field's matches).
 *  `markdown`: read and written as Markdown (a pack's text), which has no underline or alignment; a tag is a link. */
export function editorExtensions(extended: boolean, suggestion: MentionOptions["suggestion"], markdown = false): AnyExtension[] {
  const kit = { heading: { levels: [2, 3, 4] as (2 | 3 | 4)[] }, code: false, codeBlock: false, strike: false, horizontalRule: false, link: false } as const;
  if (markdown) {
    return [
      StarterKit.configure({ ...kit, underline: false }),
      TableKit.configure({ table: { resizable: true }, tableCell: false, tableHeader: false }),
      // A Markdown table cell is one line: text only, no line breaks (they'd be saved as raw <br>, shown as text).
      TableCell.extend({ content: "text*" }),
      TableHeader.extend({ content: "text*" }),
      Tag.configure({ deleteTriggerWithBackspace: true, renderText: ({ node }) => node.attrs.label, suggestion }),
      Markdown,
    ];
  }
  return [
    ...(extended ? [TextAlign.configure({ types: ["heading", "paragraph"], alignments: ["left", "center", "right"] })] : []),
    StarterKit.configure(kit),
    TableKit.configure({ table: { resizable: true }, tableCell: false, tableHeader: false }),
    inlineCell(TableCell),
    inlineCell(TableHeader),
    Tag.configure({ deleteTriggerWithBackspace: true, renderText: ({ node }) => node.attrs.label, suggestion }),
  ];
}
