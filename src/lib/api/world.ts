import { invoke } from "@tauri-apps/api/core";
import type { Picture } from "./campaigns";
import type { Pin } from "./encounters";

/** The campaign's locations and NPCs, two trees. Mirrors src-tauri/src/commands/world.rs. `parent`: null at the top.
 *  Atmosphere, purpose and secrets are rich text. `images`: its image files' names, in order; `pictures` (one per name)
 *  and `map` are set by Rust only (ignored when saving). */
export type Location = {
  id: string;
  parent: string | null;
  name: string;
  atmosphere: string;
  purpose: string;
  secrets: string;
  npcIds: string[];
  pins: Pin[];
  images: string[];
  pictures?: Picture[];
  map?: Picture | null;
};

export type Npc = {
  id: string;
  parent: string | null;
  name: string;
  race: string;
  gender: string;
  age: string;
  job: string;
  alignment: string;
  /** Rich text, as are motivation, traits and backstory. */
  summary: string;
  motivation: string;
  traits: string;
  backstory: string;
  stats: NpcStats;
  /** Set by Rust, never saved: stored at once with `setNpcPortrait`. */
  portrait?: Picture | null;
};

/** An NPC's basic stats, each a short text (as written, or copied from a catalog creature). */
export type NpcStats = {
  ac: string;
  hp: string;
  speed: string;
  cr: string;
  str: string;
  dex: string;
  con: string;
  int: string;
  wis: string;
  cha: string;
};

/** A folder in the NPCs tree: NPCs (and other groups) sit in it; NPCs never sit in each other. */
export type NpcGroup = { id: string; parent: string | null; name: string };

export type World = { locations: Location[]; npcGroups: NpcGroup[]; npcs: Npc[] };

// All of these reject with a StorageError.
export function getWorld(campaign: string): Promise<World> {
  return invoke("get_world", { campaign });
}

/** Replaces the saved world. Returns it as saved. */
export function saveWorld(campaign: string, world: World): Promise<World> {
  return invoke("save_world", { campaign, world });
}

/** Stores the picture file at `path` (from `choosePictureFile`) as a new image of the location; add its `name` to the
 *  location's `images` (and the picture to `pictures`) for it to be kept when the world is saved. */
export function addLocationImage(campaign: string, location: string, path: string): Promise<{ name: string; picture: Picture }> {
  return invoke("add_location_image", { campaign, location, path });
}

/** Makes the picture file at `path` the NPC's portrait, or removes it (`null`); saved at once. */
export function setNpcPortrait(campaign: string, npc: string, path: string | null): Promise<Picture | null> {
  return invoke("set_npc_portrait", { campaign, npc, path });
}

/** Makes the picture file at `path` the location's map, or removes it (`null`); saved at once. */
export function setLocationMap(campaign: string, location: string, path: string | null): Promise<Picture | null> {
  return invoke("set_location_map", { campaign, location, path });
}
