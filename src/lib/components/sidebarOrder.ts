/** The items in the saved order; items the saved order doesn't name (new ones) follow in their default order. */
export function ordered<T extends { id: string }>(items: T[], saved: string[]): T[] {
  const at = (id: string) => {
    const i = saved.indexOf(id);
    return i < 0 ? saved.length : i;
  };
  return items
    .map((item, i) => ({ item, i }))
    .sort((a, b) => at(a.item.id) - at(b.item.id) || a.i - b.i)
    .map((x) => x.item);
}

/** The list with the entry at `from` moved to `to`; the same list when either is out of range. */
export function moveTo<T>(list: T[], from: number, to: number): T[] {
  if (from === to || from < 0 || to < 0 || from >= list.length || to >= list.length) return list;
  const next = [...list];
  next.splice(to, 0, ...next.splice(from, 1));
  return next;
}
