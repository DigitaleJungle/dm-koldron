import { describe, expect, it } from "vitest";
import { addonStatus, restartPending } from "./addonStatus";

const row = (over: Partial<Parameters<typeof addonStatus>[0]> = {}) => ({
  id: "a",
  version: "1.0",
  enabled: true,
  loadedVersion: "1.0" as string | null,
  removed: false,
  errors: [] as string[],
  ...over,
});
const none = { enabled: {}, uninstall: {} };

describe("addonStatus", () => {
  it("puts drafts first, uninstall over a switch, then what a restart changes", () => {
    expect(addonStatus(row(), { enabled: { a: false }, uninstall: { a: true } })).toBe("onSaveUninstall");
    expect(addonStatus(row(), { enabled: { a: false }, uninstall: {} })).toBe("onSaveDisable");
    expect(addonStatus(row({ enabled: false, loadedVersion: null }), { enabled: { a: true }, uninstall: {} })).toBe("onSaveEnable");
    expect(addonStatus(row({ removed: true }), none)).toBe("pendingRemove");
    expect(addonStatus(row({ enabled: false }), none)).toBe("pendingDisable");
    expect(addonStatus(row({ loadedVersion: null }), none)).toBe("pendingLoad");
    expect(addonStatus(row({ version: "2.0" }), none)).toBe("pendingUpdate");
  });

  it("says nothing when the running addon is what's saved", () => {
    expect(addonStatus(row(), none)).toBeNull();
    expect(addonStatus(row({ enabled: false, loadedVersion: null }), none)).toBeNull();
    expect(addonStatus(row({ errors: ["bad"], loadedVersion: null }), none)).toBeNull();
    expect(restartPending(row({ errors: ["bad"], loadedVersion: null }))).toBe(false);
  });
});
