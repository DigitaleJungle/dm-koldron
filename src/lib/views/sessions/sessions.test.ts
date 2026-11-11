import { describe, expect, it } from "vitest";
import type { Encounter } from "$lib/api/encounters";
import type { Scene, Session } from "$lib/api/sessions";
import {
  checkCounts,
  filterSessions,
  linkTo,
  newLocation,
  newNpc,
  newScene,
  nextNumber,
  parseDate,
  parseTime,
  readyToRun,
  readyCount,
  removeLocation,
  removeNpc,
  scenesWith,
  toggleSection,
} from "./sessions";

const enc = (id: string, name = id, status = "ready"): Encounter => ({ id, name, status }) as Encounter;
const scene = (id: string, more: Partial<Scene> = {}): Scene => ({ ...newScene(), id, ...more });

/** Two scenes at the ossuary with the abbot, a third elsewhere. */
function session(): Session {
  return {
    id: "s",
    number: 1,
    title: "Crypt",
    date: null,
    time: null,
    partyNote: "",
    previously: "",
    premise: "",
    locations: [{ ...newLocation("Ossuary"), id: "oss" }, { ...newLocation("Vault"), id: "vault" }],
    npcs: [{ ...newNpc("Abbot"), id: "abbot" }, { ...newNpc("Wenna"), id: "wenna" }],
    checklist: [{ text: "Minis", done: true }],
    scenes: [
      scene("a", { locationId: "oss", npcIds: ["abbot"], encounterIds: ["e1"], checklist: [{ text: "Clue", done: false }] }),
      scene("b", { locationId: "oss", npcIds: ["abbot", "wenna"] }),
      scene("c", { locationId: "vault" }),
    ],
    updated: 0,
  };
}

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

  it("starts a scene untitled, empty and not ready, each with its own id", () => {
    const s = newScene();
    expect([s.title, s.ready, s.locationId, s.npcIds, s.checklist]).toEqual(["", false, null, [], []]);
    expect(newScene().id).not.toBe(newScene().id);
  });

  it("keeps one section open at a time", () => {
    expect(toggleSection(null, "npcs")).toBe("npcs");
    expect(toggleSection("npcs", "premise")).toBe("premise");
    expect(toggleSection("premise", "premise")).toBeNull();
  });

  it("shares a location: an edit shows in every scene at it", () => {
    const s = session();
    s.locations[0].name = "Bone hall";
    const names = s.scenes.map((sc) => s.locations.find((l) => l.id === sc.locationId)?.name);
    expect(names).toEqual(["Bone hall", "Bone hall", "Vault"]);
    expect(scenesWith(s, { kind: "location", id: "oss" }).map((sc) => sc.id)).toEqual(["a", "b"]);
  });

  it("links without duplicates, and a location replaces the scene's location", () => {
    const a = session().scenes[0];
    linkTo(a, { kind: "npc", id: "abbot" });
    linkTo(a, { kind: "npc", id: "wenna" });
    linkTo(a, { kind: "encounter", id: "e1" });
    linkTo(a, { kind: "location", id: "vault" });
    expect([a.npcIds, a.encounterIds, a.locationId]).toEqual([["abbot", "wenna"], ["e1"], "vault"]);
  });

  it("clears a removed location from every scene", () => {
    const s = session();
    removeLocation(s, "oss");
    expect(s.locations.map((l) => l.id)).toEqual(["vault"]);
    expect(s.scenes.map((sc) => sc.locationId)).toEqual([null, null, "vault"]);
  });

  it("unlinks a removed NPC from every scene", () => {
    const s = session();
    removeNpc(s, "abbot");
    expect(s.npcs.map((n) => n.id)).toEqual(["wenna"]);
    expect(s.scenes.map((sc) => sc.npcIds)).toEqual([[], ["wenna"], []]);
  });

  it("counts the checklist across the session and its scenes, and the ready scenes", () => {
    expect(checkCounts(session())).toEqual({ done: 1, total: 2 });
    expect(readyCount([{ ready: true }, { ready: false }, { ready: true }])).toBe(2);
  });

  it("offers the encounters that are ready to run, by name", () => {
    const all = [enc("a"), enc("c", "Cellar"), enc("d", "Draft", "draft"), enc("b", "Bridge")];
    expect(readyToRun(all).map((e) => e.name)).toEqual(["a", "Bridge", "Cellar"]);
  });

  it("reads a date and a time, null when they aren't", () => {
    expect(parseDate("2026-11-10")?.getDate()).toBe(10);
    expect(parseDate(null)).toBeNull();
    expect(parseDate("nope")).toBeNull();
    expect(parseTime("19:30")?.getMinutes()).toBe(30);
    expect(parseTime("7:30")).toBeNull();
  });
});
