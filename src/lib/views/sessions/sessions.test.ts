import { describe, expect, it } from "vitest";
import type { Encounter } from "$lib/api/encounters";
import type { Scene, Session, Status } from "$lib/api/sessions";
import type { World } from "$lib/api/world";
import {
  ancestors,
  averageLevel,
  checkCounts,
  descendants,
  linkedIn,
  linkTo,
  newLocation,
  newNpc,
  newScene,
  nextNumber,
  overview,
  parseDate,
  parseTime,
  readyToRun,
  readyCount,
  removeEntry,
  removeGroup,
  dropParent,
  UndoHistory,
  removeLocation,
  removeNpc,
  scenesWith,
  toggleSection,
  treeOrder,
  treeRows,
} from "./sessions";

const enc = (id: string, name = id, status = "ready"): Encounter => ({ id, name, status }) as Encounter;
const scene = (id: string, more: Partial<Scene> = {}): Scene => ({ ...newScene(), id, ...more });

/** The ossuary and the vault, inside the crypt; the abbot and Wenna. */
function world(): World {
  return {
    locations: [{ ...newLocation("Crypt"), id: "crypt" }, { ...newLocation("Ossuary", "crypt"), id: "oss" }, { ...newLocation("Vault", "crypt"), id: "vault" }],
    npcGroups: [],
    npcs: [{ ...newNpc("Abbot"), id: "abbot" }, { ...newNpc("Wenna"), id: "wenna" }],
  };
}

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
    summary: "",
    started: null,
    ended: null,
    playedScenes: null,
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

  it("puts the lowest unplayed session up next, the other unplayed later, and the played highest first", () => {
    const row = (number: number, status: Status) => ({ number, status });
    const parts = overview([row(9, "played"), row(13, "drafting"), row(11, "played"), row(12, "ready"), row(14, "playing")]);
    expect(parts.next).toEqual(row(12, "ready"));
    expect(parts.later.map((r) => r.number)).toEqual([13, 14]);
    expect(parts.played.map((r) => r.number)).toEqual([11, 9]);
    expect(overview([row(1, "played")]).next).toBeNull(); // Planned shows
    expect(overview([row(2, "drafting")]).next?.number).toBe(2); // any status
  });

  it("averages the party's level, rounded", () => {
    expect(averageLevel([5, 5, 6, 4])).toBe(5);
    expect(averageLevel([5, 6])).toBe(6);
    expect(averageLevel([])).toBeNull();
  });

  it("walks a tree: what's under, what's above, and the rows that show", () => {
    const w = world();
    expect(descendants(w.locations, "crypt")).toEqual(["crypt", "oss", "vault"]);
    expect(ancestors(w.locations, "vault").map((l) => l.id)).toEqual(["crypt"]);
    const rows = (open: string[], q = "") => treeRows(w.locations, new Set(open), q).map((r) => `${r.depth}:${r.entry.id}`);
    expect(rows([])).toEqual(["0:crypt"]);
    expect(rows(["crypt"])).toEqual(["0:crypt", "1:oss", "1:vault"]);
    expect(rows([], "VAU")).toEqual(["0:crypt", "1:vault"]); // a match with its parents, open
    expect(treeRows(w.locations, new Set())[0].kids).toBe(2);
    expect(treeOrder([{ ...newLocation("Lost", "gone"), id: "lost" }]).map((r) => r.depth)).toEqual([0]); // parent gone: top
  });

  it("moves what was under a removed entry up a level", () => {
    const deeper = [...world().locations, { ...newLocation("Niche", "oss"), id: "niche" }];
    expect(removeEntry(deeper, "oss").find((l) => l.id === "niche")?.parent).toBe("crypt");
    expect(removeEntry(deeper, "crypt").filter((l) => l.parent === null).map((l) => l.id)).toEqual(["oss", "vault"]);
  });

  it("lists what a session's scenes link, each once", () => {
    expect(linkedIn(session(), "location")).toEqual(["oss", "vault"]);
    expect(linkedIn(session(), "npc")).toEqual(["abbot", "wenna"]);
    expect(linkedIn(session(), "encounter")).toEqual(["e1"]);
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
    const [s, w] = [session(), world()];
    w.locations[1].name = "Bone hall";
    const names = s.scenes.map((sc) => w.locations.find((l) => l.id === sc.locationId)?.name);
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
    const [s, w] = [session(), world()];
    removeLocation(w, s, "oss");
    expect(w.locations.map((l) => l.id)).toEqual(["crypt", "vault"]);
    expect(s.scenes.map((sc) => sc.locationId)).toEqual([null, null, "vault"]);
    removeLocation(w, null, "vault"); // from the overview: no session open
    expect(w.locations.map((l) => l.id)).toEqual(["crypt"]);
  });

  it("unlinks a removed NPC from every scene", () => {
    const [s, w] = [session(), world()];
    w.locations[0].npcIds = ["abbot", "wenna"];
    removeNpc(w, s, "abbot");
    expect(w.npcs.map((n) => n.id)).toEqual(["wenna"]);
    expect(w.locations[0].npcIds).toEqual(["wenna"]); // unlinked from its locations too
    expect(s.scenes.map((sc) => sc.npcIds)).toEqual([[], ["wenna"], []]);
  });

  it("goes back through the undo history latest first, keeping only the newest", () => {
    const undo = new UndoHistory<number>(2);
    [1, 2, 3].forEach((n) => undo.push(n));
    expect([undo.pop(), undo.pop(), undo.pop()]).toEqual([3, 2, undefined]); // 1 was dropped
    undo.push(4);
    undo.clear();
    expect(undo.pop()).toBeUndefined();
  });

  it("finds where a dragged entry lands, never inside itself", () => {
    const w = world();
    const any = () => true;
    expect(dropParent(w.locations, "vault", "oss", any)).toBe("oss");
    expect(dropParent(w.locations, "oss", null, any)).toBeNull();
    expect(dropParent(w.locations, "crypt", "vault", any)).toBeUndefined(); // under itself
    expect(dropParent(w.locations, "crypt", "crypt", any)).toBeUndefined();
    // NPCs hold nothing: dropped on one, an NPC goes in beside it.
    const tree = [{ id: "harbour", parent: null, name: "Harbour" }, { id: "abbot", parent: "harbour", name: "Abbot" }, { id: "wenna", parent: null, name: "Wenna" }];
    const groups = (id: string) => id === "harbour";
    expect(dropParent(tree, "wenna", "abbot", groups)).toBe("harbour");
    expect(dropParent(tree, "harbour", "abbot", groups)).toBeUndefined();
  });

  it("moves what was in a removed group up to its parent", () => {
    const w = world();
    w.npcGroups = [{ id: "harbour", parent: null, name: "Harbour" }, { id: "smugglers", parent: "harbour", name: "Smugglers" }];
    w.npcs[1].parent = "smugglers";
    removeGroup(w, "smugglers");
    expect(w.npcGroups.map((g) => g.id)).toEqual(["harbour"]);
    expect(w.npcs[1].parent).toBe("harbour");
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
