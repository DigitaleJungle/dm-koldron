// Global search results: what each kind is called, where a result opens, and the types in display order.
import type { SearchHit } from "$lib/api/search";
import { strings } from "$lib/strings";
import { ENTRY_CATEGORIES, entryAddress, entryCategory, sectionAddress } from "$lib/views/library/catalog";

const RULE_SECTIONS = "rule-sections";
const CHARACTERS = "characters";

/** Characters first, then rule sections, then the Catalog's order. */
export const KIND_ORDER = [CHARACTERS, RULE_SECTIONS, ...ENTRY_CATEGORIES.map((cat) => cat.id)];

export const kindLabel = (kind: string) =>
  kind === CHARACTERS ? strings.campaign.characters : kind === RULE_SECTIONS ? strings.search.rules : entryCategory(kind).label;

/** Where it opens: the Rules tab at its chapter, the Catalog, or the campaign's Characters screen. */
export const placeLabel = (hit: SearchHit) =>
  hit.kind === CHARACTERS ? strings.campaign.characters
  : hit.kind === RULE_SECTIONS ? strings.search.inChapter(hit.chapterName ?? "")
  : strings.catalog.title;

/** Type, source and place, each once ("Characters · Strahd", not "Characters · Strahd · Characters"); `withKind`
 * false leaves the type out (under a type's heading). */
export const hitMeta = (hit: SearchHit, withKind = true) =>
  [...new Set([...(withKind ? [kindLabel(hit.kind)] : []), hit.source, placeLabel(hit)])].join(" · ");

/** Where a hit opens; pack content in the Library at `base` (see `entryAddress`). */
export const hitAddress = (hit: SearchHit, base = "/library") =>
  hit.kind === CHARACTERS ? `/campaign/${hit.campaign}/characters?${new URLSearchParams({ edit: hit.index })}`
  : hit.kind === RULE_SECTIONS ? sectionAddress(hit.pack, hit.chapterIndex ?? "", hit.index, base)
  : entryAddress(hit.kind, hit.pack, hit.index, base);

/** The search screen's address for a search, inside a campaign when one is given. */
export const searchAddress = (query: string, campaign?: string) =>
  `/search?${new URLSearchParams({ q: query.trim(), ...(campaign ? { campaign } : {}) })}`;

/** The hits per kind, in `KIND_ORDER`, each keeping the hits' order (best first). */
export function groupByKind(hits: SearchHit[]): { kind: string; hits: SearchHit[] }[] {
  const groups = new Map<string, SearchHit[]>();
  for (const hit of hits) groups.set(hit.kind, [...(groups.get(hit.kind) ?? []), hit]);
  const rank = (kind: string) => (KIND_ORDER.includes(kind) ? KIND_ORDER.indexOf(kind) : KIND_ORDER.length);
  return [...groups].sort(([a], [b]) => rank(a) - rank(b)).map(([kind, hits]) => ({ kind, hits }));
}
