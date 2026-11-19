import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("$lib/api/addons", () => ({
  addonPacks: vi.fn(async (addon: string) => [{ id: "srd", addon }]),
  addonInstallPack: vi.fn(async () => ({ id: "home" })),
  addonRemovePack: vi.fn(async () => undefined),
  addonFetch: vi.fn(async () => new ArrayBuffer(3)),
  addonPickFile: vi.fn(async () => new ArrayBuffer(0)),
  addonPickFolder: vi.fn(async () => null),
  addonStorageGet: vi.fn(async () => ({ saved: true })),
  addonStorageSet: vi.fn(async () => undefined),
  addonCharacters: vi.fn(async () => []),
  addonSaveCharacter: vi.fn(async (_a: string, _c: string, character: object) => character),
  addonSetCharacterPicture: vi.fn(async () => ({ id: "c" })),
  addonPutFile: vi.fn(async () => undefined),
  addonGetFile: vi.fn(async (_a: string, name: string) => (name === "gone" ? new ArrayBuffer(0) : new Uint8Array([1, 7, 8]).buffer)),
  addonSetLibraryVisible: vi.fn(async () => undefined),
  addonSecretSet: vi.fn(async () => undefined),
  addonSetSearch: vi.fn(async () => undefined),
  addonOpenExternal: vi.fn(async () => undefined),
}));

import { addonFetch, addonOpenExternal, addonPutFile, addonSecretSet, addonSetSearch, addonSetCharacterPicture, addonSetLibraryVisible, addonStorageSet } from "$lib/api/addons";
import { answer, isCall } from "./addonBridge";

const host = () => ({ hidden: false, setHeight: vi.fn(), packsChanged: vi.fn(), charactersChanged: vi.fn(), theme: () => "dark", close: vi.fn(), notify: vi.fn(), navigate: vi.fn(async () => {}), libraryVisible: vi.fn(), sidebarButton: vi.fn(), unsaved: vi.fn(), settingsDone: vi.fn(), openLibrary: vi.fn(async () => {}) });

