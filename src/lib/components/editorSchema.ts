// What a rich text field may hold (RichText.svelte), as Tiptap extensions: headings 1 to 3 (h2 to h4), bold, italic,
// underline, lists, boxed text (blockquote), tables whose cells hold text and line breaks, and tags; an extended field
// also left, centre and right alignment of paragraphs and headings. Kept apart from the component so a test can load
// and save notes with exactly the same rules.
import type { AnyExtension } from "@tiptap/core";
import Mention, { type MentionOptions } from "@tiptap/extension-mention";
import { TableCell, TableHeader, TableKit } from "@tiptap/extension-table";
import TextAlign from "@tiptap/extension-text-align";
import StarterKit from "@tiptap/starter-kit";

/** A tag attribute stored as `data-<name>` on the tag's <a>. */
const data = (name: string) => ({
  default: null,
  parseHTML: (el: HTMLElement) => el.getAttribute(`data-${name}`),
  renderHTML: (attrs: Record<string, string | null>) => (attrs[name] ? { [`data-${name}`]: attrs[name] } : {}),
});

/** A tag: an <a> naming a Catalog entry or a character by data attributes only (richtext.ts gives it its address when shown). */
const Tag = Mention.extend({
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

/** The extensions; `extended` adds alignment, `suggestion` is how typing @ offers what to tag (the field's matches). */
export function editorExtensions(extended: boolean, suggestion: MentionOptions["suggestion"]): AnyExtension[] {
  return [
    ...(extended ? [TextAlign.configure({ types: ["heading", "paragraph"], alignments: ["left", "center", "right"] })] : []),
    StarterKit.configure({ heading: { levels: [2, 3, 4] }, code: false, codeBlock: false, strike: false, horizontalRule: false, link: false }),
    TableKit.configure({ table: { resizable: true }, tableCell: false, tableHeader: false }),
    inlineCell(TableCell),
    inlineCell(TableHeader),
    Tag.configure({ deleteTriggerWithBackspace: true, renderText: ({ node }) => node.attrs.label, suggestion }),
  ];
}
