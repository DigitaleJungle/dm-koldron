import { invoke } from "@tauri-apps/api/core";

/** The campaign's locations and NPCs, two trees. Mirrors src-tauri/src/commands/world.rs. `parent`: null at the top. */
export type Location = { id: string; parent: string | null; name: string; description: string };

export type Npc = { id: string; parent: string | null; name: string; wants: string; voice: string };

export type World = { locations: Location[]; npcs: Npc[] };

// Both reject with a StorageError.
export function getWorld(campaign: string): Promise<World> {
  return invoke("get_world", { campaign });
}

/** Replaces the saved world. Returns it as saved. */
export function saveWorld(campaign: string, world: World): Promise<World> {
  return invoke("save_world", { campaign, world });
}
