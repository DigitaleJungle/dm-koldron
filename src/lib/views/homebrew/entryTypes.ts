// The entry types the Homebrew tab writes, and each one's form fields, in the pack schema's field names
// (5e-database 2024, `packs/packs.md`). Choices and suggestions come from the switched-on packs, never from here.
import type { PackEntry } from "$lib/api/homebrew";
import { strings } from "$lib/strings";
import { ABILITIES as ABILITY_KEYS } from "$lib/views/library/entries/format";

/** Where a field's choices come from: the entries of `kind`, or with `field` the values the data has for it. */
export type Source = { kind: string; field?: string };

export type Field = { key: string; label: string } & (
  | { input: "text"; suggest?: Source }
  | { input: "number"; min?: number; step?: number | "any"; required?: boolean }
  | { input: "bool" }
  /** `list`: stored as a list of paragraphs (a magic item's `desc`). */
  | { input: "markdown"; list?: boolean }
  /** One entry referred to, or several (`refs`): `{ index, name, url }`. */
  | { input: "ref" | "refs"; of: string; required?: boolean }
  /** Several of the values the data has (a spell's components). */
  | { input: "tags"; suggest: Source }
  /** Entries of `of` each with a number: `{ value, proficiency: { index, name, url } }` (a monster's saves and skills). */
  | { input: "bonuses"; of: string }
  /** Named texts in order: `{ name, desc }` (a monster's traits and actions); `text` names the text field instead of
   *  `desc`, and `leveled` adds a `level` to each (a subclass's features). */
  | { input: "actions"; text?: string; leveled?: boolean }
  /** Fields side by side under one label (the six abilities). */
  | { input: "group"; fields: Field[] }
  /** Entries of `of` this one lists, edited here as named texts (`text`: their text field) and pointing back at it
   *  in their `back` list (a species' traits). */
  | { input: "linked"; of: string; text: string; back: string }
  /** A class's level table: its `levels` entries and the `features` they list. */
  | { input: "levels" }
);

export type EntryType = { id: string; label: string; fields: Field[] };

const c = strings.catalog.columns;
const e = strings.entry;
const h = strings.homebrew.fields;
const kinds = strings.catalog.categories;
const m = strings.homebrew.monster;
const num = (key: string, label: string, extra: { min?: number; step?: number | "any"; required?: boolean } = {}): Field => ({ key, label, input: "number", ...extra });
const text = (key: string, label: string): Field => ({ key, label, input: "text" });

