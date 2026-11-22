// The order of the app-wide Library's tabs (Settings → UI), saved app-wide. The first tab is where the Library opens.
import { getSettings, setSettings } from "$lib/api/settings";
import { errorMessage } from "$lib/api/storage";
import { ordered } from "$lib/components/sidebarOrder";
import { addons } from "$lib/stores/addons.svelte";
import { strings } from "$lib/strings";

export const libraryTabs = $state<{ order: string[]; loaded: boolean; error: string | null }>({ order: [], loaded: false, error: null });

let loading: Promise<void> | null = null;

/** Reads the saved order once per run; unreadable settings leave the default order (Settings → UI says why). */
export function loadLibraryTabs(): Promise<void> {
  loading ??= getSettings()
    .then((saved) => {
      libraryTabs.order = saved.libraryTabs;
    })
    .catch(() => {})
    .finally(() => (libraryTabs.loaded = true));
  return loading;
}

/** The tabs in the saved order: Rules, Catalog, then Homebrew and each shown addon tab (`appWide`: the Library outside
 *  any campaign only). */
export function libraryTabList(appWide = true): { id: string; label: string }[] {
  const addonTabs = appWide ? addons.list.filter((a) => a.library && a.libraryVisible).map((a) => ({ id: a.id, label: a.library!.label })) : [];
  const homebrew = appWide ? [{ id: "homebrew", label: strings.homebrew.title }] : [];
  return ordered([{ id: "rules", label: strings.library.rulesTab }, { id: "catalog", label: strings.catalog.title }, ...homebrew, ...addonTabs], libraryTabs.order);
}

/** Saves a new order of the shown tabs; hidden addons' tabs keep their saved places after them. */
export async function setLibraryTabs(ids: string[]) {
  libraryTabs.order = [...ids, ...libraryTabs.order.filter((id) => !ids.includes(id))];
  try {
    await setSettings({ libraryTabs: libraryTabs.order });
    libraryTabs.error = null;
  } catch (e) {
    libraryTabs.error = strings.settings.ui.saveError(errorMessage(e));
  }
}
