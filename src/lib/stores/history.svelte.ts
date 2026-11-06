import { afterNavigate, beforeNavigate, goto, onNavigate } from "$app/navigation";

// How many screens the user can go back within the app: Back returns to the previous one when there is one, else to a
// fixed screen (the app was just opened there). Counted from SvelteKit's navigations, since `history` can't tell.
let depth = 0;
let replacing = false;
// Whether the current screen was reached with Back (or Forward), and a number per navigation.
let cameBack = false;
let navigation = 0;
// The address the current screen was opened from with a link or `goto`; null after Back or Forward, or on opening.
let previous = $state<URL | null>(null);
// Scroll positions of the scrolling panes, per address, saved when leaving it.
const saved = new Map<string, number>();
const panes = new Map<HTMLElement, string>();

/** Call once, from the root layout. */
export function trackHistory() {
  beforeNavigate(({ from }) => {
    if (from) for (const [pane, id] of panes) saved.set(`${from.url.href} ${id}`, pane.scrollTop);
  });
  onNavigate((nav) => {
    cameBack = nav.type === "popstate";
    navigation++;
  });
  afterNavigate((nav) => {
    if (nav.type === "enter") depth = 0;
    else if (nav.type === "popstate") depth = Math.max(0, depth + (nav.delta ?? 0));
    else if (!replacing) depth++;
    if (!replacing) previous = nav.type === "enter" || nav.type === "popstate" ? null : (nav.from?.url ?? null);
    replacing = false;
  });
}

export const canGoBack = () => depth > 0;

/** Where the current screen was opened from, when that was a link or `goto` (reactive). */
export const previousUrl = () => previous;

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
