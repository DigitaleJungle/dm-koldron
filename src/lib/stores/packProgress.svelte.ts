/** Work an addon reports on a pack (`packs.setProgress`), shown in its Settings → Rules row. `fraction` 0–1, or null
 *  when it can't tell; `name` labels the row of a pack not installed yet. `owner`: the frame that set it. */
export type PackProgress = { owner: symbol; name: string; fraction: number | null; text: string };

/** By pack id. */
export const packProgress = $state<Record<string, PackProgress>>({});

/** Clears everything one frame set: when it closes, its work stops. */
export function clearProgressOf(owner: symbol) {
  for (const [pack, p] of Object.entries(packProgress)) if (p.owner === owner) delete packProgress[pack];
}
