// Rich text notes are stored as a small subset of HTML. Everything else is taken out before it's shown or saved.
// The strict Content Security Policy already blocks scripts and inline event handlers; this keeps the markup tidy too.
// A tag (typed with @) is an <a> naming a Catalog entry or a character by data attributes only; its address is made
// from those when it's shown, so a stored note never carries a link of its own.
import type { SearchHit } from "$lib/api/search";
import { hitAddress } from "$lib/views/search/search";

/** Tags kept (without any attributes); other elements are replaced by their content, these are dropped with theirs. */
const KEEP = new Set(["P", "DIV", "BR", "B", "STRONG", "I", "EM", "U", "H2", "H3", "H4", "UL", "OL", "LI", "A"]);
const DROP = new Set(["SCRIPT", "STYLE", "TEMPLATE", "IFRAME", "OBJECT", "SVG", "MATH", "HEAD", "TITLE"]);
/** A tag's attributes: what it is, its pack, its index, and a character's campaign. */
const TAG_FIELDS = ["kind", "pack", "index", "campaign"] as const;
const SAFE = /^[A-Za-z0-9_.-]{1,100}$/;

export type Tag = Pick<SearchHit, "kind" | "pack" | "index" | "campaign">;

/** Older notes were plain text: escape them and keep their line breaks. */
export function toHtml(value: string): string {
  if (/<[a-z][^>]*>/i.test(value)) return value;
  return value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/\n/g, "<br>");
}

/** A tag's fields from its element, or null when it isn't a valid tag. */
function tagOf(a: Element): Tag | null {
  const [kind, pack, index, campaign] = TAG_FIELDS.map((f) => a.getAttribute(`data-${f}`));
  const ok = (v: string | null) => v === null || SAFE.test(v);
  // A character lives in a campaign, so its tag needs one.
  const valid = kind && index && SAFE.test(kind) && SAFE.test(index) && ok(pack) && ok(campaign) && (kind !== "characters" || campaign);
  return valid ? { kind, pack: pack ?? "", index, campaign } : null;
}

/** Where a tag opens: the entry in the Catalog, or the character. */
export const tagAddress = (tag: Tag, base?: string) => hitAddress({ ...tag, source: "", name: "", chapterIndex: null, chapterName: null }, base);

/**
 * The allowed subset of `value` as HTML. `links`: give each tag its address (to show it; Catalog entries in the Library
 * at `base`), else keep only its data (to store it). Needs a DOM (the window), so it isn't unit-tested.
 */
export function cleanHtml(value: string, { links = false, base }: { links?: boolean; base?: string } = {}): string {
  const doc = new DOMParser().parseFromString(`<body>${toHtml(value)}</body>`, "text/html");
  const clean = (node: Element) => {
    for (const child of [...node.children]) {
      if (DROP.has(child.tagName)) {
        child.remove();
        continue;
      }
      clean(child);
      const tag = child.tagName === "A" ? tagOf(child) : null;
      if (!KEEP.has(child.tagName) || (child.tagName === "A" && !tag)) {
        child.replaceWith(...child.childNodes);
        continue;
      }
      [...child.attributes].forEach((a) => child.removeAttribute(a.name));
      if (tag) {
        TAG_FIELDS.forEach((f) => tag[f] && child.setAttribute(`data-${f}`, tag[f]!));
        if (links) child.setAttribute("href", tagAddress(tag, base));
      }
    }
  };
  clean(doc.body);
  // Comments carry nothing a note needs.
  const walker = doc.createTreeWalker(doc.body, NodeFilter.SHOW_COMMENT);
  const comments: Node[] = [];
  while (walker.nextNode()) comments.push(walker.currentNode);
  comments.forEach((c) => c.parentNode?.removeChild(c));
  return doc.body.innerHTML;
}

/** True when the note has no text (an emptied editor leaves a stray `<br>`). */
export const isEmpty = (value: string) => !value || !new DOMParser().parseFromString(toHtml(value), "text/html").body.textContent?.trim();
