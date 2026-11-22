// The Homebrew form an addon opens beside its Library tab with a draft (`homebrew.openDraft`), where it can send in more
// fields and a picture (`homebrew.fill`, `homebrew.setPicture`). Nothing is saved until the user saves the form.

let next = 0;

/** The form open beside an addon's Library tab: whose, for which book and kind, its draft, and the latest fields sent. */
export type Beside = {
  key: number;
  addon: string;
  book: string;
  kind: string;
  draft: Record<string, unknown>;
  /** `replace`: the fields take the place of what the form has (a picture), rather than filling it. */
  fill: { id: number; fields: Record<string, unknown>; replace?: boolean } | null;
};

export const beside = $state<{ open: Beside | null }>({ open: null });

/** The book last picked in a form beside a tab, where the next one starts. */
let lastBook = "";

/** Opens (or replaces) the form beside `addon`'s Library tab, in the book last picked. */
export function openBeside(addon: string, kind: string, draft: Record<string, unknown>) {
  beside.open = { key: ++next, addon, book: lastBook, kind, draft: structuredClone(draft), fill: null };
}

/** The form beside the tab is now for another book ("": none yet), picked in the form. */
export function moveBeside(key: number, book: string) {
  if (beside.open?.key !== key) return;
  beside.open.book = book;
  if (book) lastBook = book;
}

/** Sends fields into the form beside `addon`'s tab (in place of what's there, with `replace`); an error when it has
 *  none open. */
export function fillBeside(addon: string, fields: Record<string, unknown>, replace = false) {
  if (beside.open?.addon !== addon) throw new Error("No homebrew form is open beside this page.");
  beside.open.fill = { id: ++next, fields: structuredClone(fields), replace };
}

/** Closes the form beside the tab (only this one, when a key is given: a newer one stays). */
export function closeBeside(key?: number) {
  if (key === undefined || beside.open?.key === key) beside.open = null;
}
