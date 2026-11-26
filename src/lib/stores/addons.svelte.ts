// The loaded addons, plus what the app tells their background pages: whether something live (a running encounter or
// session) is on screen, and a count that goes up whenever an addon saved a character, so screens showing
// characters read them again.
import { untrack } from "svelte";
import { listAddons, type AddonSummary } from "$lib/api/addons";

export const addons = $state<{ list: AddonSummary[]; live: number; charactersChanged: number }>({ list: [], live: 0, charactersChanged: 0 });

let loading: Promise<void> | null = null;

/** Reads the addons once per run (they only change on a restart). Only the running ones (valid and enabled); Settings → Addons lists them all. */
export function loadAddons(): Promise<void> {
  loading ??= listAddons().then(
    (list) => void (addons.list = list.filter((a) => a.loadedVersion !== null)),
    () => {}, // Settings → Addons and Rules show why the addons couldn't be read
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

/** An addon showed or hid its Library tab: the tab bar follows at once. */
export function setLibraryVisible(addon: string, visible: boolean) {
  const found = addons.list.find((a) => a.id === addon);
  if (found) found.libraryVisible = visible;
}

/** An addon showed or hid its campaign screen in one campaign: the sidebar follows at once. */
export function setCampaignVisible(addon: string, campaign: string, visible: boolean) {
  const found = addons.list.find((a) => a.id === addon);
  if (found) found.campaignVisibility = { ...found.campaignVisibility, [campaign]: visible };
}

/** Whether an addon's campaign screen shows in a campaign: as the addon set it there, else as its manifest says. */
export const campaignScreenShown = (addon: AddonSummary, campaign: string) => !!addon.campaign && (addon.campaignVisibility[campaign] ?? addon.campaign.visible);

/** The fields of a character its addon owns (keeps up to date), while that addon is loaded; none otherwise. */
export function ownedFields(character: { linked?: string | null } | undefined): string[] {
  return addons.list.find((a) => a.id === character?.linked)?.characters?.owns ?? [];
}

/** For a screen that's live while shown: `$effect(goLive)`. */
export function goLive(): () => void {
  // Untracked: `++` reads `live`, so the effect would depend on what it changes and run again and again.
  untrack(() => addons.live++);
  return () => void untrack(() => addons.live--);
}
