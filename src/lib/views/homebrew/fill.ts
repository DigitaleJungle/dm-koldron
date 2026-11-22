// Fields an addon sends into an open Homebrew form (`homebrew.fill`), merged so nothing typed is lost: an empty field
// takes the value; a text the new one continues (it starts with it) grows to it; a list gets the items it doesn't have
// (an item with the same name, index or proficiency is merged the same way); everything else stays as it is.

type Obj = Record<string, unknown>;
const isObj = (v: unknown): v is Obj => typeof v === "object" && v !== null && !Array.isArray(v);
const isEmpty = (v: unknown): boolean =>
  v === undefined || v === null || v === "" || (Array.isArray(v) && v.length === 0) || (isObj(v) && Object.values(v).every(isEmpty));

/** What tells two list items apart: a proficiency's index, else the item's name, else its index (a saved section's index
 *  is made from its name, so the name is what text sent in shares with it). */
function keyOf(v: unknown): string {
  if (!isObj(v)) return JSON.stringify(v);
  const p = v.proficiency;
  const k = isObj(p) ? p.index : v.name || v.index;
  return typeof k === "string" ? k.toLowerCase() : JSON.stringify(v);
}

/** Merges `fields` into `target` (changed in place). Its `index` is never changed. */
export function fill(target: Obj, fields: Obj): void {
  for (const [k, value] of Object.entries(fields)) {
    if (k === "index") continue;
    const current = target[k];
    if (isEmpty(current)) {
      if (!isEmpty(value)) target[k] = structuredClone(value);
    } else if (typeof current === "string" && typeof value === "string") {
      if (value.length > current.length && value.startsWith(current)) target[k] = value;
    } else if (Array.isArray(current) && Array.isArray(value)) {
      for (const item of value) {
        if (typeof item === "string") {
          const at = current.findIndex((c) => typeof c === "string" && item.startsWith(c));
          if (at < 0) current.push(item);
          else if (item.length > (current[at] as string).length) current[at] = item;
          continue;
        }
        const same = current.find((c) => keyOf(c) === keyOf(item));
        if (same === undefined) current.push(structuredClone(item));
        else if (isObj(same) && isObj(item)) fill(same, item);
      }
    } else if (isObj(current) && isObj(value)) fill(current, value);
  }
}
