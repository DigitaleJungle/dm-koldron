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
}));

import { addonSetCharacterPicture, addonStorageSet } from "$lib/api/addons";
import { answer, isCall } from "./addonBridge";

const host = () => ({ setHeight: vi.fn(), packsChanged: vi.fn(), charactersChanged: vi.fn(), theme: () => "dark", close: vi.fn(), notify: vi.fn() });

describe("addon bridge", () => {
  beforeEach(() => vi.clearAllMocks());

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
    await answer("ddb", "characters.list", ["camp"], h);
    expect(h.charactersChanged).not.toHaveBeenCalled();
    await answer("ddb", "characters.save", ["camp", { name: "Gylbard" }], h);
    await answer("ddb", "characters.setPicture", ["camp", "c", new Uint8Array([1, 2]).buffer], h);
    expect(h.charactersChanged).toHaveBeenCalledTimes(2);
    expect(addonSetCharacterPicture).toHaveBeenCalledWith("ddb", "camp", "c", new Uint8Array([1, 2]));
    await expect(answer("ddb", "characters.setPicture", ["camp", "c", "nope"], h)).rejects.toThrow("ArrayBuffer");
    await answer("ddb", "ui.close", [], h);
    expect(h.close).toHaveBeenCalled();
    await answer("ddb", "ui.notify", ["  "], h);
    await answer("ddb", "ui.notify", ["x".repeat(500)], h);
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
});
