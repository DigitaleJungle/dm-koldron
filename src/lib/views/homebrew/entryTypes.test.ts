import { describe, expect, it } from "vitest";
import { ENTRY_TYPES, sectionsOf } from "./entryTypes";

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
