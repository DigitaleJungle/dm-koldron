import { describe, expect, it } from "vitest";
import type { ContentsChapter } from "$lib/api/rules";
import { booksOf } from "./rulesContents";

const chapter = (pack: string, packName: string, name: string): ContentsChapter => ({ pack, packName, index: name.toLowerCase(), name, sections: [] });
const contents = [
  chapter("srd", "SRD 5.2.1", "Playing the Game"),
  chapter("srd", "SRD 5.2.1", "Combat"),
  chapter("phb", "Player's Handbook", "Combat Options"),
  chapter("phb", "Player's Handbook", "Spells"),
];
const shape = (search?: string) => booksOf(contents, search).map((b) => [b.name, b.chapters.map((c) => c.name)]);

describe("booksOf", () => {
  it("groups the chapters by book, in their order", () => {
    expect(shape()).toEqual([
      ["SRD 5.2.1", ["Playing the Game", "Combat"]],
      ["Player's Handbook", ["Combat Options", "Spells"]],
    ]);
  });

  it("keeps the chapters with every typed word, and all of a book whose name matches", () => {
    expect(shape("COMBAT")).toEqual([
      ["SRD 5.2.1", ["Combat"]],
      ["Player's Handbook", ["Combat Options"]],
    ]);
    expect(shape("combat opt")).toEqual([["Player's Handbook", ["Combat Options"]]]);
    expect(shape("handbook")).toEqual([["Player's Handbook", ["Combat Options", "Spells"]]]);
    expect(shape("nothing like it")).toEqual([]);
  });
});
