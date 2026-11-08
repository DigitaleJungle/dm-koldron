import { beforeEach, describe, expect, it, vi } from "vitest";

// The three SvelteKit navigation hooks, captured so the test can play navigations through them.
type Nav = { type: string; delta?: number; from?: { url: URL } | null };
const hooks: Partial<Record<"before" | "on" | "after", (nav: Nav) => void>> = {};
vi.mock("$app/navigation", () => ({
  beforeNavigate: (fn: (nav: Nav) => void) => (hooks.before = fn),
  onNavigate: (fn: (nav: Nav) => void) => (hooks.on = fn),
  afterNavigate: (fn: (nav: Nav) => void) => (hooks.after = fn),
  goto: () => Promise.resolve(),
}));

type History = typeof import("./history.svelte");
let h: History;
let at = "http://app/start";

/** Leaves `at` for `to` the way SvelteKit does: before, on, (the new screen renders), after. */
function navigate(type: string, to: string, delta?: number) {
  const from = { url: new URL(at) };
  hooks.before?.({ type, from });
  hooks.on?.({ type, delta });
  at = to;
  hooks.after?.({ type, delta, from });
}

beforeEach(async () => {
  vi.resetModules(); // fresh counters per test
  h = await import("./history.svelte");
  h.trackHistory();
  at = "http://app/start";
  hooks.after?.({ type: "enter" });
});

describe("canGoBack", () => {
  it("counts steps, not replaced screens", async () => {
    expect(h.canGoBack()).toBe(false); // opened right here
    navigate("link", "http://app/library");
    expect(h.canGoBack()).toBe(true);

    const replacing = h.gotoReplacing("http://app/library?q=fire");
    navigate("goto", "http://app/library?q=fire"); // a refined list is no step
    await replacing;
    navigate("popstate", "http://app/start", -1);
    expect(h.canGoBack()).toBe(false);
  });
});

describe("isReplacing", () => {
  it("is true only while a gotoReplacing is under way, so a guard that holds it can resume it in place", async () => {
    expect(h.isReplacing()).toBe(false);
    const replacing = h.gotoReplacing("http://app/start?x=1");
    expect(h.isReplacing()).toBe(true); // what a beforeNavigate guard sees
    await replacing;
    expect(h.isReplacing()).toBe(false);
  });
});

describe("keepScroll", () => {
  it("returns a pane to where it was only when coming back, once its content is there", () => {
    navigate("link", "http://app/list");
    const pane = { scrollTop: 0 } as HTMLElement;
    const action = h.keepScroll(pane, { id: "list", url: at, ready: true });
    pane.scrollTop = 300;

    navigate("link", "http://app/entry"); // leaving the list saves 300
    pane.scrollTop = 0;
    action.update({ id: "list", url: at, ready: true });
    expect(pane.scrollTop).toBe(0); // a new place starts where it starts

    navigate("popstate", "http://app/list", -1);
    action.update({ id: "list", url: at, ready: false });
    expect(pane.scrollTop).toBe(0); // not loaded yet: nothing to scroll
    action.update({ id: "list", url: at, ready: true });
    expect(pane.scrollTop).toBe(300);

    pane.scrollTop = 50; // the DM scrolls on; a later update in the same visit leaves it alone
    action.update({ id: "list", url: at, ready: true });
    expect(pane.scrollTop).toBe(50);
    action.destroy();
  });
});
