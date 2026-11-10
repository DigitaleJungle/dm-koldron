import type { Encounter } from "$lib/api/encounters";
import type { Location, Npc, Scene, Session } from "$lib/api/sessions";

/** The number a new session gets: the highest + 1, 1 for the first. */
export const nextNumber = (sessions: { number: number }[]): number => sessions.reduce((n, s) => Math.max(n, s.number), 0) + 1;

/** Sessions whose title contains the text (any case); all of them for blank text. */
export function filterSessions<T extends { title: string }>(sessions: T[], query: string): T[] {
  const q = query.trim().toLowerCase();
  return q ? sessions.filter((s) => s.title.toLowerCase().includes(q)) : sessions;
}

/** The right column's sections; locations, NPCs and the checklist also have a full view in the editor. */
export type Section = "previously" | "premise" | "locations" | "npcs" | "encounters" | "checklist";
export type Pane = "locations" | "npcs" | "checklist";

/** What can be dragged onto a scene. */
export type Linkable = { kind: "location" | "npc" | "encounter"; id: string };

/** Opening a section closes the open one; opening the open one closes it. */
export const toggleSection = (open: Section | null, key: Section): Section | null => (open === key ? null : key);

export const newScene = (): Scene => ({ id: crypto.randomUUID(), title: "", summary: "", ready: false, locationId: null, npcIds: [], encounterIds: [], checklist: [] });
export const newLocation = (name = ""): Location => ({ id: crypto.randomUUID(), name, description: "" });
export const newNpc = (name = ""): Npc => ({ id: crypto.randomUUID(), name, wants: "", voice: "" });

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

/** Removes the location, and clears it from every scene at it. */
export function removeLocation(session: Session, id: string) {
  session.locations = session.locations.filter((l) => l.id !== id);
  for (const scene of session.scenes) if (scene.locationId === id) scene.locationId = null;
}

/** Removes the NPC, and unlinks it from every scene. */
export function removeNpc(session: Session, id: string) {
  session.npcs = session.npcs.filter((n) => n.id !== id);
  for (const scene of session.scenes) unlink(scene.npcIds, id);
}

/** Moves the scene at `from` one place up (-1) or down (1); nothing past either end. */
export function moveScene(session: Session, from: number, by: -1 | 1) {
  const to = from + by;
  if (from < 0 || to < 0 || to >= session.scenes.length) return;
  const [scene] = session.scenes.splice(from, 1);
  session.scenes.splice(to, 0, scene);
}

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
