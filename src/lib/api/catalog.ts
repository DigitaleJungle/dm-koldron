import { invoke } from "@tauri-apps/api/core";
import { campaigns } from "$lib/stores/campaigns.svelte";

/** Mirrors `Also`: another place with the same rule. A rule section has its chapter, to open it in the Rules tab. */
export type Also = {
  kind: string;
  pack: string;
  source: string;
  index: string;
  name: string;
  chapterIndex: string | null;
  chapterName: string | null;
};

/** Mirrors `CatalogRow` in src-tauri/src/commands/catalog.rs: `fields` holds only the requested fields the entry has. */
export type CatalogRow = {
  kind: string;
  pack: string;
  source: string;
  index: string;
  name: string;
  fields: Record<string, any>;
  /** The same entry in other packs, shown as this one row, each with its own requested `fields`. */
  others: (Also & { fields: Record<string, any> })[];
};

/** Mirrors `CatalogEntry`: the whole entry, and its description as HTML (raw HTML, links and images removed). */
export type CatalogEntry = {
  pack: string;
  source: string;
  entry: Record<string, any>;
  html: string;
  /** Other texts as HTML: `higher_level`, and `<list>.<n>` for each monster trait or action (`actions.0`…). */
  parts: Record<string, string>;
  /** A class's levels and features, a species' traits (same pack), with their description as HTML. */
  /** `source`: its pack's name. `parts`: a subclass's features' descriptions as HTML, in order. */
  related: { kind: string; entry: Record<string, any>; html: string; source: string; parts: string[] }[];
  /** The same rule elsewhere: another Catalog entry or a rule section, any switched-on pack. */
  also: Also[];
};

/** One category's entries from the switched-on packs, only the campaign's (by default the one worked in) with a
 * `campaign`. Rejects with a StorageError. */
export function catalogRows(kind: string, fields: string[], campaign: string | null = campaigns.scope): Promise<CatalogRow[]> {
  return invoke("catalog_rows", { kind, fields, campaign });
}

/** Rejects with a StorageError (`notFound` when it isn't in a switched-on pack, or the `campaign` leaves its pack out). */
export function catalogEntry(kind: string, pack: string, index: string, campaign: string | null = campaigns.scope): Promise<CatalogEntry> {
  return invoke("catalog_entry", { kind, pack, index, campaign });
}
