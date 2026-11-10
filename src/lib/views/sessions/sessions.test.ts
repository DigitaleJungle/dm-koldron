import { describe, expect, it } from "vitest";
import type { Encounter } from "$lib/api/encounters";
import type { Item, Step } from "$lib/api/sessions";
import { blankItem, choices, filterSessions, hasItems, newStep, nextNumber, parseDate, parseTime, readyCount } from "./sessions";

const enc = (id: string, name = id): Encounter => ({ id, name }) as Encounter;
const step = (kind: Step["kind"], items: Partial<Item>[] = [], ready = false): Step => ({ ...{ ...newStep(kind), title: "S" }, ready, items: items.map((i) => ({ ...blankItem(), ...i })) });

describe("sessions", () => {
  it("numbers a new session after the highest, from 1", () => {
    expect(nextNumber([])).toBe(1);
    expect(nextNumber([{ number: 3 }, { number: 12 }, { number: 5 }])).toBe(13);
  });

  it("filters by title in any case", () => {
    const all = [{ title: "The Sunken Vault" }, { title: "Bridge" }];
    expect(filterSessions(all, " vAULT ")).toEqual([all[0]]);
    expect(filterSessions(all, " ")).toBe(all);
  });

  it("starts a step untitled, empty and not ready", () => {
    const s = newStep("list");
    expect([s.title, s.ready, s.items]).toEqual(["", false, []]);
    expect(newStep("list").id).not.toBe(newStep("list").id);
  });

  it("counts the ready steps", () => {
    expect(readyCount([])).toBe(0);
    expect(readyCount([{ ready: true }, { ready: false }, { ready: true }])).toBe(2);
  });

  it("knows when an encounter step has its encounter", () => {
    expect(hasItems(step("encounter"))).toBe(false);
    expect(hasItems({ ...step("encounter"), encounter: "a" })).toBe(true);
    expect(hasItems(step("map"))).toBe(false);
    expect(hasItems({ ...step("map"), pins: [{ x: 0.5, y: 0.5, title: "", text: "" }] })).toBe(true);
  });

  it("offers the encounters by name", () => {
    const all = [enc("a"), enc("c", "Cellar"), enc("b", "Bridge")];
    expect(choices(all).map((e) => e.name)).toEqual(["a", "Bridge", "Cellar"]);
  });

  it("reads a date and a time, null when they aren't", () => {
    expect(parseDate("2026-11-10")?.getDate()).toBe(10);
    expect(parseDate(null)).toBeNull();
    expect(parseDate("nope")).toBeNull();
    expect(parseTime("19:30")?.getMinutes()).toBe(30);
    expect(parseTime("7:30")).toBeNull();
  });
});
