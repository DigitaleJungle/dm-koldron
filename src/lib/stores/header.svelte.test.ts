// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { flushSync } from "svelte";
import { header, showHeaderBack } from "./header.svelte";

describe("header", () => {
  it("leaving a screen doesn't clear the next screen's Back", () => {
    const leaveA = $effect.root(() => showHeaderBack(() => "/a"));
    flushSync();
    expect(header.back).toBe("/a");
    flushSync();
    // The next screen mounts before the old one is destroyed.
    const leaveB = $effect.root(() => showHeaderBack(() => "/b"));
    flushSync();
    leaveA();
    expect(header.back).toBe("/b");
    leaveB();
    expect(header.back).toBeNull();
  });
});
