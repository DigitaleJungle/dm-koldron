import { invoke } from "@tauri-apps/api/core";
import type { Also } from "./catalog";

/** Mirrors `Hit` in src-tauri/src/commands/search.rs: what it is (`kind`), where it lives, and a rule section's chapter. */
export type SearchHit = Also;

/** Best matches first; `limit` caps them (the dropdown), none returns all (the search screen). Rejects with a StorageError. */
export function searchAll(query: string, limit?: number): Promise<SearchHit[]> {
  return invoke("search_all", { query, limit });
}
