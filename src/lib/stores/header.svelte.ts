import type { Snippet } from "svelte";

// The campaign header (CampaignShell): Back and a screen's own controls left of the search bar, its main action right of it. The screen
// under it fills them while it's shown.
export const header = $state<{ back: string | null; lead: Snippet | null; action: Snippet | null }>({
  back: null,
  lead: null,
  action: null,
});
const owners = { back: 0, lead: 0, action: 0 };
let claims = 0;

function fill<K extends keyof typeof header>(key: K, value: () => (typeof header)[K]) {
  $effect(() => {
    const id = (owners[key] = ++claims);
    header[key] = value();
    // The next screen may have filled it already; only clear our own.
    return () => {
      if (owners[key] === id) header[key] = null;
    };
  });
}

/** From a screen's script: the header's Back while `fallback()` (where it goes without history) is a string. */
export const showHeaderBack = (fallback: () => string | null) => fill("back", fallback);

/** From a screen's script: `action()` (a snippet, e.g. a button) right of the search bar while it isn't null. */
export const showHeaderAction = (action: () => Snippet | null) => fill("action", action);

/** From a screen's script: `controls()` (a snippet) next to Back while it isn't null. */
export const showHeaderLead = (controls: () => Snippet | null) => fill("lead", controls);
