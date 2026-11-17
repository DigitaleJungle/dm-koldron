import { invoke } from "@tauri-apps/api/core";

/** Mirrors `Hit` in src-tauri/src/commands/search.rs: what it is (`kind`) and where it lives. */
export type SearchHit = {
  kind: string;
  /** The pack's id; empty for what isn't from a pack (a character). */
  pack: string;
  /** The pack's or the campaign's name. */
  source: string;
  index: string;
  name: string;
  /** A rule section's chapter. */
  chapterIndex: string | null;
  chapterName: string | null;
  /** The campaign it belongs to; null for pack content, which every campaign shares. */
  campaign: string | null;
  /** Shown under its name: an addon entry's tags. */
  detail: string | null;
  /** An addon entry's place in its Library tab. */
  state: string | null;
};

/**
 * Best matches first; `limit` caps them (the dropdown), none returns all (the search screen). With a `campaign`, only
 * that campaign's characters (and the packs). Rejects with a StorageError.
 */
export function searchAll(query: string, limit?: number, campaign?: string): Promise<SearchHit[]> {
  return invoke("search_all", { query, limit, campaign });
}
