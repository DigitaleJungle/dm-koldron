// The addon panels open in the Settings dialog: which have unsaved changes (`ui.setUnsaved`), and how to tell each to
// save or discard them (a `settings` event, answered with `ui.settingsDone`). The dialog's Save and Cancel use it.
type Action = "save" | "discard";

/** Panel key → it has unsaved changes. */
export const unsavedPanels = $state<Record<string, boolean>>({});
const requests = new Map<string, (action: Action) => Promise<void>>();

/** A panel opened; the returned function forgets it when it closes. */
export function registerPanel(key: string, request: (action: Action) => Promise<void>): () => void {
  requests.set(key, request);
  return () => {
    requests.delete(key);
    delete unsavedPanels[key];
  };
}

/** Tells every panel with unsaved changes to save or discard them; the panels' errors, if any. */
export async function settlePanels(action: Action): Promise<string[]> {
  const errors: string[] = [];
  for (const [key, request] of requests) {
    if (!unsavedPanels[key]) continue;
    try {
      await request(action);
      unsavedPanels[key] = false;
    } catch (e) {
      errors.push(e instanceof Error ? e.message : String(e));
    }
  }
  return errors;
}
