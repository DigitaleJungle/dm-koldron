import type { ContentsChapter } from "$lib/api/rules";

export type Book = { pack: string; name: string; chapters: ContentsChapter[] };

/** The Rules contents by book (pack), in their order. With `search`, only the books and chapters whose name has every
 *  typed word: a matching book keeps all its chapters, another keeps only its matching ones. */
export function booksOf(contents: ContentsChapter[], search = ""): Book[] {
  const words = search.toLowerCase().split(/\s+/).filter(Boolean);
  const has = (name: string) => words.every((w) => name.toLowerCase().includes(w));
  const books: Book[] = [];
  for (const c of contents) {
    let book = books.find((b) => b.pack === c.pack);
    if (!book) books.push((book = { pack: c.pack, name: c.packName, chapters: [] }));
    book.chapters.push(c);
  }
  return books
    .map((b) => (has(b.name) ? b : { ...b, chapters: b.chapters.filter((c) => has(c.name)) }))
    .filter((b) => b.chapters.length > 0);
}
