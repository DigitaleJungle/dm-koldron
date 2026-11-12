import type { Encounter } from "$lib/api/encounters";
import type { Scene, Session, Status } from "$lib/api/sessions";
import type { Location, Npc, NpcGroup, World } from "$lib/api/world";

/** The number a new session gets: the highest + 1, 1 for the first. */
export const nextNumber = (sessions: { number: number }[]): number => sessions.reduce((n, s) => Math.max(n, s.number), 0) + 1;

/** The overview's parts: Up next (the lowest-numbered session not played, any status), Later (the other unplayed ones,
 *  lowest first) and Played (highest first). */
export function overview<T extends { number: number; status: Status }>(rows: T[]) {
  const unplayed = rows.filter((r) => r.status !== "played").toSorted((a, b) => a.number - b.number);
  return { next: unplayed[0] ?? null, later: unplayed.slice(1), played: rows.filter((r) => r.status === "played").toSorted((a, b) => b.number - a.number) };
}

/** The party's average level, rounded; null without a party. */
export const averageLevel = (levels: number[]): number | null => (levels.length ? Math.round(levels.reduce((a, b) => a + b, 0) / levels.length) : null);

/** The right column's sections; locations, NPCs and the checklist also have a full view in the editor. */
export type Section = "previously" | "premise" | "summary" | "partyNote" | "locations" | "npcs" | "encounters" | "checklist";
export type Pane = "locations" | "npcs" | "checklist";

/** The drag data type an NPC carries (its id), so a location editor can tell it from other cards while it's dragged. */
export const NPC_DRAG = "application/x-koldron-npc";

/** What can be dragged onto a scene. */
export type Linkable = { kind: "location" | "npc" | "encounter"; id: string };

/** Opening a section closes the open one; opening the open one closes it. */
export const toggleSection = (open: Section | null, key: Section): Section | null => (open === key ? null : key);

export const newScene = (): Scene => ({ id: crypto.randomUUID(), title: "", summary: "", ready: false, locationId: null, npcIds: [], encounterIds: [], checklist: [] });
export const newLocation = (name = "", parent: string | null = null): Location => ({
  id: crypto.randomUUID(),
  parent,
  name,
  atmosphere: "",
  purpose: "",
  secrets: "",
  npcIds: [],
  pins: [],
  images: [],
});
export const newNpc = (name = "", parent: string | null = null): Npc => ({
  id: crypto.randomUUID(),
  parent,
  name,
  race: "",
  gender: "",
  age: "",
  job: "",
  alignment: "",
  summary: "",
  motivation: "",
  traits: "",
  backstory: "",
  stats: { ac: "", hp: "", speed: "", cr: "", str: "", dex: "", con: "", int: "", wis: "", cha: "" },
});

export const newGroup = (name = "", parent: string | null = null): NpcGroup => ({ id: crypto.randomUUID(), parent, name });

/** Earlier states to go back to (Ctrl+Z), latest first; past `max`, the oldest is dropped. */
export class UndoHistory<T> {
  #items: T[] = [];
  constructor(private max = 50) {}

