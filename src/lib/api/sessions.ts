import { invoke } from "@tauri-apps/api/core";

/** A checklist item. Mirrors `Check` in src-tauri/src/commands/sessions.rs, as do the types below. */
export type Check = { text: string; done: boolean };

/** `locationId` and `npcIds` point into the campaign's world (`./world`), `encounterIds` to its encounters. */
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

/** `date`: `YYYY-MM-DD`, `time`: `HH:MM`, or null; `updated`: ms, set by Rust. `started`, `ended` (ms) and
 *  `playedScenes` are set by the live session view. */
export type Session = {
  id: string;
  number: number;
  title: string;
  date: string | null;
  time: string | null;
  partyNote: string;
  previously: string;
  premise: string;
  summary: string;
  started: number | null;
  ended: number | null;
  playedScenes: number | null;
  scenes: Scene[];
  checklist: Check[];
  updated: number;
};

export type Status = "drafting" | "ready" | "playing" | "played";

/** What the overview shows of a session. `locationIds`, `npcIds`: what its scenes link, each once. */
export type SessionRow = {
  id: string;
  number: number;
  title: string;
  date: string | null;
  time: string | null;
  status: Status;
  premise: string;
  summary: string;
  ready: number;
  scenes: number;
  locationIds: string[];
  npcIds: string[];
  encounters: number;
  playedScenes: number | null;
  playedMs: number | null;
};

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
