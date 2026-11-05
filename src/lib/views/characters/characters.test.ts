import { describe, expect, it } from "vitest";
import type { Character } from "$lib/api/characters";
import { blankCharacter, clamp01, classLine, selectCharacters, totalLevel } from "./characters";

const make = (name: string, player: string, classes: [string, number][]): Character => ({
  ...blankCharacter(),
  id: name.toLowerCase(),
  name,
  player,
  classes: classes.map(([n, level]) => ({ name: n, level })),
});

describe("characters", () => {
  const mira = make("Mira Ashvale", "Lotte", [["Rogue", 3], ["Wizard", 2]]);
  const brannoc = make("Brannoc", "Jens", [["Fighter", 5]]);
  const tavi = make("Tavi", "Wout", [["Wizard", 4]]);

  it("show the class line and total level, and keep a crop inside the picture", () => {
    expect(classLine(mira)).toBe("Rogue 3 / Wizard 2");
    expect(totalLevel(mira)).toBe(5);
    expect([clamp01(-0.2), clamp01(0.4), clamp01(1.3)]).toEqual([0, 0.4, 1]);
  });

  it("search by name or player and sort by name or level", () => {
    const all = [tavi, mira, brannoc];
    expect(selectCharacters(all, "", "name").map((c) => c.name)).toEqual(["Brannoc", "Mira Ashvale", "Tavi"]);
    expect(selectCharacters(all, "", "level").map((c) => c.name)).toEqual(["Brannoc", "Mira Ashvale", "Tavi"]);
    expect(selectCharacters(all, "lot", "name").map((c) => c.name)).toEqual(["Mira Ashvale"]);
    expect(selectCharacters(all, "mira lotte", "name").map((c) => c.name)).toEqual(["Mira Ashvale"]);
    expect(selectCharacters(all, "nobody", "name")).toEqual([]);
  });
});
