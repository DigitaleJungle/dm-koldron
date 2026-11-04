// What the Catalog shows per category (docs/SPEC.md): the fields it loads, its columns and its filters, and how rows
// are searched, filtered and sorted. Values come from the 5e-database fields of the entries.
import type { CatalogRow } from "$lib/api/catalog";
import { strings } from "$lib/strings";

export type Column = {
  id: string;
  label: string;
  value: (row: CatalogRow) => string;
  /** Sort key when the text doesn't sort right (numbers, CR fractions). */
  sort?: (row: CatalogRow) => number;
};

/** A row matches an option when `values` includes it. Options are the values found in the rows, in `order` if given. */
export type Filter = { id: string; label: string; values: (row: CatalogRow) => string[]; order?: string[] };

export type Category = { id: string; label: string; fields: string[]; columns: Column[]; filters: Filter[] };

const c = strings.catalog.columns;
const none = "—";
const names = (refs: unknown): string[] => (Array.isArray(refs) ? refs.map((r) => r?.name).filter(Boolean) : []);
/** "fighting-style" → "Fighting Style". */
const title = (s: string) => s.replace(/[-_]/g, " ").replace(/\b\w/g, (ch) => ch.toUpperCase());
const num = (n: unknown) => (typeof n === "number" ? n : Number.NaN);
const sortNum = (n: unknown) => (Number.isNaN(num(n)) ? -1 : num(n));

const levelLabel = (level: number) => (level === 0 ? strings.catalog.cantrip : String(level));
const LEVELS = [strings.catalog.cantrip, "1", "2", "3", "4", "5", "6", "7", "8", "9"];

const FRACTIONS: Record<number, string> = { 0.125: "1/8", 0.25: "1/4", 0.5: "1/2" };
export const crText = (cr: unknown) => (typeof cr === "number" ? (FRACTIONS[cr] ?? String(cr)) : none);
const CR_BANDS: [string, number][] = [["0–½", 1], ["1–4", 5], ["5–10", 11], ["11–16", 17], ["17+", Infinity]];
const crBand = (cr: number) => CR_BANDS.find(([, below]) => cr < below)?.[0] ?? "";

const SIZES = ["Tiny", "Small", "Medium", "Large", "Huge", "Gargantuan"];
/** "Medium or small" → ["Medium", "Small"]. */
const sizes = (size: unknown) => (typeof size === "string" ? size.split(/\s+or\s+/i).map(title) : []);

// The 5e coin values, only to sort the Cost column (hard-coded by decision, Nov 2026).
const COIN: Record<string, number> = { cp: 1, sp: 10, ep: 50, gp: 100, pp: 1000 };
const costText = (cost: any) => (cost?.quantity !== undefined ? `${cost.quantity} ${cost.unit ?? ""}`.trim() : none);
const costSort = (cost: any) => (cost?.quantity ?? -1) * (COIN[cost?.unit] ?? 1);

const RARITIES = ["Common", "Uncommon", "Rare", "Very Rare", "Legendary", "Artifact", "Varies"];
/** "Rare (+1), Very Rare (+2), or Legendary (+3)" → ["Rare", "Very Rare", "Legendary"]; "Rarity Varies" → ["Varies"]. */
export function rarities(name: unknown): string[] {
  if (typeof name !== "string") return [];
  if (/varies/i.test(name)) return ["Varies"];
  return name
    .replace(/\([^)]*\)/g, "")
    .split(/,|\bor\b/)
    .map((s) => title(s.trim().toLowerCase()))
    .filter(Boolean);
}

const spellTags = (f: Record<string, any>) => [f.concentration && strings.catalog.concentration, f.ritual && strings.catalog.ritual].filter(Boolean) as string[];
const valueOr = (s: unknown) => (typeof s === "string" && s ? s : none);