describe("addon bridge", () => {
  beforeEach(() => vi.clearAllMocks());

  it("opens links only from a page on screen", async () => {
    await answer("lore", "ui.openExternal", ["https://example.com/a"], host());
    expect(addonOpenExternal).toHaveBeenCalledWith("lore", "https://example.com/a");
    await expect(answer("lore", "ui.openExternal", ["https://example.com/b"], { ...host(), hidden: true })).rejects.toThrow("on screen");
    expect(addonOpenExternal).toHaveBeenCalledTimes(1);
  });

  it("only takes well-formed calls", () => {
    expect(isCall({ koldron: 1, id: 1, call: "packs.list" })).toBe(true);
    expect(isCall({ koldron: 1, id: 1, call: "packs.list", args: ["x"] })).toBe(true);
    for (const bad of [null, "x", { id: 1, call: "a" }, { koldron: 1, id: "1", call: "a" }, { koldron: 1, id: 1, call: "a", args: "x" }]) {
      expect(isCall(bad)).toBe(false);
    }
  });

  it("calls on behalf of the addon and tells the frame when packs changed", async () => {
    const h = host();
    expect(await answer("importer", "packs.list", [], h)).toEqual([{ id: "srd", addon: "importer" }]);
    expect(h.packsChanged).not.toHaveBeenCalled();
    await answer("importer", "packs.install", [{ manifest: {}, files: {} }], h);
    await answer("importer", "packs.remove", ["home"], h);
    expect(h.packsChanged).toHaveBeenCalledTimes(2);
    await answer("importer", "storage.set", [], h);
    expect(addonStorageSet).toHaveBeenCalledWith("importer", null);
  });

  it("tells the app when a character changed and sends pictures as bytes", async () => {
    const h = host();
    await answer("sync", "characters.list", ["camp"], h);
    expect(h.charactersChanged).not.toHaveBeenCalled();
    await answer("sync", "characters.save", ["camp", { name: "Gylbard" }], h);
    await answer("sync", "characters.setPicture", ["camp", "c", new Uint8Array([1, 2]).buffer], h);
    expect(h.charactersChanged).toHaveBeenCalledTimes(2);
    expect(addonSetCharacterPicture).toHaveBeenCalledWith("sync", "camp", "c", new Uint8Array([1, 2]));
    await expect(answer("sync", "characters.setPicture", ["camp", "c", "nope"], h)).rejects.toThrow("ArrayBuffer");
    await answer("sync", "ui.close", [], h);
    expect(h.close).toHaveBeenCalled();
    await answer("sync", "ui.notify", ["  "], h);
    await answer("sync", "ui.notify", ["x".repeat(500)], h);
    expect(h.notify.mock.calls).toEqual([["x".repeat(200)]]); // an empty message shows nothing; a long one is cut
  });

  it("maps a cancelled file pick to null and keeps the frame height in bounds", async () => {
    const h = host();
    expect(await answer("a", "files.pickFile", [["zip"]], h)).toBeNull();
    await answer("a", "ui.setHeight", [99999], h);
    await answer("a", "ui.setHeight", [-5], h);
    expect(h.setHeight.mock.calls).toEqual([[4000], [0]]);
    await expect(answer("a", "ui.setHeight", ["tall"], h)).rejects.toThrow("number");
    expect(await answer("a", "ui.theme", [], h)).toBe("dark");
    await expect(answer("a", "app.exec", [], h)).rejects.toThrow('Unknown call "app.exec"');
  });

  it("keeps stored files apart from missing ones and checks navigate and visibility arguments", async () => {
    const h = host();
    await answer("a", "storage.putFile", ["a.bin", new Uint8Array([5]).buffer], h);
    expect(addonPutFile).toHaveBeenCalledWith("a", "a.bin", new Uint8Array([5]));
    await expect(answer("a", "storage.putFile", ["a.bin", "text"], h)).rejects.toThrow("ArrayBuffer");
    expect(new Uint8Array((await answer("a", "storage.getFile", ["a.bin"], h)) as ArrayBuffer)).toEqual(new Uint8Array([7, 8]));
    expect(await answer("a", "storage.getFile", ["gone"], h)).toBeNull();
    await answer("a", "ui.navigate", ["page=3"], h);
    await answer("a", "ui.navigate", [null, true], h);
    expect(h.navigate.mock.calls).toEqual([["page=3", false], [null, true]]);
    await expect(answer("a", "ui.navigate", [5], h)).rejects.toThrow("string or null");
    await expect(answer("a", "ui.navigate", ["x".repeat(2001)], h)).rejects.toThrow("2000");
    await answer("a", "library.setVisible", [false], h);
    expect(addonSetLibraryVisible).toHaveBeenCalledWith("a", false);
    expect(h.libraryVisible).toHaveBeenCalledWith(false);
    await expect(answer("a", "library.setVisible", ["no"], h)).rejects.toThrow("true or false");
    await answer("a", "ui.setSidebarButton", [false], h);
    expect(h.sidebarButton).toHaveBeenCalledWith(false);
    await expect(answer("a", "ui.setSidebarButton", [1], h)).rejects.toThrow("true or false");
    await answer("a", "ui.openLibrary", ["page=3"], h);
    expect(h.openLibrary).toHaveBeenCalledWith("page=3");
    await expect(answer("a", "ui.openLibrary", [5], h)).rejects.toThrow("string or null");
    await answer("a", "ui.setUnsaved", [true], h);
    expect(h.unsaved).toHaveBeenCalledWith(true);
    await answer("a", "ui.settingsDone", [3, null], h);
    await answer("a", "ui.settingsDone", [4, "Disk full"], h);
    expect(h.settingsDone.mock.calls).toEqual([[3, null], [4, "Disk full"]]);
    await expect(answer("a", "ui.settingsDone", ["x"], h)).rejects.toThrow("id");
  });
  it("passes net.fetch options as name/value pairs and keeps secrets text", async () => {
    const h = host();
    await answer("a", "net.fetch", ["https://api.x.com/auth", { method: "POST", headers: { Authorization: "tok" } }], h);
    expect(addonFetch).toHaveBeenLastCalledWith("a", "https://api.x.com/auth", "POST", [["Authorization", "tok"]]);
    await answer("a", "net.fetch", ["https://api.x.com/list"], h);
    expect(addonFetch).toHaveBeenLastCalledWith("a", "https://api.x.com/list", undefined, []);
    await expect(answer("a", "net.fetch", ["https://api.x.com", { headers: ["x"] }], h)).rejects.toThrow("headers");
    await answer("a", "secrets.set", ["key", "abc"], h);
    expect(addonSecretSet).toHaveBeenCalledWith("a", "key", "abc");
    await expect(answer("a", "secrets.set", ["key", 5], h)).rejects.toThrow("string");
    await answer("a", "search.setEntries", [[{ id: "1", name: "Tomb" }]], h);
    expect(addonSetSearch).toHaveBeenCalledWith("a", [{ id: "1", name: "Tomb" }]);
    await expect(answer("a", "search.setEntries", [{}], h)).rejects.toThrow("list");
  });
});
