import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
// @ts-expect-error plain .mjs script without type declarations
import { parseBudgets, typeOf } from "./build-srd-pack.mjs";

const table = (rows: string) =>
  `Intro.\n\n**XP Budget per Character**\n\n| Party's Level | Low | Moderate | High |\n|---|---|---|---|\n${rows}\n\n#### Step 3`;

describe("parseBudgets", () => {
  it("reads the rows, with thousands separators", () => {
    expect(parseBudgets(table("| 1 | 50 | 75 | 100 |\n| 2 | 1,000 | 1,500 | 2,000 |"))).toEqual([
      { level: 1, low: 50, moderate: 75, high: 100 },
      { level: 2, low: 1000, moderate: 1500, high: 2000 },
    ]);
  });

  it("reads the real SRD table: 20 levels", () => {
    const sections = JSON.parse(readFileSync("packs/srd/source/2024/en/5e-SRD-Rule-Sections.json", "utf8"));
    const rows = parseBudgets(sections.find((s: { index: string }) => s.index === "combat-encounters").desc);
    expect(rows).toHaveLength(20);
    expect(rows[0]).toEqual({ level: 1, low: 50, moderate: 75, high: 100 });
    expect(rows[19]).toEqual({ level: 20, low: 6400, moderate: 13200, high: 22000 });
  });

  it("rejects a missing table, a changed header, bad numbers and skipped levels", () => {
    expect(() => parseBudgets("no table")).toThrow();
    expect(() => parseBudgets(table("").replace("High", "Deadly"))).toThrow();
    expect(() => parseBudgets(table("| 1 | 50 | lots | 100 |"))).toThrow();
    expect(() => parseBudgets(table("| 1 | 50 | 75 | 100 |\n| 3 | 1 | 2 | 3 |"))).toThrow();
    expect(() => parseBudgets(table(""))).toThrow();
  });
});

it("typeOf turns a 5e-database file name into an entry type", () => {
  expect(typeOf("5e-SRD-Magic-Items.json")).toBe("magic-items");
  expect(typeOf("5e-SRD-Rule-Sections.json")).toBe("rule-sections");
});
