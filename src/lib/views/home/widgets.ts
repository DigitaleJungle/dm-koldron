// The widgets on a campaign's Home: their ids (as WIDGET_IDS in src-tauri/src/commands/home.rs), default order and
// widths, and moving and sizing them.
import type { HomeLayout } from "$lib/api/home";

export const WIDGET_IDS = ["party", "nextSession", "summary", "encounter"] as const;
export type BuiltInId = (typeof WIDGET_IDS)[number];
/** A built-in widget, or an addon's: `addon:<id>`. */
export type WidgetId = BuiltInId | `addon:${string}`;
/** Columns (of 3) a widget spans. */
export type Width = 1 | 2 | 3;
export const WIDTHS: readonly Width[] = [1, 2, 3];

const DEFAULT_WIDTHS: Record<BuiltInId, Width> = { party: 2, nextSession: 1, summary: 2, encounter: 1 };

/** The saved layout as Home uses it: known widgets only (the built-in ones and `addonWidgets`, the loaded addons'),
 *  each once, any missing ones after them (unless removed); widths 1 to 3 (an addon's widget 1 by default). An addon
 *  widget that isn't loaded is left out, so the next save forgets it and it comes back at the end. */
export function fromSaved(
  saved: HomeLayout | null,
  addonWidgets: WidgetId[] = [],
): {
  order: WidgetId[];
  widths: Record<WidgetId, Width>;
  locked: boolean;
  removed: WidgetId[];
} {
  const all: WidgetId[] = [...WIDGET_IDS, ...addonWidgets];
  const isWidget = (id: string): id is WidgetId => (all as string[]).includes(id);
  const removed = [...new Set((saved?.removed ?? []).filter(isWidget))];
  const order = [...new Set([...(saved?.widgets ?? []).filter(isWidget), ...all])].filter((id) => !removed.includes(id));
  const widths: Record<WidgetId, Width> = { ...DEFAULT_WIDTHS };
  for (const id of addonWidgets) widths[id] = 1;
  for (const id of all) {
    const w = saved?.widths[id];
    if (w === 1 || w === 2 || w === 3) widths[id] = w;
  }
  return { order, widths, locked: saved?.locked ?? false, removed };
}

/** Moves `id` to position `to` (clamped to the list). */
export function moveWidget(order: WidgetId[], id: WidgetId, to: number): WidgetId[] {
  const rest = order.filter((w) => w !== id);
  const at = Math.max(0, Math.min(to, rest.length));
  return [...rest.slice(0, at), id, ...rest.slice(at)];
}

/** The width a widget's right edge dragged to `x` snaps to: the nearest whole number of columns from its left edge
 *  (`left`), in a grid whose columns are `column` wide with `gap` between them; 1 to 3. */
export function snapWidth(x: number, left: number, column: number, gap: number): Width {
  const columns = Math.round((x - left + gap) / (column + gap));
  return Math.max(1, Math.min(3, columns)) as Width;
}

/** A random item that isn't in `avoid` (the ones already shown); null when there's none left. */
export function pickRandom<T>(items: T[], avoid: readonly T[], random = Math.random): T | null {
  const pool = items.filter((i) => !avoid.includes(i));
  return pool.length ? pool[Math.floor(random() * pool.length)] : null;
}

/** `shown` grown or shrunk to `count` items, the new ones picked at random from the rest. */
export function fillRandom<T>(items: T[], shown: readonly T[], count: number, random = Math.random): T[] {
  const out = shown.filter((i) => items.includes(i)).slice(0, count);
  for (let next = pickRandom(items, out, random); out.length < count && next !== null; next = pickRandom(items, out, random)) out.push(next);
  return out;
}
