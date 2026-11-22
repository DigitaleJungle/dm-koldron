// Entries a form edits together with its own entry, though the pack keeps them apart (`packs/packs.md`): a species'
// traits (`traits` entries it lists), and a class's level table (`levels` entries naming the class, each listing
// `features` entries). In the form they sit in the draft under the field's key; saving writes them back as entries.
import type { Book, PackEntry } from "$lib/api/homebrew";
import { freeIndex, withoutEntry } from "./book";
import { allFields, reference, type EntryType, type Field } from "./entryTypes";

/** A named text as the form edits it; `index` once saved. Its text is `desc` unless its field names another. */
export type Item = { index?: string; name: string; desc?: unknown } & Record<string, unknown>;
/** A level of a class's table as the form edits it, with the level's fields the form doesn't show (its table
 *  columns: `class_specific`, `spellcasting`). */
export type Level = { level: number; prof_bonus?: number; features: Item[] } & Record<string, unknown>;

/** A level entry's fields that the form doesn't edit or that are made again on save. */
export function levelRest(l: Record<string, unknown>): Record<string, unknown> {
  const { index: _i, name: _n, url: _u, class: _c, subclass: _s, level: _l, prof_bonus: _p, features: _f, ...rest } = l;
  return rest;
}

type Linked = Field & { input: "linked" };
const text = (v: unknown) => (typeof v === "string" ? v : "");
const refIndex = (r: unknown) => text((r as { index?: unknown } | null)?.index);
const entries = (book: Book, kind: string) => book.entries[kind] ?? [];

/** A class's own levels in the book (not a subclass's), by level. */
const levelsOf = (book: Book, cls: string) =>
  entries(book, "levels")
    .filter((l) => refIndex(l.class) === cls && !l.subclass)
    .toSorted((a, b) => Number(a.level) - Number(b.level));

/** The draft a form starts from: the entry with its linked entries filled in under their fields' keys. */
export function openEntry(book: Book, type: EntryType, entry: PackEntry): PackEntry {
  const draft = structuredClone(entry);
  if (!entry.index) return draft; // a new entry (a copy too) already has its texts, not references
  for (const f of allFields(type.fields)) {
    if (f.input === "linked") {
      const listed = Array.isArray(entry[f.key]) ? (entry[f.key] as unknown[]) : [];
      draft[f.key] = listed.flatMap((r) => {
        const found = entries(book, f.of).find((e) => e.index === refIndex(r));
        return found ? [{ index: found.index, name: found.name, desc: text(found[f.text]) }] : [];
      });
    } else if (f.input === "levels" && entry.index) {
      const features = entries(book, "features");
      draft[f.key] = levelsOf(book, entry.index).map((l) => ({
        ...levelRest(l),
        level: Number(l.level),
        prof_bonus: typeof l.prof_bonus === "number" ? l.prof_bonus : undefined,
        features: (Array.isArray(l.features) ? l.features : []).flatMap((r: unknown) => {
          const found = features.find((e) => e.index === refIndex(r));
          return found ? [{ index: found.index, name: found.name, desc: text(found.description) }] : [];
        }),
      }));
    }
  }
  return draft;
}

/** The items' indexes in `kind`: kept ones stay, new ones are made from `base(item)` and never taken. */
function indexes(items: Item[], kind: string, book: Book, mine: Set<string>, base: (item: Item) => string): string[] {
  const kept = (i: Item) => !!i.index && mine.has(i.index);
  const taken = new Set([...entries(book, kind).map((e) => e.index).filter((i) => !mine.has(i)), ...items.filter(kept).map((i) => i.index!)]);
  return items.map((item) => {
    const index = kept(item) ? item.index! : freeIndex(base(item), taken);
    taken.add(index);
    return index;
  });
}

/** Puts `saved` in `kind` in place of the entries in `mine` (by index; fields the form doesn't edit are kept). */
function replace(book: Book, kind: string, mine: Set<string>, saved: PackEntry[]): Book {
  const old = new Map(entries(book, kind).map((e) => [e.index, e]));
  const merged = saved.map((e) => ({ ...old.get(e.index), ...e }));
  return { ...book, entries: { ...book.entries, [kind]: [...entries(book, kind).filter((e) => !mine.has(e.index)), ...merged] } };
}

/** Saves a draft from `openEntry`: the entry, and its linked entries in their own types. Returns the book and the
 *  entry's index. */
