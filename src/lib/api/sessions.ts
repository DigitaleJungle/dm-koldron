import { invoke } from "@tauri-apps/api/core";

/** A checklist item. Mirrors `Check` in src-tauri/src/commands/sessions.rs, as do the types below. */
export type Check = { text: string; done: boolean };

/** `locationId` and `npcIds` point into the session's own lists, `encounterIds` to the campaign's encounters. */
export type Scene = {
  id: string;
  title: string;
  summary: string;
  ready: boolean;
  locationId: string | null;
  npcIds: string[];
  encounterIds: string[];
  checklist: Check[];
};

export type Location = { id: string; name: string; description: string };

export type Npc = { id: string; name: string; wants: string; voice: string };

/** `date`: `YYYY-MM-DD`, `time`: `HH:MM`, or null; `updated`: ms, set by Rust. */
export type Session = {
  id: string;
  number: number;
  title: string;
  date: string | null;
  time: string | null;
  partyNote: string;
  previously: string;
  premise: string;
  scenes: Scene[];
  locations: Location[];
  npcs: Npc[];
  checklist: Check[];
  updated: number;
};

/** What the list shows of a session. */
export type SessionRow = { id: string; number: number; title: string; date: string | null; time: string | null; ready: number; scenes: number };

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
