import { invoke } from "@tauri-apps/api/core";

/** A pack entry as the pack schema has it (`packs/packs.md`). */
export type PackEntry = Record<string, unknown> & { index: string; name: string };

/** Mirrors `Book` in src-tauri/src/commands/homebrew.rs: a homebrew book's manifest fields and its entries per type. */
export type Book = {
  /** Empty for a new book: made from the name on its first save. */
  id: string;
  name: string;
  version: string;
  license: string;
  attribution: string;
  ruleset: string;
  entries: Record<string, PackEntry[]>;
};

/** Every valid homebrew book. Rejects with a StorageError. */
export function listHomebrew(): Promise<Book[]> {
  return invoke("list_homebrew");
}

/** Saves a whole book (a new one when `id` is empty) and returns it as saved. Rejects with a StorageError (a book
 *  that doesn't validate is refused with its errors; the last good one stays). */
export function saveHomebrew(book: Book): Promise<Book> {
  return invoke("save_homebrew", { book });
}

/** Mirrors `Choice` in src-tauri/src/commands/homebrew.rs. */
export type Choice = { index: string; name: string };

/** A Homebrew form's choices from the switched-on packs: the entries of `kind`, or with `field` ("a.b" inside one)
 *  the different values the data has for it. Rejects with a StorageError. */
export function homebrewChoices(kind: string, field?: string): Promise<Choice[]> {
  return invoke("homebrew_choices", { kind, field: field ?? null });
}

/** Export: the user picks where the book goes as a pack .zip. False when cancelled. Rejects with a StorageError. */
export function exportHomebrew(id: string, fileType: string): Promise<boolean> {
  return invoke("export_homebrew", { id, fileType });
}

/** Keeps a picked picture for a saved book (shrunk to 1024 px) and returns the name to put in an entry's `image`; a
 *  save that no longer names it removes it. Rejects with a StorageError (not an image, larger than 20 MB). */
export function putHomebrewPicture(book: string, bytes: Uint8Array): Promise<string> {
  return invoke("put_homebrew_picture", bytes, { headers: { book } });
}

/** Keeps book `from`'s picture `image` for book `to` too, and returns its name there. */
export function copyHomebrewPicture(from: string, to: string, image: string): Promise<string> {
  return invoke("copy_homebrew_picture", { from, to, image });
}
