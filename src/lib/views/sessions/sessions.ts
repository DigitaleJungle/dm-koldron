import type { Encounter } from "$lib/api/encounters";
import type { Item, ListStyle, Step, StepKind } from "$lib/api/sessions";

/** The number a new session gets: the highest + 1, 1 for the first. */
export const nextNumber = (sessions: { number: number }[]): number => sessions.reduce((n, s) => Math.max(n, s.number), 0) + 1;

/** Sessions whose title contains the text (any case); all of them for blank text. */
export function filterSessions<T extends { title: string }>(sessions: T[], query: string): T[] {
  const q = query.trim().toLowerCase();
  return q ? sessions.filter((s) => s.title.toLowerCase().includes(q)) : sessions;
}

export const KINDS: StepKind[] = ["note", "list", "encounter", "map"];

export const LIST_STYLES: ListStyle[] = ["bulleted", "numbered", "check"];

export const blankItem = (): Item => ({ title: "", body: "", done: false });

/** A new, empty step of a kind. */
export const newStep = (kind: StepKind): Step => ({ id: crypto.randomUUID(), kind, title: "", ready: false, items: [], encounter: null, listStyle: "bulleted", pins: [], map: null });

/** How many steps are ready. */
export const readyCount = (steps: Pick<Step, "ready">[]): number => steps.filter((s) => s.ready).length;

/** Whether the step has anything in it: a note with text, an encounter, or any item. */
export const hasItems = (step: Step): boolean =>
  step.kind === "note"
    ? step.items.some((i) => i.body !== "")
    : step.kind === "encounter"
      ? step.encounter !== null
      : step.kind === "map"
        ? step.map !== null || step.pins.length > 0
        : step.items.length > 0;

/** The campaign's encounters, by name. */
export const choices = (encounters: Encounter[]): Encounter[] => encounters.toSorted((a, b) => a.name.localeCompare(b.name));

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
