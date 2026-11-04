import { invoke } from "@tauri-apps/api/core";

/** Mirrors `CatalogRow` in src-tauri/src/commands/catalog.rs: `fields` holds only the requested fields the entry has. */
export type CatalogRow = { kind: string; pack: string; source: string; index: string; name: string; fields: Record<string, any> };

/** Mirrors `CatalogEntry`: the whole entry, and its description as HTML (raw HTML, links and images removed). */
export type CatalogEntry = { pack: string; source: string; entry: Record<string, any>; html: string };

/** One category's entries from the switched-on packs. Rejects with a StorageError. */
export function catalogRows(kind: string, fields: string[]): Promise<CatalogRow[]> {
  return invoke("catalog_rows", { kind, fields });
}

/** Rejects with a StorageError (`notFound` when it isn't in a switched-on pack). */
export function catalogEntry(kind: string, pack: string, index: string): Promise<CatalogEntry> {
  return invoke("catalog_entry", { kind, pack, index });
}
