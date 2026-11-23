import { invoke } from "@tauri-apps/api/core";
import { campaigns } from "$lib/stores/campaigns.svelte";

/** Mirrors `ContentsChapter` in src-tauri/src/commands/rules.rs. */
export type ContentsChapter = {
  pack: string;
  packName: string;
  index: string;
  name: string;
  sections: { index: string; name: string }[];
};

/** Mirrors `Chapter`: names as text, `html` rendered from the pack's Markdown with raw HTML, links and images removed. */
/** `image`: a picture kept for the chapter's pack (a homebrew chapter's or section's), if it has one. */
export type Chapter = { name: string; html: string; image: string | null; sections: { index: string; name: string; html: string; image: string | null }[] };

// Each takes the packs of the campaign worked in, if any (`campaigns.scope`).

/** The chapters of the switched-on packs, in pack order. Rejects with a StorageError. */
export function rulesContents(): Promise<ContentsChapter[]> {
  return invoke("rules_contents", { campaign: campaigns.scope });
}

/** One chapter with its sections. Rejects with a StorageError (`notFound` when it isn't in a switched-on pack). */
export function rulesChapter(pack: string, index: string): Promise<Chapter> {
  return invoke("rules_chapter", { pack, index, campaign: campaigns.scope });
}

/** One rule section as HTML. Rejects with a StorageError (`notFound` when it isn't in a switched-on pack). */
export function rulesSection(pack: string, index: string, campaign: string | null = campaigns.scope): Promise<Chapter["sections"][number]> {
  return invoke("rules_section", { pack, index, campaign });
}
