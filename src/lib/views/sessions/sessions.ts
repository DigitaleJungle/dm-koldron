import type { Encounter } from "$lib/api/encounters";
import type { Scene, Session, Status } from "$lib/api/sessions";
import type { Location, Npc, World } from "$lib/api/world";

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
export const newNpc = (name = "", parent: string | null = null): Npc => ({ id: crypto.randomUUID(), parent, name, wants: "", voice: "" });

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

/** The sessions the entry, or anything under it, appears in, lowest number first. */
export function sessionsWith<T extends { number: number; locationIds: string[]; npcIds: string[] }>(rows: T[], entries: Entry[], kind: "location" | "npc", id: string): T[] {
  const ids = descendants(entries, id);
  return rows.filter((s) => (kind === "location" ? s.locationIds : s.npcIds).some((x) => ids.includes(x))).toSorted((a, b) => a.number - b.number);
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

/** Removes the NPC from the campaign (what was under it moves up), and unlinks it from its locations and the open
 *  session's scenes. */
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
