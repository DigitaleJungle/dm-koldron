import { describe, expect, it } from "vitest";
import { blankBook } from "./book";
import { entryType } from "./entryTypes";
import { deleteEntry, openEntry, saveEntry, type Level } from "./linked";

const species = entryType("species")!;
const spells = entryType("spells")!;
const classes = entryType("classes")!;

describe("linked entries", () => {
  it("adds, edits and removes a plain entry, keeping its index and fields the form doesn't edit", () => {
    const one = saveEntry(blankBook("2024", "Homebrew"), spells, { index: "", name: "Fire Bolt ", level: 0 });
    const two = saveEntry(one.book, spells, { index: "", name: "Fire bolt", level: 1 });
    expect([one.index, two.index]).toEqual(["fire-bolt", "fire-bolt-2"]);
    const edited = saveEntry(two.book, spells, { ...two.book.entries.spells![0], name: "Firebolt" });
    expect(edited.book.entries.spells!.map((s) => [s.index, s.name, s.level])).toEqual([["fire-bolt", "Firebolt", 0], ["fire-bolt-2", "Fire bolt", 1]]);
    expect(deleteEntry(edited.book, spells, "fire-bolt").entries.spells!.map((s) => s.index)).toEqual(["fire-bolt-2"]);
  });

  it("saves a species' traits as trait entries pointing back at it, and opens them again", () => {
    const first = saveEntry(blankBook("2024", "Homebrew"), species, {
      index: "",
      name: "Owlfolk",
      speed: 30,
      traits: [{ name: "Flight", desc: "You fly." }, { name: "Night Eyes", desc: "You see." }],
    });
    expect(first.index).toBe("owlfolk");
    const [owl] = first.book.entries.species!;
    expect(owl.traits).toEqual([
      { index: "flight", name: "Flight", url: "/api/traits/flight" },
      { index: "night-eyes", name: "Night Eyes", url: "/api/traits/night-eyes" },
    ]);
    expect(first.book.entries.traits![0]).toEqual({ index: "flight", name: "Flight", description: "You fly.", species: [{ index: "owlfolk", name: "Owlfolk", url: "/api/species/owlfolk" }] });

    // Opened, edited (one removed, one renamed) and saved: indexes stay, the removed trait goes.
    const draft = openEntry(first.book, species, owl);
    expect(draft.traits).toEqual([{ index: "flight", name: "Flight", desc: "You fly." }, { index: "night-eyes", name: "Night Eyes", desc: "You see." }]);
    const again = saveEntry(first.book, species, { ...draft, traits: [{ index: "night-eyes", name: "Dark Sight", desc: "You see." }] });
    expect(again.book.entries.traits!.map((t) => [t.index, t.name])).toEqual([["night-eyes", "Dark Sight"]]);
    expect(deleteEntry(again.book, species, "owlfolk").entries.traits).toEqual([]);
  });

  it("saves a class's level table as levels and features, and opens it again", () => {
    const levels: Level[] = [
      { level: 2, prof_bonus: 2, features: [{ name: "Trick", desc: "Two." }] },
      { level: 1, prof_bonus: 2, features: [{ name: "Spark", desc: "One." }], spellcasting: { cantrips_known: 2 } },
    ];
    const first = saveEntry(blankBook("2024", "Homebrew"), classes, { index: "", name: "Tinker", hit_die: 8, levels });
    const book = first.book;
    // A level's table columns are kept.
    expect(book.entries.levels![0].spellcasting).toEqual({ cantrips_known: 2 });
    expect(book.entries.classes![0]).toEqual({ index: "tinker", name: "Tinker", hit_die: 8 });
    expect(book.entries.levels!.map((l) => [l.index, l.name, l.level, l.features])).toEqual([
      ["tinker-1", "Tinker 1", 1, [{ index: "tinker-spark", name: "Spark", url: "/api/features/tinker-spark" }]],
      ["tinker-2", "Tinker 2", 2, [{ index: "tinker-trick", name: "Trick", url: "/api/features/tinker-trick" }]],
    ]);
    expect(book.entries.features![0]).toMatchObject({ index: "tinker-spark", class: { index: "tinker" }, level: { index: "tinker-1" }, description: "One." });

    const draft = openEntry(book, classes, book.entries.classes![0]);
    expect(draft.levels).toEqual([
      { level: 1, prof_bonus: 2, features: [{ index: "tinker-spark", name: "Spark", desc: "One." }], spellcasting: { cantrips_known: 2 } },
      { level: 2, prof_bonus: 2, features: [{ index: "tinker-trick", name: "Trick", desc: "Two." }] },
    ]);
    // Level 2 removed: its level and feature go; level 1 keeps its indexes.
    const again = saveEntry(book, classes, { ...draft, levels: [(draft.levels as Level[])[0]] });
    expect(again.book.entries.levels!.map((l) => l.index)).toEqual(["tinker-1"]);
    expect(again.book.entries.features!.map((f) => f.index)).toEqual(["tinker-spark"]);
    const gone = deleteEntry(again.book, classes, "tinker");
    expect([gone.entries.classes, gone.entries.levels, gone.entries.features]).toEqual([[], [], []]);
  });
});
