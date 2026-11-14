import { invoke } from "@tauri-apps/api/core";

/** Mirrors `Layout` in src-tauri/src/commands/home.rs: the widgets in order, the columns (of 3) each spans (one
 *  without spans 1), whether they're locked (can't be moved or resized), and the widgets taken off Home. */
export type HomeLayout = { widgets: string[]; widths: Record<string, number>; locked: boolean; removed?: string[] };

// Both reject with a StorageError.
/** The campaign's saved layout, or null when nothing was saved yet. */
export function getHomeLayout(campaign: string): Promise<HomeLayout | null> {
  return invoke("get_home_layout", { campaign });
}

export function setHomeLayout(campaign: string, layout: HomeLayout): Promise<void> {
  return invoke("set_home_layout", { campaign, layout });
}
