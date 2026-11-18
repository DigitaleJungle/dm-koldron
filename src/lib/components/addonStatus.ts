import type { AddonSummary } from "$lib/api/addons";

type Row = Pick<AddonSummary, "id" | "version" | "enabled" | "loadedVersion" | "removed" | "errors">;

/** Settings → Addons drafts waiting for Save: switches changed (id → on), and uninstalls (id → also delete data). */
export type AddonDrafts = { enabled: Record<string, boolean>; uninstall: Record<string, boolean> };

export type AddonStatus = "onSaveUninstall" | "onSaveEnable" | "onSaveDisable" | "pendingRemove" | "pendingDisable" | "pendingLoad" | "pendingUpdate";

/** Whether what's saved differs from what runs this session, so a restart changes something. */
export const restartPending = (a: Row) => a.removed || (a.errors.length === 0 && (a.enabled ? a.version : null) !== a.loadedVersion);

/** What changes for this addon: a draft on Save first, then what's saved after a restart. */
export function addonStatus(a: Row, drafts: AddonDrafts): AddonStatus | null {
  if (a.id in drafts.uninstall) return "onSaveUninstall";
  if (a.id in drafts.enabled) return drafts.enabled[a.id] ? "onSaveEnable" : "onSaveDisable";
  if (a.removed) return "pendingRemove";
  if (!restartPending(a)) return null;
  if (!a.enabled) return "pendingDisable";
  return a.loadedVersion === null ? "pendingLoad" : "pendingUpdate";
}