export const ENTRY_TYPES: EntryType[] = [
  {
    id: "spells",
    label: kinds.spells,
    fields: [
      { key: "level", label: c.level, input: "number", min: 0, step: 1, required: true },
      { key: "school", label: c.school, input: "ref", of: "magic-schools" },
      { key: "classes", label: h.classes, input: "refs", of: "classes" },
      { key: "casting_time", label: e.castingTime, input: "text", suggest: { kind: "spells", field: "casting_time" } },
      { key: "ritual", label: strings.catalog.ritual, input: "bool" },
      { key: "range", label: e.range, input: "text", suggest: { kind: "spells", field: "range" } },
      { key: "components", label: e.components, input: "tags", suggest: { kind: "spells", field: "components" } },
      { key: "material", label: h.material, input: "text" },
      { key: "duration", label: e.duration, input: "text", suggest: { kind: "spells", field: "duration" } },
      { key: "concentration", label: strings.catalog.concentration, input: "bool" },
      { key: "description", label: h.description, input: "markdown" },
      { key: "higher_level", label: h.higherLevel, input: "markdown" },
    ],
  },
  {
    id: "monsters",
    label: kinds.monsters,
    fields: [
      { key: "size", label: c.size, input: "text", suggest: { kind: "monsters", field: "size" } },
      { key: "type", label: c.type, input: "text", suggest: { kind: "monsters", field: "type" } },
      { key: "alignment", label: m.alignment, input: "text", suggest: { kind: "monsters", field: "alignment" } },
      { key: "defense", label: m.defense, input: "group", fields: [num("armor_class.0.value", e.ac, { min: 0 }), num("hit_points", e.hp, { min: 1, required: true }), text("hit_dice", m.hitDice)] },
      {
        key: "speed",
        label: e.speed,
        input: "group",
        fields: [text("speed.walk", m.walk), text("speed.fly", m.fly), { key: "speed.hover", label: m.hover, input: "bool" }, text("speed.swim", m.swim), text("speed.climb", m.climb), text("speed.burrow", m.burrow)],
      },
      { key: "abilities", label: e.abilityTable, input: "group", fields: ABILITY_KEYS.map((a) => num(a, e.abilities[a], { min: 1 })) },
      { key: "proficiencies", label: m.proficiencies, input: "bonuses", of: "proficiencies" },
      { key: "damage_vulnerabilities", label: e.vulnerabilities, input: "tags", suggest: { kind: "damage-types", field: "index" } },
      { key: "damage_resistances", label: e.resistances, input: "tags", suggest: { kind: "damage-types", field: "index" } },
      { key: "damage_immunities", label: e.immunities, input: "tags", suggest: { kind: "damage-types", field: "index" } },
      { key: "condition_immunities", label: m.conditionImmunities, input: "refs", of: "conditions" },
      {
        key: "senses",
        label: e.senses,
        input: "group",
        fields: [text("senses.darkvision", m.darkvision), text("senses.blindsight", m.blindsight), text("senses.tremorsense", m.tremorsense), text("senses.truesight", m.truesight), num("senses.passive_perception", m.passivePerception, { min: 0 })],
      },
      { key: "languages", label: e.languages, input: "text" },
      { key: "challenge", label: e.cr, input: "group", fields: [num("challenge_rating", e.cr, { min: 0, step: "any" }), num("xp", m.xp, { min: 0 }), num("proficiency_bonus", m.pb)] },
      { key: "special_abilities", label: e.traits, input: "actions" },
      { key: "actions", label: e.actions, input: "actions" },
      { key: "bonus_actions", label: e.bonusActions, input: "actions" },
      { key: "reactions", label: e.reactions, input: "actions" },
      { key: "legendary_actions", label: e.legendaryActions, input: "actions" },
    ],
  },
  {
    id: "classes",
    label: kinds.classes,
    fields: [
      num("hit_die", c.hitDie, { min: 1, required: true }),
      { key: "primary_ability.desc", label: c.primaryAbility, input: "text" },
      { key: "saving_throws", label: e.savingThrows, input: "refs", of: "ability-scores" },
      { key: "proficiencies", label: h.proficiencies, input: "refs", of: "proficiencies" },
      { key: "proficiency_choices.0.desc", label: h.proficiencyChoice, input: "text" },
      { key: "levels", label: h.levelTable, input: "levels" },
    ],
  },
  {
    id: "subclasses",
    label: h.subclasses,
    fields: [
      { key: "class", label: h.class, input: "ref", of: "classes", required: true },
      { key: "summary", label: h.summary, input: "text" },
      { key: "description", label: h.description, input: "markdown" },
      { key: "features", label: strings.entry.features, input: "actions", text: "description", leveled: true },
    ],
  },
  {
    id: "species",
    label: kinds.species,
    fields: [
      { key: "type", label: e.creatureType, input: "text", suggest: { kind: "species", field: "type" } },
      { key: "size", label: c.size, input: "text", suggest: { kind: "species", field: "size" } },
      num("speed", c.speed, { min: 0 }),
      { key: "traits", label: e.traits, input: "linked", of: "traits", text: "description", back: "species" },
    ],
  },
  {
    id: "magic-items",
    label: kinds.magicItems,
    fields: [
      { key: "equipment_category", label: c.category, input: "ref", of: "equipment-categories" },
      { key: "rarity.name", label: c.rarity, input: "text", suggest: { kind: "magic-items", field: "rarity.name" } },
      { key: "attunement", label: c.attunement, input: "bool" },
      { key: "desc", label: h.description, input: "markdown", list: true },
    ],
  },
  {
    id: "equipment",
    label: kinds.equipment,
    fields: [
      { key: "equipment_categories", label: c.category, input: "refs", of: "equipment-categories" },
      { key: "cost.quantity", label: c.cost, input: "number", min: 0, step: "any" },
      { key: "cost.unit", label: h.costUnit, input: "text", suggest: { kind: "equipment", field: "cost.unit" } },
      { key: "weight", label: c.weight, input: "number", min: 0, step: "any" },
      { key: "description", label: h.description, input: "markdown" },
    ],
  },
  {
    id: "feats",
    label: kinds.feats,
    fields: [
      { key: "type", label: c.type, input: "text", suggest: { kind: "feats", field: "type" } },
      { key: "description", label: h.description, input: "markdown" },
    ],
  },
  {
    id: "backgrounds",
    label: kinds.backgrounds,
    fields: [
      { key: "ability_scores", label: c.abilityScores, input: "refs", of: "ability-scores" },
      { key: "feat", label: e.feat, input: "ref", of: "feats" },
      { key: "proficiencies", label: h.proficiencies, input: "refs", of: "proficiencies" },
      { key: "description", label: h.description, input: "markdown" },
    ],
  },
  {
    id: "conditions",
    label: kinds.conditions,
    fields: [{ key: "description", label: h.description, input: "markdown" }],
  },
];

export const entryType = (id: string | null) => ENTRY_TYPES.find((t) => t.id === id);

/** Every field, the ones inside groups too. */
export const allFields = (fields: Field[]): Field[] => fields.flatMap((f) => (f.input === "group" ? allFields(f.fields) : [f]));

/** A field's value; `a.b` is a field inside one. */
export function getField(entry: PackEntry, key: string): unknown {
  return key.split(".").reduce<unknown>((value, part) => (value as Record<string, unknown> | undefined)?.[part], entry);
}

/** Sets a field (`a.b` inside one, `a.0.b` in a list's first item, made when missing); `undefined`, `null` or ""
 *  leaves it out, and with it what holds it once that's empty (`speed: {}`, `armor_class: [{}]`). */
export function setField(entry: PackEntry, key: string, value: unknown) {
  const parts = key.split(".");
  const path: Record<string, unknown>[] = [entry];
  parts.slice(0, -1).forEach((part, i) => {
    const target = path.at(-1)!;
    if (typeof target[part] !== "object" || target[part] === null) target[part] = /^\d+$/.test(parts[i + 1]) ? [] : {};
    path.push(target[part] as Record<string, unknown>);
  });
  if (value !== undefined && value !== null && value !== "") {
    path.at(-1)![parts.at(-1)!] = value;
    return;
  }
  const empty = (v: unknown): boolean => typeof v === "object" && v !== null && Object.values(v).every((x) => x === undefined || empty(x));
  for (let i = parts.length - 1; i >= 0; i--) {
    delete path[i][parts[i]];
    if (i > 0 && !empty(path[i])) break;
  }
}

/** A reference to an entry, as the pack schema has them; the url names its type, so it opens like the SRD's. */
export const reference = (kind: string, index: string, name: string) => ({ index, name, url: `/api/${kind}/${index}` });