export const CATEGORIES: Category[] = [
  {
    id: "spells",
    label: strings.catalog.categories.spells,
    fields: ["level", "school", "casting_time", "range", "concentration", "ritual", "classes"],
    columns: [
      { id: "level", label: c.level, value: (r) => levelLabel(r.fields.level), sort: (r) => sortNum(r.fields.level) },
      { id: "school", label: c.school, value: (r) => valueOr(r.fields.school?.name) },
      // "Reaction, which you take when…" → "Reaction"
      { id: "castingTime", label: c.castingTime, value: (r) => valueOr(String(r.fields.casting_time ?? "").split(",")[0]) },
      { id: "range", label: c.range, value: (r) => valueOr(r.fields.range) },
      { id: "tags", label: c.tags, value: (r) => spellTags(r.fields).join(", ") || none },
    ],
    filters: [
      { id: "level", label: c.level, values: (r) => [levelLabel(r.fields.level)], order: LEVELS },
      { id: "school", label: c.school, values: (r) => names([r.fields.school]) },
      { id: "class", label: c.class, values: (r) => names(r.fields.classes) },
      { id: "other", label: c.other, values: (r) => spellTags(r.fields) },
    ],
  },
  {
    id: "monsters",
    label: strings.catalog.categories.monsters,
    fields: ["challenge_rating", "type", "size", "armor_class", "hit_points"],
    columns: [
      { id: "cr", label: c.cr, value: (r) => crText(r.fields.challenge_rating), sort: (r) => sortNum(r.fields.challenge_rating) },
      { id: "type", label: c.type, value: (r) => (r.fields.type ? title(r.fields.type) : none) },
      { id: "size", label: c.size, value: (r) => sizes(r.fields.size).join(" or ") || none, sort: (r) => SIZES.indexOf(sizes(r.fields.size)[0]) },
      { id: "ac", label: c.ac, value: (r) => String(r.fields.armor_class?.[0]?.value ?? none), sort: (r) => sortNum(r.fields.armor_class?.[0]?.value) },
      { id: "hp", label: c.hp, value: (r) => String(r.fields.hit_points ?? none), sort: (r) => sortNum(r.fields.hit_points) },
    ],
    filters: [
      { id: "cr", label: c.cr, values: (r) => (typeof r.fields.challenge_rating === "number" ? [crBand(r.fields.challenge_rating)] : []), order: CR_BANDS.map(([b]) => b) },
      { id: "type", label: c.type, values: (r) => (r.fields.type ? [title(r.fields.type)] : []) },
      { id: "size", label: c.size, values: (r) => sizes(r.fields.size), order: SIZES },
    ],
  },
  {
    id: "equipment",
    label: strings.catalog.categories.equipment,
    fields: ["equipment_categories", "cost", "weight"],
    columns: [
      { id: "category", label: c.category, value: (r) => names(r.fields.equipment_categories)[0] ?? none },
      { id: "cost", label: c.cost, value: (r) => costText(r.fields.cost), sort: (r) => costSort(r.fields.cost) },
      { id: "weight", label: c.weight, value: (r) => (typeof r.fields.weight === "number" ? strings.catalog.pounds(r.fields.weight) : none), sort: (r) => sortNum(r.fields.weight) },
    ],
    filters: [{ id: "category", label: c.category, values: (r) => names(r.fields.equipment_categories).slice(0, 1) }],
  },
  {
    id: "magic-items",
    label: strings.catalog.categories.magicItems,
    fields: ["equipment_category", "rarity", "attunement"],
    columns: [
      { id: "category", label: c.category, value: (r) => valueOr(r.fields.equipment_category?.name) },
      { id: "rarity", label: c.rarity, value: (r) => valueOr(r.fields.rarity?.name), sort: (r) => RARITIES.indexOf(rarities(r.fields.rarity?.name)[0]) },
      { id: "attunement", label: c.attunement, value: (r) => (r.fields.attunement ? strings.catalog.required : none) },
    ],
    filters: [
      { id: "rarity", label: c.rarity, values: (r) => rarities(r.fields.rarity?.name), order: RARITIES },
      { id: "category", label: c.category, values: (r) => names([r.fields.equipment_category]) },
      { id: "attunement", label: c.attunement, values: (r) => [r.fields.attunement ? strings.catalog.required : strings.catalog.notRequired] },
    ],
  },
  {
    id: "feats",
    label: strings.catalog.categories.feats,
    fields: ["type"],
    columns: [{ id: "type", label: c.type, value: (r) => (r.fields.type ? title(r.fields.type) : none) }],
    filters: [{ id: "type", label: c.type, values: (r) => (r.fields.type ? [title(r.fields.type)] : []), order: ["Origin", "General", "Fighting Style", "Epic Boon"] }],
  },
  {
    id: "classes",
    label: strings.catalog.categories.classes,
    fields: ["hit_die", "primary_ability"],
    columns: [
      { id: "hitDie", label: c.hitDie, value: (r) => (r.fields.hit_die ? `d${r.fields.hit_die}` : none), sort: (r) => sortNum(r.fields.hit_die) },
      { id: "primary", label: c.primaryAbility, value: (r) => valueOr(r.fields.primary_ability?.desc) },
    ],
    filters: [],
  },
  {
    id: "species",
    label: strings.catalog.categories.species,
    fields: ["size", "speed"],
    columns: [
      { id: "size", label: c.size, value: (r) => sizes(r.fields.size).join(" or ") || none, sort: (r) => SIZES.indexOf(sizes(r.fields.size)[0]) },
      { id: "speed", label: c.speed, value: (r) => (typeof r.fields.speed === "number" ? strings.catalog.feet(r.fields.speed) : none), sort: (r) => sortNum(r.fields.speed) },
    ],
    filters: [{ id: "size", label: c.size, values: (r) => sizes(r.fields.size), order: SIZES }],
  },
  {
    id: "backgrounds",
    label: strings.catalog.categories.backgrounds,
    fields: ["ability_scores", "feat"],
    columns: [
      { id: "abilities", label: c.abilityScores, value: (r) => names(r.fields.ability_scores).join(", ") || none },
      { id: "feat", label: c.feat, value: (r) => valueOr(r.fields.feat?.name) },
    ],
    filters: [],
  },
  { id: "conditions", label: strings.catalog.categories.conditions, fields: [], columns: [], filters: [] },
];

