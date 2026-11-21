// The window's side of the addon message API (addons/addons.md): maps an addon frame's call to a command. Permissions
// are checked in Rust, so nothing here decides what an addon may do.
import type { Character } from "$lib/api/characters";
import { forgetPictures } from "$lib/stores/monsterPictures";
import {
  addonAddFolder,
  addonCampaigns,
  addonCharacters,
  addonDeleteFile,
  addonGetFile,
  addonListFiles,
  addonPutFile,
  addonRevealFile,
  addonHashFile,
  addonDownloadFile,
  addonSecretGet,
  addonSetSearch,
  addonSecretSet,
  addonSecretDelete,
  addonSetCampaignVisible,
  addonSetLibraryVisible,
  addonFolders,
  addonReadFile,
  addonReadOpened,
  addonOpenFiles,
  addonPutImages,
  addonRemoveFolder,
  addonSaveFile,
  addonScanFolder,
  addonFetch,
  addonCheckPackProgress,
  addonInstallPack,
  addonOpenExternal,
  addonPacks,
  addonPickFile,
  addonPickFolder,
  addonRemovePack,
  addonSaveCharacter,
  addonSetCharacterPicture,
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
export type FrameHost = {
  /** A page out of sight (a background page): it can't open links, so it can't fill the browser unseen. */
  hidden: boolean;
  setHeight: (px: number) => void;
  packsChanged: () => void;
  /** `packs.setProgress`, checked: shows (or with null clears) progress in that pack's Settings → Rules row. */
  packProgress: (pack: string, progress: { name: string; fraction: number | null; text: string } | null) => void;
  charactersChanged: () => void;
  theme: () => string;
  close: () => void;
  notify: (message: string) => void;
  /** `ui.navigate`: moves the app to this state of the addon's Library tab. Only that page's host can; others throw. */
  navigate: (state: string | null, replace: boolean) => Promise<void>;
  /** The addon's Library tab was shown or hidden. */
  libraryVisible: (visible: boolean) => void;
  /** The addon's campaign screen was shown or hidden in a campaign. */
  campaignVisible: (campaign: string, visible: boolean) => void;
  /** `ui.setSidebarButton`: shows or hides the Library header's sidebar button. Only that page's host can; others throw. */
  sidebarButton: (shown: boolean) => void;
  /** `ui.openLibrary`: opens the addon's Library tab at a state (as `ui.navigate`). Only its Start, preview and Home pages can. */
  openLibrary: (state: string | null) => Promise<void>;
  /** `ui.openCampaignScreen`: opens the addon's campaign screen in a campaign at a state. Only its Start, preview and
   * Home pages can, and only where that screen is shown. */
  openCampaignScreen: (campaign: string, state: string | null) => Promise<void>;
  /** `ui.setUnsaved`: a Settings panel has (or no longer has) unsaved changes. Others throw. */
  unsaved: (has: boolean) => void;
  /** `ui.settingsDone`: a Settings panel saved or discarded, as asked by request `id`; `error` when it couldn't. */
  settingsDone: (id: number, error: string | null) => void;
};

/** The tallest an addon can make its frame, in pixels. */
const MAX_HEIGHT = 4000;
/** The longest message an addon can show. */
const MAX_NOTICE = 200;
/** The longest name or text `packs.setProgress` shows. */
const MAX_PROGRESS_TEXT = 100;
/** `packs.setProgress`: the number of the latest call per addon and pack. */
const latestProgress = new Map<string, number>();
/** The longest state `ui.navigate` keeps in the address. */
const MAX_STATE = 2000;

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
    case "packs.setProgress": {
      const [pack, p] = [String(args[0] ?? ""), args[1] as { name?: unknown; fraction?: unknown; text?: unknown } | null];
      const fraction = p?.fraction ?? null;
      if (p !== null && (typeof p !== "object" || (fraction !== null && (typeof fraction !== "number" || !(fraction >= 0 && fraction <= 1)))))
        throw new Error("packs.setProgress needs a pack id and { name, fraction (0–1 or null), text }, or null.");
      // Each call waits for Rust's check; only the latest per pack is shown, so a quick set-then-clear can't end set.
      const key = `${addon}/${pack}`;
      const mine = (latestProgress.get(key) ?? 0) + 1;
      latestProgress.set(key, mine);
      await addonCheckPackProgress(addon, pack);
      if (latestProgress.get(key) !== mine) return null;
      const short = (v: unknown) => [...String(v ?? "")].slice(0, MAX_PROGRESS_TEXT).join("");
      host.packProgress(pack, p && { name: short(p.name) || pack, fraction: fraction as number | null, text: short(p.text) });
      return null;
    }
    case "packs.putImages": {
      const images = args[1];
      if (!Array.isArray(images) || images.some((i) => typeof i?.path !== "string" || !(i?.bytes instanceof ArrayBuffer)))
        throw new Error("packs.putImages needs a pack id and a list of { path, bytes } with each image as an ArrayBuffer.");
      if (images.some((i) => !/^[ -~]+$/.test(i.path))) throw new Error("packs.putImages takes plain ASCII paths.");
      const total = images.reduce((sum, i) => sum + i.bytes.byteLength, 0);
      // Same limit as Rust's MAX_BATCH, checked here too so a too-large call never builds its buffer.
      if (total > 100 * 1024 * 1024) throw new Error("packs.putImages takes at most 100 MB of images at once.");
      const body = new Uint8Array(total);
      let at = 0;
      for (const i of images) {
        body.set(new Uint8Array(i.bytes), at);
        at += i.bytes.byteLength;
      }
      await addonPutImages(addon, String(args[0]), images.map((i) => ({ path: i.path, size: i.bytes.byteLength })), body);
      forgetPictures(); // the encounter screens ask again: a picture may be there now
      return null;
    }
    case "net.fetch": {
      const options = args[1] ?? {};
      if (typeof options !== "object") throw new Error("net.fetch's options are an object.");
      const { method, headers = {} } = options as { method?: unknown; headers?: unknown };
      if (typeof headers !== "object" || headers === null || Array.isArray(headers)) throw new Error("net.fetch's headers are an object of names and values.");
      const pairs = Object.entries(headers).map(([name, value]): [string, string] => [name, String(value)]);
      return addonFetch(addon, String(args[0]), method === undefined ? undefined : String(method), pairs);
    }
    case "files.pickFile": {
      const bytes = await addonPickFile(addon, Array.isArray(args[0]) ? args[0].map(String) : []);
      return bytes.byteLength > 0 ? bytes : null;
    }
    case "files.open":
      return addonOpenFiles(addon, Array.isArray(args[0]) ? args[0].map(String) : []);
    case "files.read": {
      const [offset, length] = [Number(args[1]), Number(args[2])];
      if (!Number.isSafeInteger(offset) || !Number.isSafeInteger(length) || offset < 0 || length < 0) throw new Error("files.read needs a byte offset and length.");
      return addonReadOpened(addon, String(args[0]), offset, length);
    }
    case "files.pickFolder":
      return addonPickFolder(addon);
    case "files.saveFile": {
      if (!(args[1] instanceof ArrayBuffer)) throw new Error("files.saveFile needs the file as an ArrayBuffer.");
      return addonSaveFile(addon, String(args[0] ?? ""), new Uint8Array(args[1]));
    }
    case "folders.list":
      return addonFolders(addon);
    case "folders.add":
      return addonAddFolder(addon);
    case "folders.remove":
      await addonRemoveFolder(addon, String(args[0]));
      return null;
    case "folders.scan":
      return addonScanFolder(addon, String(args[0]));
    case "folders.read": {
      const [offset, length] = [Number(args[2]), Number(args[3])];
      if (!Number.isSafeInteger(offset) || !Number.isSafeInteger(length) || offset < 0 || length < 0) throw new Error("folders.read needs a byte offset and length.");
      return addonReadFile(addon, String(args[0]), String(args[1]), offset, length);
    }
    case "folders.reveal":
      await addonRevealFile(addon, String(args[0]), String(args[1]));
      return null;
    case "folders.hash":
      return addonHashFile(addon, String(args[0]), String(args[1]));
    case "folders.download":
      return addonDownloadFile(addon, String(args[0]), String(args[1]), String(args[2]));
    case "search.setEntries":
      if (!Array.isArray(args[0])) throw new Error("search.setEntries needs a list of entries.");
      await addonSetSearch(addon, args[0]);
      return null;
    case "secrets.get":
      return addonSecretGet(addon, String(args[0]));
    case "secrets.set":
      if (typeof args[1] !== "string") throw new Error("secrets.set needs the secret as a string.");
      await addonSecretSet(addon, String(args[0]), args[1]);
      return null;
    case "secrets.delete":
      await addonSecretDelete(addon, String(args[0]));
      return null;
    case "campaigns.list":
      return addonCampaigns(addon);
    case "characters.list":
      return addonCharacters(addon, String(args[0]));
    case "characters.save": {
      const saved = await addonSaveCharacter(addon, String(args[0]), args[1] as Character);
      host.charactersChanged();
      return saved;
    }
    case "characters.setPicture": {
      const bytes = args[2] instanceof ArrayBuffer ? new Uint8Array(args[2]) : args[2];
      if (!(bytes instanceof Uint8Array)) throw new Error("characters.setPicture needs the picture as an ArrayBuffer.");
      const saved = await addonSetCharacterPicture(addon, String(args[0]), String(args[1]), bytes);
      host.charactersChanged();
      return saved;
    }
    case "storage.get":
      return addonStorageGet(addon);
    case "storage.set":
      await addonStorageSet(addon, args[0] ?? null);
      return null;
    case "storage.putFile": {
      if (!(args[1] instanceof ArrayBuffer)) throw new Error("storage.putFile needs the file as an ArrayBuffer.");
      await addonPutFile(addon, String(args[0]), new Uint8Array(args[1]));
      return null;
    }
    case "storage.getFile": {
      const bytes = await addonGetFile(addon, String(args[0]));
      return bytes.byteLength > 0 ? bytes.slice(1) : null;
    }
    case "storage.deleteFile":
      await addonDeleteFile(addon, String(args[0]));
      return null;
    case "storage.listFiles":
      return addonListFiles(addon);
    case "library.setVisible": {
      if (typeof args[0] !== "boolean") throw new Error("library.setVisible needs true or false.");
      await addonSetLibraryVisible(addon, args[0]);
      host.libraryVisible(args[0]);
      return null;
    }
    case "campaign.setVisible": {
      if (typeof args[0] !== "string" || typeof args[1] !== "boolean") throw new Error("campaign.setVisible needs a campaign id and true or false.");
      await addonSetCampaignVisible(addon, args[0], args[1]);
      host.campaignVisible(args[0], args[1]);
      return null;
    }
    case "ui.navigate": {
      const state = args[0] ?? null;
      if (state !== null && typeof state !== "string") throw new Error("ui.navigate needs a string or null.");
      if (state !== null && state.length > MAX_STATE) throw new Error(`ui.navigate takes at most ${MAX_STATE} characters.`);
      await host.navigate(state, args[1] === true);
      return null;
    }
    case "ui.openLibrary": {
      const state = args[0] ?? null;
      if (state !== null && typeof state !== "string") throw new Error("ui.openLibrary needs a string or null.");
      if (state !== null && state.length > MAX_STATE) throw new Error(`ui.openLibrary takes at most ${MAX_STATE} characters.`);
      await host.openLibrary(state);
      return null;
    }
    case "ui.openCampaignScreen": {
      const [campaign, state = null] = args;
      // A campaign id as the app makes them (it goes into the address).
      if (typeof campaign !== "string" || !/^[a-z0-9-]{1,100}$/.test(campaign)) throw new Error("ui.openCampaignScreen needs a campaign id.");
      if (state !== null && typeof state !== "string") throw new Error("ui.openCampaignScreen needs a string or null as its state.");
      if (state !== null && state.length > MAX_STATE) throw new Error(`ui.openCampaignScreen takes at most ${MAX_STATE} characters.`);
      await host.openCampaignScreen(campaign, state);
      return null;
    }
    case "ui.setSidebarButton": {
      if (typeof args[0] !== "boolean") throw new Error("ui.setSidebarButton needs true or false.");
      host.sidebarButton(args[0]);
      return null;
    }
    case "ui.setUnsaved":
      if (typeof args[0] !== "boolean") throw new Error("ui.setUnsaved needs true or false.");
      host.unsaved(args[0]);
      return null;
    case "ui.settingsDone": {
      const [request, error] = [args[0], args[1] ?? null];
      if (!Number.isSafeInteger(request)) throw new Error("ui.settingsDone needs the request's id.");
      if (error !== null && typeof error !== "string") throw new Error("ui.settingsDone's error is a string or null.");
      host.settingsDone(request as number, error === null ? null : error.slice(0, MAX_NOTICE));
      return null;
    }
    case "ui.setHeight": {
      const px = Number(args[0]);
      if (!Number.isFinite(px)) throw new Error("ui.setHeight needs a number of pixels.");
      host.setHeight(Math.min(Math.max(Math.round(px), 0), MAX_HEIGHT));
      return null;
    }
    case "ui.openExternal":
      if (host.hidden) throw new Error("ui.openExternal works only from a page on screen.");
      if (typeof args[0] !== "string") throw new Error("ui.openExternal needs an https:// address.");
      await addonOpenExternal(addon, args[0]);
      return null;
    case "ui.theme":
      return host.theme();
    case "ui.close":
      host.close();
      return null;
    case "ui.notify": {
      const message = String(args[0] ?? "").trim();
      if (message) host.notify([...message].slice(0, MAX_NOTICE).join(""));
      return null;
    }
    default:
      throw new Error(`Unknown call "${call}".`);
  }
}
