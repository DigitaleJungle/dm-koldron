// Copy as homebrew: a Catalog entry as a new homebrew draft, with what its page brings along (a species' traits, a
// class's levels and features) filled into the form's linked fields. Its index, address and picture stay behind:
// they belong to its own pack.
import type { CatalogEntry } from "$lib/api/catalog";
import type { PackEntry } from "$lib/api/homebrew";
import { allFields, type EntryType } from "./entryTypes";
import { levelRest, type Item, type Level } from "./linked";

const text = (v: unknown) => (typeof v === "string" ? v : "");
const refIndex = (r: unknown) => text((r as { index?: unknown } | null)?.index);

export function copyDraft(type: EntryType, loaded: CatalogEntry): PackEntry {
  const { index: _index, url: _url, image: _image, ...fields } = structuredClone(loaded.entry);
  const draft: PackEntry = { ...fields, index: "", name: text(fields.name) };
  const related = (kind: string) => loaded.related.filter((r) => r.kind === kind).map((r) => r.entry);
  for (const f of allFields(type.fields)) {
    if (f.input === "linked") {
      const found = new Map(related(f.of).map((e) => [e.index, e]));
      draft[f.key] = (Array.isArray(fields[f.key]) ? fields[f.key] : []).flatMap((r: unknown): Item[] => {
        const e = found.get(refIndex(r));
        return e ? [{ name: text(e.name), desc: text(e[f.text]) }] : [];
      });
    } else if (f.input === "levels") {
      const features = new Map(related("features").map((e) => [e.index, e]));
      draft[f.key] = related("levels").map(
        (l): Level => ({
          ...levelRest(l),
          level: Number(l.level),
          prof_bonus: typeof l.prof_bonus === "number" ? l.prof_bonus : undefined,
          features: (Array.isArray(l.features) ? l.features : []).flatMap((r: unknown): Item[] => {
            const e = features.get(refIndex(r));
            return e ? [{ name: text(e.name), desc: text(e.description) }] : [];
          }),
        }),
      );
    }
  }
  return draft;
}
