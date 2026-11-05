import { invoke } from "@tauri-apps/api/core";
import type { Picture } from "./campaigns";

export type ClassLevel = { name: string; level: number };

/** Mirrors `Character` in src-tauri/src/commands/characters.rs. Stats are whole numbers or null; times are ms. */
export type Character = {
  /** Empty for a character that isn't saved yet. */
  id: string;
  name: string;
  player: string;
  classes: ClassLevel[];
  maxHp: number | null;
  currentHp: number | null;
  tempHp: number | null;
  ac: number | null;
  passivePerception: number | null;
  passiveInsight: number | null;
  passiveInvestigation: number | null;
  onHome: boolean;
  createdAt: number;
  updatedAt: number;
  picture?: Picture;
};

/** `unreadable`: files that couldn't be read as a character; they are left untouched. */
export type CharacterList = { characters: Character[]; unreadable: string[] };

// All of these reject with a StorageError.
export function listCharacters(campaign: string): Promise<CharacterList> {
  return invoke("list_characters", { campaign });
}

/** Creates the character when its `id` is empty, else replaces it. Returns it as saved. */
export function saveCharacter(campaign: string, character: Character): Promise<Character> {
  return invoke("save_character", { campaign, character });
}

export function deleteCharacter(campaign: string, id: string): Promise<void> {
  return invoke("delete_character", { campaign, id });
}

/** Sets the picture from an image's bytes (Rust decodes, shrinks and stores it), or removes it (`null`). */
export function setCharacterPicture(campaign: string, id: string, bytes: Uint8Array | null): Promise<Character> {
  return invoke("set_character_picture", { campaign, id, bytes: bytes && Array.from(bytes) });
}
