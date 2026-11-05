import { invoke } from "@tauri-apps/api/core";

/** Mirrors `ContentsChapter` in src-tauri/src/commands/rules.rs. */
export type ContentsChapter = {
  pack: string;
  packName: string;
  index: string;
  name: string;
  sections: { index: string; name: string }[];
};

/** Mirrors `Chapter`: names as text, `html` rendered from the pack's Markdown with raw HTML, links and images removed. */
export type Chapter = { name: string; html: string; sections: { index: string; name: string; html: string }[] };

/** The chapters of the switched-on packs, in pack order. Rejects with a StorageError. */
export function rulesContents(): Promise<ContentsChapter[]> {
  return invoke("rules_contents");
}

/** One chapter with its sections. Rejects with a StorageError (`notFound` when it isn't in a switched-on pack). */
export function rulesChapter(pack: string, index: string): Promise<Chapter> {
  return invoke("rules_chapter", { pack, index });
}

/** One rule section as HTML. Rejects with a StorageError (`notFound` when it isn't in a switched-on pack). */
export function rulesSection(pack: string, index: string): Promise<Chapter["sections"][number]> {
  return invoke("rules_section", { pack, index });
}
