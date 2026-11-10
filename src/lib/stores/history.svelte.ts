import { afterNavigate, beforeNavigate, goto, onNavigate } from "$app/navigation";

// How many screens the user can go back within the app: Back returns to the previous one when there is one, else to a
// fixed screen (the app was just opened there). Counted from SvelteKit's navigations, since `history` can't tell.
let depth = 0;
let replacing = false;
// Whether the current screen was reached with Back (or Forward), and a number per navigation.
let cameBack = false;
let navigation = 0;
// Scroll positions of the scrolling panes, per address, saved when leaving it; the oldest go past MAX_SAVED.
const saved = new Map<string, number>();
const MAX_SAVED = 500;
const panes = new Map<HTMLElement, string>();

/** Call once, from the root layout. */
export function trackHistory() {
  beforeNavigate(({ from }) => {
    if (!from) return;
    for (const [pane, id] of panes) {
      const key = `${from.url.href} ${id}`;
      saved.delete(key); // set again below, so it counts as the newest
      saved.set(key, pane.scrollTop);
    }
    // Every visited address adds positions; keep the most recent ones (Back rarely goes further).
    for (const key of saved.keys()) {
      if (saved.size <= MAX_SAVED) break;
      saved.delete(key);
    }
  });
  onNavigate((nav) => {
    cameBack = nav.type === "popstate";
    navigation++;
  });
  afterNavigate((nav) => {
    if (nav.type === "enter") depth = 0;
    else if (nav.type === "popstate") depth = Math.max(0, depth + (nav.delta ?? 0));
    else if (!replacing) depth++;
    replacing = false;
  });
}

export const canGoBack = () => depth > 0;

/** True while a `gotoReplacing` is under way: a guard that holds the navigation resumes it in the same place. */
export const isReplacing = () => replacing;

/** True when this screen was reached with Back: then a pane's saved scroll position wins over scrolling to a target. */
export const reachedByBack = () => cameBack;

/** `goto` that takes the current screen's place in the history (Back skips it): use it for every replaceState. */
export function gotoReplacing(url: string, options: Omit<NonNullable<Parameters<typeof goto>[1]>, "replaceState"> = {}) {
  replacing = true;
  // Reset either way: a navigation that fails or goes nowhere must not leave the next one uncounted.
  return goto(url, { ...options, replaceState: true }).finally(() => (replacing = false));
}

type KeepScroll = { id: string; url: string; ready: boolean };

/**
 * `use:keepScroll={{ id, url: page.url.href, ready }}` on a scrolling pane: coming back to an address puts the pane
 * where it was. `ready` is true once its content is there (scrolling an empty pane goes nowhere); `url` makes the
 * action see every navigation, also when the pane stays on screen.
 */
export function keepScroll(pane: HTMLElement, params: KeepScroll) {
  let restored = -1; // the navigation this pane last restored for
  function restore({ id, url, ready }: KeepScroll) {
    panes.set(pane, id);
    if (!ready || restored === navigation) return;
    restored = navigation;
    const top = cameBack ? saved.get(`${url} ${id}`) : undefined;
    if (top !== undefined) pane.scrollTop = top;
  }
  restore(params);
  return { update: restore, destroy: () => panes.delete(pane) };
}
