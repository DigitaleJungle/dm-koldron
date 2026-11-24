import { describe, expect, it } from "vitest";
import { ENTRY_TYPES, entryType, layoutOf, sectionsOf } from "./entryTypes";

const shape = (blocks: ReturnType<typeof layoutOf>) =>
  blocks.map((b) => ("row" in b ? b.row.map((c) => (c.flag ? `${c.field.key}+${c.flag.key}` : c.field.key)) : b.field.key));

describe("card layout", () => {
  it("puts short fields in a row, a checkbox by the field before it, and the rest alone", () => {
    const casting = sectionsOf(entryType("spells")!).find((s) => s.id === "casting")!;
    expect(shape(layoutOf(casting.fields))).toEqual([["casting_time+ritual", "range"], "components", ["material", "duration+concentration"]]);
    const item = sectionsOf(entryType("magic-items")!)[0];
    expect(shape(layoutOf(item.fields))).toEqual([["equipment_category", "rarity.name+attunement"]]);
    // A field the form draws itself stays alone, and a checkbox with no short field before it too.
    const [identity, challenge] = sectionsOf(entryType("monsters")!)[0].fields;
    expect(shape(layoutOf([identity, challenge], () => true))).toEqual(["identity", "challenge"]);
    expect(shape(layoutOf([{ key: "b", label: "B", input: "bool" }]))).toEqual(["b"]);
  });
});

describe("entry type cards", () => {
  it("puts every field in one card, the first being Basics", () => {
    for (const type of ENTRY_TYPES) {
      const keys = type.sections.flatMap((s) => s.keys);
      expect(keys.toSorted(), type.id).toEqual(type.fields.map((f) => f.key).toSorted());
      expect(type.sections[0].id).toBe("basics");
      expect(sectionsOf(type).every((s) => s.fields.every(Boolean))).toBe(true);
    }
  });
});
