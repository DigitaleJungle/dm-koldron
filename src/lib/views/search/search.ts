// Global search results: what each kind is called, where a result opens, and the types in display order.
import type { SearchHit } from "$lib/api/search";
import { strings } from "$lib/strings";
import { ENTRY_CATEGORIES, entryAddress, entryCategory, sectionAddress } from "$lib/views/library/catalog";

const RULE_SECTIONS = "rule-sections";

/** Rule sections first, then the Catalog's order. */
export const KIND_ORDER = [RULE_SECTIONS, ...ENTRY_CATEGORIES.map((cat) => cat.id)];

export const kindLabel = (kind: string) => (kind === RULE_SECTIONS ? strings.search.rules : entryCategory(kind).label);

/** Where it opens: the Rules tab at its chapter, or the Catalog. */
export const placeLabel = (hit: SearchHit) =>
  hit.kind === RULE_SECTIONS ? strings.search.inChapter(hit.chapterName ?? "") : strings.catalog.title;

export const hitAddress = (hit: SearchHit) =>
  hit.kind === RULE_SECTIONS ? sectionAddress(hit.pack, hit.chapterIndex ?? "", hit.index) : entryAddress(hit.kind, hit.pack, hit.index);

/** The search screen's address for a search. */
export const searchAddress = (query: string) => `/search?${new URLSearchParams({ q: query.trim() })}`;

/** The hits per kind, in `KIND_ORDER`, each keeping the hits' order (best first). */
export function groupByKind(hits: SearchHit[]): { kind: string; hits: SearchHit[] }[] {
  const groups = new Map<string, SearchHit[]>();
  for (const hit of hits) groups.set(hit.kind, [...(groups.get(hit.kind) ?? []), hit]);
  const rank = (kind: string) => (KIND_ORDER.includes(kind) ? KIND_ORDER.indexOf(kind) : KIND_ORDER.length);
  return [...groups].sort(([a], [b]) => rank(a) - rank(b)).map(([kind, hits]) => ({ kind, hits }));
}