  push(item: T) {
    this.#items.push(item);
    if (this.#items.length > this.max) this.#items.shift();
  }

  pop(): T | undefined {
    return this.#items.pop();
  }

  clear() {
    this.#items = [];
  }
}

/** An NPC's race, gender, age, job and alignment, those filled in, as one line ("Half-elf · Woman · Smith"). */
export const npcSpecs = (n: Npc) => [n.race, n.gender, n.age, n.job, n.alignment].map((s) => s.trim()).filter(Boolean).join(" · ");

// ---- Trees: the campaign's locations and NPCs -----------------------------------------------------
type Entry = { id: string; parent: string | null; name: string };

/** The entry's id and every id under it. */
export function descendants(entries: Entry[], id: string): string[] {
  const out = [id];
  for (let i = 0; i < out.length; i++) for (const e of entries) if (e.parent === out[i] && !out.includes(e.id)) out.push(e.id);
  return out;
}

/** The entries above this one, top first. */
export function ancestors<T extends Entry>(entries: T[], id: string): T[] {
  const byId = new Map(entries.map((e) => [e.id, e]));
  const out: T[] = [];
  for (let p = byId.get(id)?.parent; p && byId.has(p) && !out.some((e) => e.id === p); p = byId.get(p)!.parent) out.unshift(byId.get(p)!);
  return out;
}

export type TreeRow<T> = { entry: T; depth: number; kids: number; open: boolean };

/** The rows a tree shows, in tree order: everything under an open entry. With a query, the entries whose name contains
 *  it (any case) and their parents, all open. An entry whose parent is gone counts as top-level. */
export function treeRows<T extends Entry>(entries: T[], open: Set<string>, query = ""): TreeRow<T>[] {
  const ids = new Set(entries.map((e) => e.id));
  const kids = new Map<string | null, T[]>();
  for (const e of entries) {
    const p = e.parent && ids.has(e.parent) ? e.parent : null;
    kids.set(p, [...(kids.get(p) ?? []), e]);
  }
  const q = query.trim().toLowerCase();
  const shown = q ? new Set(entries.filter((e) => e.name.toLowerCase().includes(q)).flatMap((e) => [e.id, ...ancestors(entries, e.id).map((a) => a.id)])) : null;
  const out: TreeRow<T>[] = [];
  const walk = (parent: string | null, depth: number) => {
    for (const entry of kids.get(parent) ?? []) {
      if (shown && !shown.has(entry.id)) continue;
      const isOpen = shown ? true : open.has(entry.id);
      out.push({ entry, depth, kids: kids.get(entry.id)?.length ?? 0, open: isOpen });
      if (isOpen) walk(entry.id, depth + 1);
    }
  };
  walk(null, 0);
  return out;
}

/** Every entry in tree order, with its depth: for dropdowns. */
export const treeOrder = <T extends Entry>(entries: T[]): TreeRow<T>[] => treeRows(entries, new Set(entries.map((e) => e.id)));

/** The parent `id` gets when dropped on `target` in its tree (null: on the tree itself, so at the top), or undefined
 *  when it can't go there (into itself or something under it). An entry that can't hold others (`canHold`: an NPC)
 *  takes the dropped one in beside it, in its own parent. */
export function dropParent(entries: Entry[], id: string, target: string | null, canHold: (id: string) => boolean): string | null | undefined {
  const parent = target === null || canHold(target) ? target : (entries.find((e) => e.id === target)?.parent ?? null);
  return parent !== null && descendants(entries, id).includes(parent) ? undefined : parent;
}

/** Removes the entry; what was under it moves up to its parent. */
export function removeEntry<T extends Entry>(entries: T[], id: string): T[] {
  const parent = entries.find((e) => e.id === id)?.parent ?? null;
  return entries.filter((e) => e.id !== id).map((e) => (e.parent === id ? { ...e, parent } : e));
}

/** Adds the id unless it's there already. */
export function link(ids: string[], id: string) {
  if (!ids.includes(id)) ids.push(id);
}

export function unlink(ids: string[], id: string) {
  const at = ids.indexOf(id);
  if (at >= 0) ids.splice(at, 1);
}

/** A location sets the scene's location; an NPC or encounter is linked once. */
export function linkTo(scene: Scene, item: Linkable) {
  if (item.kind === "location") scene.locationId = item.id;
  else link(item.kind === "npc" ? scene.npcIds : scene.encounterIds, item.id);
}

/** Removes the location from the campaign (what was under it moves up), and clears it from the open session's scenes;
 *  other sessions drop it when they next save. */
export function removeLocation(world: World, session: Session | null, id: string) {
  world.locations = removeEntry(world.locations, id);
  for (const scene of session?.scenes ?? []) if (scene.locationId === id) scene.locationId = null;
}

/** Removes the group; what was in it moves up to its parent. */
export function removeGroup(world: World, id: string) {
  const parent = world.npcGroups.find((g) => g.id === id)?.parent ?? null;
  world.npcGroups = removeEntry(world.npcGroups, id);
  for (const n of world.npcs) if (n.parent === id) n.parent = parent;
}

/** Removes the NPC from the campaign, and unlinks it from its locations and the open session's scenes. */
export function removeNpc(world: World, session: Session | null, id: string) {
  world.npcs = removeEntry(world.npcs, id);
  for (const l of world.locations) unlink(l.npcIds, id);
  for (const scene of session?.scenes ?? []) unlink(scene.npcIds, id);
}

/** The ids the session's scenes link, each once: locations or NPCs. */
export const linkedIn = (session: Session, kind: "location" | "npc"): string[] => [
  ...new Set(session.scenes.flatMap((s) => (kind === "location" ? (s.locationId ? [s.locationId] : []) : s.npcIds))),
];

/** The scenes that use the location, NPC or encounter. */
export const scenesWith = (session: Session, item: Linkable): Scene[] =>
  session.scenes.filter((s) => (item.kind === "location" ? s.locationId === item.id : (item.kind === "npc" ? s.npcIds : s.encounterIds).includes(item.id)));

/** Checklist items done and in all, the session's and every scene's. */
export function checkCounts(session: Session): { done: number; total: number } {
  const all = [...session.checklist, ...session.scenes.flatMap((s) => s.checklist)];
  return { done: all.filter((c) => c.done).length, total: all.length };
}

/** How many scenes are ready. */
export const readyCount = (scenes: Pick<Scene, "ready">[]): number => scenes.filter((s) => s.ready).length;

/** The campaign's encounters that are ready to run, by name. */
export const readyToRun = (encounters: Encounter[]): Encounter[] =>
  encounters.filter((e) => e.status === "ready").toSorted((a, b) => a.name.localeCompare(b.name));

/** A saved `YYYY-MM-DD` as a local date (not UTC, so it never shifts a day), or null. */
export function parseDate(date: string | null): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date ?? "");
  return m ? new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3])) : null;
}

/** A saved `HH:MM` as a local date today at that time, or null. */
export function parseTime(time: string | null): Date | null {
  const m = /^(\d{2}):(\d{2})$/.exec(time ?? "");
  return m ? new Date(2000, 0, 1, Number(m[1]), Number(m[2])) : null;
}
