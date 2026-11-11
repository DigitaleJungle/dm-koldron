// @vitest-environment jsdom
import { Editor } from "@tiptap/core";
import { describe, expect, it } from "vitest";
import { editorExtensions } from "./editorSchema";
import { cleanHtml, isEmpty } from "./richtext";

const goblin = '<a data-kind="monsters" data-pack="srd" data-index="goblin">Goblin</a>';
const table = '<table><tbody><tr><th colspan="2" colwidth="120,0">Loot</th></tr><tr><td>Ring<br>Gold</td><td>Bell</td></tr></tbody></table>';

/** A stored note opened in the editor and saved again, as RichText.svelte does. */
function roundTrip(stored: string): string {
  const editor = new Editor({ element: document.createElement("div"), extensions: editorExtensions({ items: () => [] }), content: cleanHtml(stored) });
  const saved = cleanHtml(editor.getHTML());
  editor.destroy();
  return saved;
}

describe("cleanHtml", () => {
  it("keeps the allowed tags, a tag's data, and a cell's spans and widths, and nothing else", () => {
    const dirty = `<p onclick="x()" style="color:red">Meet ${goblin.replace("<a ", '<a href="https://evil" ')}</p><script>x()</script>`;
    expect(cleanHtml(dirty)).toBe(`<p>Meet ${goblin}</p>`);
    expect(cleanHtml(table.replace("<td>Bell", '<td onclick="x()" colwidth="9e9" rowspan="1">Bell'))).toBe(table);
    expect(cleanHtml('<table><tr><td colwidth="80" colspan="x" title="t">a</td></tr></table>')).toBe('<table><tbody><tr><td colwidth="80">a</td></tr></tbody></table>');
  });

  it("gives a tag its address and a cell its width only when shown", () => {
    const shown = cleanHtml(`<p>${goblin}</p>${table}`, { links: true });
    expect(shown).toContain('href="');
    expect(shown).toContain('style="width: 120px"');
    expect(cleanHtml('<table><tr><td colwidth="0">a</td></tr></table>', { links: true })).not.toContain("style");
  });

  it("reads older plain-text notes as text with their line breaks", () => {
    expect(cleanHtml("1 < 2\nand more")).toBe("1 &lt; 2<br>and more");
    expect(isEmpty("<p><br></p>")).toBe(true);
  });
});

describe("the editor", () => {
  it("keeps a tag, boxed text and a table with merged cells and widths through opening and saving", () => {
    const saved = roundTrip(`<p>Meet ${goblin}</p><blockquote><p>Read aloud</p></blockquote>${table}`);
    expect(saved).toContain(`<p>Meet ${goblin}`);
    expect(saved).toContain("<blockquote><p>Read aloud</p></blockquote>");
    expect(saved).toContain(table);
  });

  it("opens an older plain-text note as one paragraph with its line breaks", () => {
    expect(roundTrip("Line one\nLine two")).toBe("<p>Line one<br>Line two</p>");
  });
});
