// The window's side of the addon message API (addons/addons.md): maps an addon frame's call to a command. Permissions
// are checked in Rust, so nothing here decides what an addon may do.
import {
  addonFetch,
  addonInstallPack,
  addonPacks,
  addonPickFile,
  addonPickFolder,
  addonRemovePack,
  addonStorageGet,
  addonStorageSet,
  type NewPack,
} from "$lib/api/addons";

/** A call from an addon frame: `{ koldron: 1, id, call, args }`. It's answered with `{ koldron: 1, id, ok, result | error }`. */
export type AddonCall = { koldron: 1; id: number; call: string; args?: unknown[] };

export function isCall(data: unknown): data is AddonCall {
  const d = data as AddonCall | null;
  return d?.koldron === 1 && typeof d.id === "number" && typeof d.call === "string" && (d.args === undefined || Array.isArray(d.args));
}

/** What the frame around the addon does for it. */
export type FrameHost = { setHeight: (px: number) => void; packsChanged: () => void; theme: () => string };

/** The tallest an addon can make its frame, in pixels. */
const MAX_HEIGHT = 4000;

export async function answer(addon: string, call: string, args: unknown[], host: FrameHost): Promise<unknown> {
  switch (call) {
    case "packs.list":
      return addonPacks(addon);
    case "packs.install": {
      const installed = await addonInstallPack(addon, args[0] as NewPack);
      host.packsChanged();
      return installed;
    }
    case "packs.remove":
      await addonRemovePack(addon, String(args[0]));
      host.packsChanged();
      return null;
    case "net.fetch":
      return addonFetch(addon, String(args[0]));
    case "files.pickFile": {
      const bytes = await addonPickFile(addon, Array.isArray(args[0]) ? args[0].map(String) : []);
      return bytes.byteLength > 0 ? bytes : null;
    }
    case "files.pickFolder":
      return addonPickFolder(addon);
    case "storage.get":
      return addonStorageGet(addon);
    case "storage.set":
      await addonStorageSet(addon, args[0] ?? null);
      return null;
    case "ui.setHeight": {
      const px = Number(args[0]);
      if (!Number.isFinite(px)) throw new Error("ui.setHeight needs a number of pixels.");
      host.setHeight(Math.min(Math.max(Math.round(px), 0), MAX_HEIGHT));
      return null;
    }
    case "ui.theme":
      return host.theme();
    default:
      throw new Error(`Unknown call "${call}".`);
  }
}
