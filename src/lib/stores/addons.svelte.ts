// The loaded addons, plus what the app tells their background pages: whether something live (a running encounter or
// session) is on screen, and a count that goes up whenever an addon saved a character, so screens showing
// characters read them again.
import { listAddons, type AddonSummary } from "$lib/api/addons";

export const addons = $state<{ list: AddonSummary[]; live: number; charactersChanged: number }>({ list: [], live: 0, charactersChanged: 0 });

let loading: Promise<void> | null = null;

/** Reads the addons once per run (they only change on a restart). Invalid ones are left out here; About lists them. */
export function loadAddons(): Promise<void> {
  loading ??= listAddons().then(
    (list) => void (addons.list = list.filter((a) => a.errors.length === 0)),
    () => {}, // Settings → About and Rules show why the addons couldn't be read
  );
  return loading;
}

const CHANGES_SETTLE_MS = 250;
let changedTimer: ReturnType<typeof setTimeout> | undefined;

/** An addon saved a character. Saves close together (one sync of several characters) count once, so screens
 * read the characters once. */
export function charactersChanged() {
  clearTimeout(changedTimer);
  changedTimer = setTimeout(() => addons.charactersChanged++, CHANGES_SETTLE_MS);
}

/** For a screen that's live while shown: `$effect(goLive)`. */
export function goLive(): () => void {
  addons.live++;
  return () => void addons.live--;
}
