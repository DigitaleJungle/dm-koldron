/** What Settings → Rules saves: the saved lists of switched-off packs and of packs with images on, with the changes
 *  made since (pack id → on). Packs without a change, loaded or not, keep what's saved. */
export function applyPending(
  saved: { disabledPacks: string[]; imagePacks: string[] },
  packs: Record<string, boolean>,
  images: Record<string, boolean>,
): { disabledPacks: string[]; imagePacks: string[] } {
  const disabled = new Set(saved.disabledPacks);
  for (const [id, on] of Object.entries(packs)) {
    if (on) disabled.delete(id);
    else disabled.add(id);
  }
  const withImages = new Set(saved.imagePacks);
  for (const [id, on] of Object.entries(images)) {
    if (on) withImages.add(id);
    else withImages.delete(id);
  }
  return { disabledPacks: [...disabled], imagePacks: [...withImages] };
}