export function saveEntry(book: Book, type: EntryType, draft: PackEntry): { book: Book; index: string } {
  const own = structuredClone(draft);
  own.name = own.name.trim();
  const index = own.index || freeIndex(own.name, new Set(entries(book, type.id).map((e) => e.index)));
  own.index = index;
  const self = reference(type.id, index, own.name);
  let next = book;
  for (const f of allFields(type.fields)) {
    if (f.input === "linked") {
      const items = (Array.isArray(own[f.key]) ? own[f.key] : []) as Item[];
      const mine = linkedTo(book, type, f, index);
      const ids = indexes(items, f.of, book, mine, (i) => i.name);
      const saved = items.map((item, i) => ({ index: ids[i], name: item.name.trim(), [f.text]: text(item.desc), [f.back]: [self] }));
      next = replace(next, f.of, mine, saved);
      own[f.key] = saved.map((e) => reference(f.of, e.index, e.name));
      if (!saved.length) delete own[f.key];
    } else if (f.input === "levels") {
      const rows = ((Array.isArray(own[f.key]) ? own[f.key] : []) as Level[]).toSorted((a, b) => a.level - b.level);
      delete own[f.key];
      const oldLevels = levelsOf(book, draft.index);
      const mineLevels = new Set(oldLevels.map((l) => l.index));
      const mineFeatures = new Set(oldLevels.flatMap((l) => (Array.isArray(l.features) ? l.features : []).map(refIndex)));
      // A level keeps its entry by its number; a new one gets an index like the SRD's ("wizard-3").
      const keptLevels = rows.map((r) => oldLevels.find((l) => Number(l.level) === r.level)?.index);
      const takenLevels = new Set([...entries(book, "levels").map((e) => e.index).filter((i) => !mineLevels.has(i)), ...keptLevels.filter((i) => i !== undefined)]);
      const levelIds = rows.map((r, i) => {
        const id = keptLevels[i] ?? freeIndex(`${index} ${r.level}`, takenLevels);
        takenLevels.add(id);
        return id;
      });
      const allItems = rows.flatMap((r) => r.features);
      const featureIds = indexes(allItems, "features", book, mineFeatures, (i) => `${index} ${i.name}`);
      let k = 0;
      const features: PackEntry[] = [];
      const levels = rows.map((r, i) => {
        const level = { index: levelIds[i], name: `${own.name} ${r.level}` };
        const listed = r.features.map((item) => {
          const feature = { index: featureIds[k++], name: item.name.trim(), class: self, level: reference("levels", level.index, level.name), description: text(item.desc) };
          features.push(feature);
          return reference("features", feature.index, feature.name);
        });
        const { level: _l, prof_bonus: _p, features: _f, ...rest } = r;
        const entry: PackEntry = { ...rest, ...level, level: r.level, features: listed, class: self };
        if (typeof r.prof_bonus === "number") entry.prof_bonus = r.prof_bonus;
        return entry;
      });
      next = replace(replace(next, "features", mineFeatures, features), "levels", mineLevels, levels);
    }
  }
  const list = entries(next, type.id);
  const saved = draft.index ? list.map((e) => (e.index === index ? own : e)) : [...list, own];
  return { book: { ...next, entries: { ...next.entries, [type.id]: saved } }, index };
}

/** The `f.of` entries this entry lists that no other entry of its type does (so they go when it does). */
function linkedTo(book: Book, type: EntryType, f: Linked, index: string): Set<string> {
  const entry = entries(book, type.id).find((e) => e.index === index);
  const listed = (Array.isArray(entry?.[f.key]) ? (entry![f.key] as unknown[]) : []).map(refIndex);
  return new Set(
    listed.filter((i) => {
      const target = entries(book, f.of).find((e) => e.index === i);
      const back = Array.isArray(target?.[f.back]) ? (target![f.back] as unknown[]).map(refIndex) : [];
      return back.every((b) => b === index);
    }),
  );
}

/** The book without the entry and its linked entries. */
export function deleteEntry(book: Book, type: EntryType, index: string): Book {
  let next = book;
  for (const f of allFields(type.fields)) {
    if (f.input === "linked") {
      const mine = linkedTo(book, type, f, index);
      next = replace(next, f.of, mine, []);
    } else if (f.input === "levels") {
      const levels = levelsOf(book, index);
      const features = new Set(levels.flatMap((l) => (Array.isArray(l.features) ? l.features : []).map(refIndex)));
      next = replace(replace(next, "features", features, []), "levels", new Set(levels.map((l) => l.index)), []);
    }
  }
  return withoutEntry(next, type.id, index);
}