/** The categories that are an entry type; "all" lists the entries of every one of them. */
export const ENTRY_CATEGORIES = [...CATEGORIES];

CATEGORIES.unshift({
  id: "all",
  label: strings.catalog.categories.all,
  fields: [],
  columns: [{ id: "category", label: c.category, value: (r) => entryCategory(r.kind).label }],
  filters: [{ id: "category", label: c.category, values: (r) => [entryCategory(r.kind).label], order: ENTRY_CATEGORIES.map((cat) => cat.label) }],
});

export const categoryById = (id: string | null) => CATEGORIES.find((cat) => cat.id === id) ?? CATEGORIES[0];
/** The entry type's own category (for an entry opened from the All tab). */
export const entryCategory = (kind: string) => ENTRY_CATEGORIES.find((cat) => cat.id === kind) ?? ENTRY_CATEGORIES[0];

/** The options a filter offers for these rows: the values present, in the filter's order (else alphabetical). */
export function filterOptions(rows: CatalogRow[], filter: Filter): string[] {
  const found = [...new Set(rows.flatMap(filter.values))];
  const rank = (v: string) => (filter.order ? filter.order.indexOf(v) : -1);
  return found.sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
}

export type Query = { search: string; filters: Record<string, string[]>; sort: string; desc: boolean };

/** Every search word in the name; options within a filter OR, filters AND; then sorted (ties by name). */
export function selectRows(rows: CatalogRow[], category: Category, query: Query): CatalogRow[] {
  const words = query.search.toLowerCase().split(/\s+/).filter(Boolean);
  const active = category.filters.filter((f) => query.filters[f.id]?.length);
  const matches = (row: CatalogRow) =>
    words.every((w) => row.name.toLowerCase().includes(w)) &&
    active.every((f) => f.values(row).some((v) => query.filters[f.id].includes(v)));
  const column = category.columns.find((col) => col.id === query.sort);
  const byName = (a: CatalogRow, b: CatalogRow) => a.name.localeCompare(b.name);
  const compare = (a: CatalogRow, b: CatalogRow) => {
    if (query.sort === "source") return a.source.localeCompare(b.source);
    if (!column) return byName(a, b);
    return column.sort ? column.sort(a) - column.sort(b) : column.value(a).localeCompare(column.value(b));
  };
  const sign = query.desc ? -1 : 1;
  return rows.filter(matches).sort((a, b) => sign * compare(a, b) || byName(a, b));
}
