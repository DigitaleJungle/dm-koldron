import { invoke } from "@tauri-apps/api/core";

export type StepKind = "note" | "list" | "encounter";

export type ListStyle = "bulleted" | "numbered" | "check";

/** Mirrors `Item` in src-tauri/src/commands/sessions.rs; the kind decides which fields count: note: `body` (cleaned
 *  HTML, the only item); list: `title`, `body` (plain text), plus `done` in a checklist. */
export type Item = { title: string; body: string; done: boolean };

/** An Encounter step has no items; `encounter` is its encounter's id, or null until one is picked. `listStyle`
 *  counts for a List step only. */
export type Step = { id: string; kind: StepKind; title: string; ready: boolean; items: Item[]; encounter: string | null; listStyle: ListStyle };

/** `date`: `YYYY-MM-DD`, `time`: `HH:MM`, or null; `updated`: ms, set by Rust. */
export type Session = {
  id: string;
  number: number;
  title: string;
  date: string | null;
  time: string | null;
  steps: Step[];
  partyNote: string;
  toBring: string[];
  updated: number;
};

/** What the list shows of a session. */
export type SessionRow = { id: string; number: number; title: string; date: string | null; time: string | null; ready: number; steps: number };

/** Highest number first; `unreadable`: files that couldn't be read, left untouched. */
export type SessionList = { sessions: SessionRow[]; unreadable: string[] };

// All of these reject with a StorageError.
export function listSessions(campaign: string): Promise<SessionList> {
  return invoke("list_sessions", { campaign });
}

export function getSession(campaign: string, id: string): Promise<Session> {
  return invoke("get_session", { campaign, id });
}

/** Creates the session when its `id` is empty, else replaces it. Returns it as saved. */
export function saveSession(campaign: string, session: Session): Promise<Session> {
  return invoke("save_session", { campaign, session });
}

/** Deletes the session only; its encounters stay. */
export function deleteSession(campaign: string, id: string): Promise<void> {
  return invoke("delete_session", { campaign, id });
}

/** The campaign's open threads, shared by every session. */
export function getThreads(campaign: string): Promise<string[]> {
  return invoke("get_threads_cmd", { campaign });
}

/** Replaces the open threads at once; returns them as saved. */
export function saveThreads(campaign: string, threads: string[]): Promise<string[]> {
  return invoke("save_threads_cmd", { campaign, threads });
}
