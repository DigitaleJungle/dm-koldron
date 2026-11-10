import { invoke } from "@tauri-apps/api/core";

/** Mirrors `Roll` in src-tauri/src/dice.rs. */
export type Roll = { notation: string; total: number; dice: { faces: number; value: number; kept: boolean }[] };

/** Rolls dice notation like `1d20+2` or `2d20kh1+4` with the one Rust dice parser. Rejects with a StorageError. */
export function rollDice(notation: string): Promise<Roll> {
  return invoke("roll_dice", { notation });
}
