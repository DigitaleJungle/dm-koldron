// Text for the Catalog's entry pages, from 5e-database fields.
import { strings } from "$lib/strings";
import { crText, title } from "../catalog";

const t = strings.entry;
const number = new Intl.NumberFormat();
const names = (refs: unknown): string[] => (Array.isArray(refs) ? refs.map((r) => r?.name).filter(Boolean) : []);
/** "+3", "−1" (a real minus sign), "+0". */
export const signed = (n: number) => (n < 0 ? `−${Math.abs(n)}` : `+${n}`);

// Spells

/** "Level 3 Evocation (Sorcerer, Wizard)", "Evocation Cantrip (Wizard)". */
export function spellHeader(spell: Record<string, any>): string {
  const school = spell.school?.name ?? "";
  const kind = spell.level === 0 ? t.cantrip(school) : t.spellLevel(spell.level, school);
  const classes = names(spell.classes);
  return classes.length ? `${kind} (${classes.join(", ")})` : kind;
}

export const castingTime = (spell: Record<string, any>) => (spell.ritual ? t.orRitual(spell.casting_time ?? "") : (spell.casting_time ?? ""));
export const components = (spell: Record<string, any>) => {
  const list = (spell.components ?? []).join(", ");
  return spell.material && spell.components?.includes("M") ? `${list} (${spell.material})` : list;
};
export const duration = (spell: Record<string, any>) => (spell.concentration ? t.concentration(spell.duration ?? "") : (spell.duration ?? ""));

// Monsters

export const ABILITIES = ["strength", "dexterity", "constitution", "intelligence", "wisdom", "charisma"] as const;
// The 5e ability modifier, for the stat block only (hard-coded like the coin values).
export const modifier = (score: number) => Math.floor((score - 10) / 2);

/** "Large Dragon, Chaotic Evil". */
export function monsterHeader(m: Record<string, any>): string {
  const what = [m.size, m.type && title(m.type)].filter(Boolean).join(" ");
  return m.alignment ? `${what}, ${m.alignment.replace(/^\w/, (ch: string) => ch.toUpperCase())}` : what;
}

/** "40 ft., Climb 40 ft., Fly 80 ft. (hover)". Walk first, without a name. */
export function speed(speeds: Record<string, string | boolean> | undefined): string {
  if (!speeds) return "";
  const hover = speeds.hover === true;
  return Object.entries(speeds)
    .filter(([, v]) => typeof v === "string")
    .sort(([a], [b]) => (a === "walk" ? -1 : b === "walk" ? 1 : 0))
    .map(([kind, v]) => (kind === "walk" ? v : `${title(kind)} ${v}${kind === "fly" && hover ? ` (${t.hover})` : ""}`))
    .join(", ") as string;
}

export const hitPoints = (m: Record<string, any>) => (m.hit_dice ? `${m.hit_points} (${m.hit_dice})` : String(m.hit_points ?? ""));

/** "DEX +6, WIS +7" from `Saving Throw: DEX`, or "Perception +13" from `Skill: Perception`. */
export function proficiencies(m: Record<string, any>, kind: "Saving Throw" | "Skill"): string {
  return (m.proficiencies ?? [])
    .filter((p: any) => p.proficiency?.name?.startsWith(`${kind}:`))
    .map((p: any) => `${p.proficiency.name.slice(kind.length + 1).trim()} ${signed(p.value)}`)
    .join(", ");
}

const words = (list: unknown) => (Array.isArray(list) ? list.map((v) => (typeof v === "string" ? title(v) : v?.name)).filter(Boolean) : []);
export const damages = (list: unknown) => words(list).join(", ");
/** "Fire; Charmed, Frightened": damage immunities, then condition immunities. */
export const immunities = (m: Record<string, any>) => [words(m.damage_immunities).join(", "), names(m.condition_immunities).join(", ")].filter(Boolean).join("; ");

/** "Darkvision 120 ft., Blindsight 60 ft.; Passive Perception 23". */
export function senses(s: Record<string, any> | undefined): string {
  if (!s) return "";
  const others = Object.entries(s)
    .filter(([k]) => k !== "passive_perception")
    .map(([k, v]) => `${title(k)} ${v}`)
    .join(", ");
  const passive = s.passive_perception !== undefined ? t.passivePerception(s.passive_perception) : "";
  return [others, passive].filter(Boolean).join("; ");
}

/** "17 (XP 18,000; PB +6)". */
export function challenge(m: Record<string, any>): string {
  const extra = [m.xp !== undefined && t.xp(number.format(m.xp)), m.proficiency_bonus !== undefined && t.pb(signed(m.proficiency_bonus))].filter(Boolean);
  return extra.length ? `${crText(m.challenge_rating)} (${extra.join("; ")})` : crText(m.challenge_rating);
}

export const ACTION_LISTS = [
  { id: "special_abilities", label: t.traits },
  { id: "actions", label: t.actions },
  { id: "bonus_actions", label: t.bonusActions },
  { id: "reactions", label: t.reactions },
  { id: "legendary_actions", label: t.legendaryActions },
] as const;

// Classes

export type TableColumn = { id: string; label: string; cell: (level: Record<string, any>) => string };

const cellValue = (v: unknown): string => {
  if (typeof v === "number") return number.format(v);
  if (v && typeof v === "object" && "dice_count" in v) return `${(v as any).dice_count}d${(v as any).dice_value}`;
  return "";
};
const SLOT = /^spell_slots_level_(\d)$/;

/** The class table's extra columns: each `class_specific` and `spellcasting` value some level has (not 0 everywhere). */
export function levelColumns(levels: Record<string, any>[]): TableColumn[] {
  const columns: TableColumn[] = [];
  for (const group of ["class_specific", "spellcasting"] as const) {
    const keys = [...new Set(levels.flatMap((l) => Object.keys(l[group] ?? {})))];
    for (const key of keys) {
      const cell = (l: Record<string, any>) => cellValue(l[group]?.[key]);
      if (!levels.some((l) => cell(l) && cell(l) !== "0")) continue;
      const slot = SLOT.exec(key);
      const label = slot ? t.slots(Number(slot[1])) : key === "cantrips_known" ? t.cantrips : key === "prepared_spells" ? t.preparedSpells : title(key);
      columns.push({ id: `${group}.${key}`, label, cell: (l) => (cell(l) === "0" ? "—" : cell(l) || "—") });
    }
  }
  return columns;
}

export const classNames = names;
