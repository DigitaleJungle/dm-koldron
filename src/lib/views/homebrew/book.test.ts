import { describe, expect, it } from "vitest";
import { blankBook, chaptersOf, freeIndex, slug, withChapter, withoutChapter } from "./book";
import { getField, setField } from "./entryTypes";

describe("homebrew book texts", () => {
  it("makes indexes from names, never one that's taken", () => {
    expect(slug(" The Old Keep! ")).toBe("the-old-keep");
    expect(slug("¿?")).toBe("entry");
    expect(freeIndex("Keep", new Set(["keep", "keep-2"]))).toBe("keep-3");
  });

  it("adds a chapter with its sections, then edits and reorders them keeping their indexes", () => {
    const empty = blankBook("2024", "Homebrew");
    const added = withChapter(empty, {
      index: "",
      name: "World",
      desc: "Intro",
      sections: [
        { index: "", name: "The city", desc: "**Big.**" },
        { index: "", name: "The city", desc: "Again" },
      ],
    });
    expect(added.index).toBe("world");
    const [chapter] = chaptersOf(added.book);
    expect(chapter.sections.map((s) => s.index)).toEqual(["the-city", "the-city-2"]);

    // Renamed, reordered, one removed and one added: indexes stay, the removed section is gone.
    const edited = withChapter(added.book, {
      ...chapter,
      name: "The world",
      sections: [{ ...chapter.sections[1], name: "Second" }, { index: "", name: "Docks", desc: "" }],
    });
    expect(edited.index).toBe("world");
    expect(chaptersOf(edited.book)).toEqual([
      { index: "world", name: "The world", desc: "Intro", sections: [
        { index: "the-city-2", name: "Second", desc: "Again" },
        { index: "docks", name: "Docks", desc: "" },
      ] },
    ]);
    expect(edited.book.entries["rule-sections"]).toHaveLength(2);
  });

  it("never gives a new section the index of a kept one further down", () => {
    const first = withChapter(blankBook("2024", "Homebrew"), { index: "", name: "A", desc: "", sections: [{ index: "", name: "Docks", desc: "" }] });
    const [chapter] = chaptersOf(first.book);
    const second = withChapter(first.book, { ...chapter, sections: [{ index: "", name: "Docks", desc: "" }, ...chapter.sections] });
    expect(chaptersOf(second.book)[0].sections.map((s) => s.index)).toEqual(["docks-2", "docks"]);
  });

  it("keeps a chapter's and a section's picture, and leaves it out once removed", () => {
    const added = withChapter(blankBook("2024", "Homebrew"), { index: "", name: "A", desc: "", image: "homebrew-1.png", sections: [{ index: "", name: "One", desc: "", image: "homebrew-2.png" }] });
    const [chapter] = chaptersOf(added.book);
    expect([chapter.image, chapter.sections[0].image]).toEqual(["homebrew-1.png", "homebrew-2.png"]);
    const removed = withChapter(added.book, { ...chapter, image: undefined, sections: [{ ...chapter.sections[0], image: undefined }] });
    expect([removed.book.entries.rules![0].image, removed.book.entries["rule-sections"]![0].image]).toEqual([undefined, undefined]);
    expect("image" in removed.book.entries.rules![0]).toBe(false);
  });

  it("removes a chapter with its sections only", () => {
    let book = withChapter(blankBook("2024", "Homebrew"), { index: "", name: "A", desc: "", sections: [{ index: "", name: "One", desc: "" }] }).book;
    book = withChapter(book, { index: "", name: "B", desc: "", sections: [{ index: "", name: "Two", desc: "" }] }).book;
    const left = withoutChapter(book, "a");
    expect(chaptersOf(left).map((c) => c.index)).toEqual(["b"]);
    expect(left.entries["rule-sections"]!.map((s) => s.index)).toEqual(["two"]);
  });

  it("reads and writes fields inside fields, leaving out empty ones", () => {
    const item = { index: "x", name: "X" };
    setField(item, "cost.quantity", 5);
    setField(item, "cost.unit", "gp");
    expect(getField(item, "cost.unit")).toBe("gp");
    setField(item, "cost.unit", "");
    expect(item).toEqual({ index: "x", name: "X", cost: { quantity: 5 } });
    expect(getField(item, "rarity.name")).toBeUndefined();
    setField(item, "armor_class.0.value", 15);
    expect((item as Record<string, unknown>).armor_class).toEqual([{ value: 15 }]);
    // Cleared, what held it goes too, but not what still holds something else.
    setField(item, "armor_class.0.value", undefined);
    setField(item, "cost.quantity", undefined);
    expect(item).toEqual({ index: "x", name: "X" });
    setField(item, "speed.walk", "30 ft.");
    setField(item, "speed.fly", "60 ft.");
    setField(item, "speed.fly", "");
    expect(item).toEqual({ index: "x", name: "X", speed: { walk: "30 ft." } });
  });
});
