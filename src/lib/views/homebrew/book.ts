// A homebrew book's texts as the Homebrew tab edits them: chapters (`rules` entries) with their sections in order
// (`rule-sections` entries, listed by the chapter's `subsections`). See `packs/packs.md`.
import type { Book, PackEntry } from "$lib/api/homebrew";

export type Section = { index: string; name: string; desc: string; image?: string };
export type Chapter = { index: string; name: string; desc: string; image?: string; sections: Section[] };

/** Lowercase letters, digits and dashes made from `name` (as the Rust side makes a book's id). */
export function slug(name: string): string {
  const out = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return out || "entry";
}

/** An index made from `name` that isn't in `taken`. */
export function freeIndex(name: string, taken: Set<string>): string {
  const base = slug(name);
  let index = base;
  for (let n = 2; taken.has(index); n++) index = `${base}-${n}`;
  return index;
}

const text = (value: unknown) => (typeof value === "string" ? value : "");
const picture = (value: unknown) => (typeof value === "string" && value ? { image: value } : {});
/** An entry with `image` set from the form, or left out when there's none. */
const withPicture = (entry: PackEntry, image: string | undefined): PackEntry => {
  const { image: _old, ...rest } = entry;
  return image ? { ...rest, image } : (rest as PackEntry);
};
const listed = (chapter: PackEntry) =>
  (Array.isArray(chapter.subsections) ? chapter.subsections : []).map((r: { index?: unknown }) => text(r?.index));

/** A new book's fields, before its first save. */
export function blankBook(ruleset: string, license: string): Book {
  return { id: "", name: "", version: "1.0.0", license, attribution: "", ruleset, entries: {} };
}

/** The book's chapters in order, each with its sections in order. */
export function chaptersOf(book: Book): Chapter[] {
  const sections = new Map((book.entries["rule-sections"] ?? []).map((s) => [s.index, s]));
  return (book.entries.rules ?? []).map((c) => ({
    index: c.index,
    name: c.name,
    desc: text(c.desc),
    ...picture(c.image),
    sections: listed(c).flatMap((i) => {
      const s = sections.get(i);
      return s ? [{ index: s.index, name: s.name, desc: text(s.desc), ...picture(s.image) }] : [];
    }),
  }));
}

/** The book with `chapter` saved in its place, or added at the end when its index is empty; its sections replace the
 *  ones it had. A new chapter or section gets an index made from its name, never changed after. Fields the form
 *  doesn't edit are kept. Returns the book and the chapter's index. */
export function withChapter(book: Book, chapter: Chapter): { book: Book; index: string } {
  const rules = book.entries.rules ?? [];
  const sections = book.entries["rule-sections"] ?? [];
  const old = chapter.index ? rules.find((c) => c.index === chapter.index) : undefined;
  const mine = new Set(old ? listed(old) : []);
  const others = sections.filter((s) => !mine.has(s.index));
  const kept = (s: Section) => !!s.index && mine.has(s.index);
  // Kept indexes are taken before new ones are made, wherever their sections are in the list.
  const taken = new Set([...others.map((s) => s.index), ...chapter.sections.filter(kept).map((s) => s.index)]);
  const saved = chapter.sections.map((s) => {
    const index = kept(s) ? s.index : freeIndex(s.name, taken);
    taken.add(index);
    const before = sections.find((e) => e.index === index);
    return withPicture({ ...before, index, name: s.name.trim(), desc: s.desc }, s.image);
  });
  const index = old ? old.index : freeIndex(chapter.name, new Set(rules.map((c) => c.index)));
  const entry = withPicture({ ...old, index, name: chapter.name.trim(), desc: chapter.desc, subsections: saved.map((s) => ({ index: s.index })) }, chapter.image);
  const chapters = old ? rules.map((c) => (c === old ? entry : c)) : [...rules, entry];
  return { book: { ...book, entries: { ...book.entries, rules: chapters, "rule-sections": [...others, ...saved] } }, index };
}

/** The book without that entry. */
export function withoutEntry(book: Book, kind: string, index: string): Book {
  return { ...book, entries: { ...book.entries, [kind]: (book.entries[kind] ?? []).filter((e) => e.index !== index) } };
}

/** The book without that chapter and its sections. */
export function withoutChapter(book: Book, index: string): Book {
  const rules = book.entries.rules ?? [];
  const old = rules.find((c) => c.index === index);
  const mine = new Set(old ? listed(old) : []);
  const sections = (book.entries["rule-sections"] ?? []).filter((s) => !mine.has(s.index));
  return { ...book, entries: { ...book.entries, rules: rules.filter((c) => c !== old), "rule-sections": sections } };
}
