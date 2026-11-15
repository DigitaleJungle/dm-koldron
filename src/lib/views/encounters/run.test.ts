import { describe, expect, it } from "vitest";
import type { Character } from "$lib/api/characters";
import type { Combatant, Run } from "$lib/api/encounters";
import { addCombatant, addLog, carryOver, damage, followLinked, giveTempHp, heal, health, initiativeNotation, inOrder, MAX_LOG, rosterMonsters, setDeathSave, stepTurn, toggleCondition } from "./run";

const fighter = (name: string, initiative: number | null, extra: Partial<Combatant> = {}): Combatant => ({
  id: name,
  kind: "character",
  pack: "",
  index: "",
  character: name,
  group: "",
  name,
  initiative,
  initiativeBonus: 0,
  hp: 10,
  maxHp: 10,
  tempHp: 0,
  ac: 12,
  concentrating: false,
  conditions: [],
  deathSuccesses: 0,
  deathFailures: 0,
  ...extra,
});
const goblin = (name: string, initiative: number, extra: Partial<Combatant> = {}) =>
  fighter(name, initiative, { kind: "monster", pack: "srd", index: "goblin", character: "", ...extra });
const run = (combatants: Combatant[], turn = 0, round = 1): Run => ({ round, turn, combatants, log: [], savedAt: 0 });

describe("order", () => {
  it("sorts by initiative, then bonus, then name, without initiative last", () => {
    const order = inOrder([fighter("C", 10), fighter("A", null), fighter("B", 10, { initiativeBonus: 2 }), fighter("Goblin 10", 5), fighter("Goblin 2", 5)]);
    expect(order.map((c) => c.name)).toEqual(["B", "C", "Goblin 2", "Goblin 10", "A"]);
  });

  it("keeps the turn on the same combatant when one joins", () => {
    const r = addCombatant(run([fighter("A", 15), fighter("B", 5)], 1), fighter("C", 10));
    expect(r.combatants.map((c) => c.name)).toEqual(["A", "C", "B"]);
    expect(r.combatants[r.turn].name).toBe("B");
  });
});

describe("turns", () => {
  const three = run([fighter("A", 15), goblin("G", 10), fighter("B", 5)]);

  it("goes round, adding a round when it wraps", () => {
    let r = stepTurn(stepTurn(stepTurn(three, 1), 1), 1);
    expect([r.turn, r.round]).toEqual([0, 2]);
    r = stepTurn(r, -1);
    expect([r.turn, r.round]).toEqual([2, 1]);
  });

  it("can't go before round 1's first turn", () => {
    expect(stepTurn(three, -1)).toBe(three);
  });

  it("skips defeated monsters but not downed characters", () => {
    const r = run([fighter("A", 15), goblin("G", 10, { hp: 0 }), fighter("B", 5, { hp: 0 })]);
    expect(stepTurn(r, 1).turn).toBe(2);
  });
});

describe("hit points", () => {
  it("takes damage from temp HP first, stops at 0 and ends concentration", () => {
    expect(damage(fighter("A", 1, { tempHp: 3 }), 5)).toMatchObject({ tempHp: 0, hp: 8 });
    expect(damage(fighter("A", 1, { concentrating: true }), 15)).toMatchObject({ hp: 0, concentrating: false });
  });

  it("heals up to the maximum, or without limit when it's unknown", () => {
    expect(heal(fighter("A", 1, { hp: 4 }), 20).hp).toBe(10);
    expect(heal(fighter("A", 1, { hp: 4, maxHp: 0 }), 20).hp).toBe(24);
  });

  it("keeps the higher temp HP", () => {
    expect(giveTempHp(fighter("A", 1, { tempHp: 5 }), 3).tempHp).toBe(5);
    expect(giveTempHp(fighter("A", 1, { tempHp: 5 }), 8).tempHp).toBe(8);
  });

  it("says bloodied, down, defeated, stable and dead", () => {
    expect(health(fighter("A", 1))).toBe(null);
    expect(health(fighter("A", 1, { hp: 5 }))).toBe("bloodied");
    expect(health(fighter("A", 1, { hp: 0 }))).toBe("down");
    expect(health(goblin("G", 1, { hp: 0 }))).toBe("defeated");
    expect(health(fighter("A", 1, { hp: 0, deathSuccesses: 3 }))).toBe("stable");
    expect(health(fighter("A", 1, { hp: 0, deathFailures: 3 }))).toBe("dead");
  });
});

describe("death saves", () => {
  it("start afresh when a character drops, fail on damage at 0, and clear on healing", () => {
    let c = damage(fighter("A", 1, { hp: 3, deathFailures: 2 }), 5);
    expect([c.deathSuccesses, c.deathFailures]).toEqual([0, 0]);
    c = damage(damage(damage(damage(c, 1), 1), 1), 1);
    expect(c.deathFailures).toBe(3);
    expect(heal(c, 2)).toMatchObject({ hp: 2, deathFailures: 0 });
  });

  it("set by pips, and the same pip again takes one back", () => {
    const c = setDeathSave(fighter("A", 1, { hp: 0 }), "deathSuccesses", 2);
    expect(c.deathSuccesses).toBe(2);
    expect(setDeathSave(c, "deathSuccesses", 2).deathSuccesses).toBe(1);
  });
});

it("toggles conditions", () => {
  const prone = { index: "prone", name: "Prone" };
  const c = toggleCondition(fighter("A", 1), prone);
  expect(c.conditions).toEqual([prone]);
  expect(toggleCondition(c, prone).conditions).toEqual([]);
});

describe("log", () => {
  it("stamps the round and whose turn it is, newest first, and keeps at most MAX_LOG", () => {
    let r = addLog(run([fighter("A", 15), fighter("B", 5)], 1, 2), "first");
    r = addLog(r, "second");
    expect(r.log[0]).toEqual({ round: 2, actor: "B", text: "second" });
    r = { ...r, log: Array.from({ length: MAX_LOG }, () => r.log[1]) };
    expect(addLog(r, "new").log).toHaveLength(MAX_LOG);
  });

  it("has no actor before Start", () => {
    expect(addLog(run([fighter("A", 15)], 0, 0), "note").log[0]).toEqual({ round: 1, actor: "", text: "note" });
  });
});

it("writes initiative notation for any bonus", () => {
  expect([initiativeNotation(2), initiativeNotation(0), initiativeNotation(-1)]).toEqual(["1d20+2", "1d20+0", "1d20-1"]);
});

describe("building from the encounter", () => {
  const row = (quantity: number, index = "goblin") => ({ pack: "srd", index, name: index === "goblin" ? "Goblin" : "Orc", source: "SRD", quantity });

  it("numbers a monster on across its split groups, each group with its own key", () => {
    const list = rosterMonsters([row(2), row(1, "orc"), row(1)]);
    expect(list.map((m) => [m.name, m.group])).toEqual([
      ["Goblin 1", "srd/goblin"],
      ["Goblin 2", "srd/goblin"],
      ["Orc", "srd/orc"],
      ["Goblin 3", "srd/goblin#2"],
    ]);
  });

  it("keeps initiatives and whoever was added when rebuilt after the encounter changed", () => {
    const roster = [row(1)];
    const saved = run(
      [
        goblin("Goblin", 7, { group: "srd/goblin" }),
        fighter("Mira", 15),
        fighter("Gone", 13), // taken out of the party since: not brought back
        fighter("Guest", 11, { group: "added-2" }),
        fighter("Ally", 9, { kind: "custom", character: "", group: "added-3" }),
        goblin("Wolf", 12, { group: "added-1" }),
      ],
      0,
      0,
    );
    const fresh = run([goblin("Goblin", 3, { group: "srd/goblin" }), goblin("Orc", 5, { group: "srd/orc" }), fighter("Mira", null)], 0, 0);
    const kept = carryOver(fresh, saved, [...roster, row(1, "orc")]);
    expect(kept.combatants.map((c) => [c.name, c.initiative])).toEqual([
      ["Goblin", 7],
      ["Orc", 5],
      ["Mira", 15],
      ["Guest", 11],
      ["Ally", 9],
      ["Wolf", 12],
    ]);
  });
});

describe("linked characters", () => {
  const sheet = (id: string, linked: string | null, extra: Partial<Character> = {}): Character => ({
    id, name: id, player: "", classes: [], maxHp: 30, currentHp: 0, tempHp: null, ac: 15, initiative: 2,
    passivePerception: null, passiveInsight: null, passiveInvestigation: null, onHome: true, createdAt: 0, updatedAt: 0,
    linked, deathSaves: { successes: 1, failures: 2 }, ...extra,
  });

  it("takes HP, max and temp HP and death saves from a linked character only", () => {
    const r = run([fighter("Gylbard", 12), fighter("Mira", 8), goblin("Goblin", 5)]);
    const next = followLinked(r, [sheet("Gylbard", "ddb"), sheet("Mira", null)])!;
    expect(next.combatants[0]).toMatchObject({ hp: 0, maxHp: 30, tempHp: 0, deathSuccesses: 1, deathFailures: 2 });
    expect(next.combatants[1]).toBe(r.combatants[1]);
    expect(next.combatants[2]).toBe(r.combatants[2]);
    expect(followLinked(next, [sheet("Gylbard", "ddb")])).toBeNull(); // nothing changed: nothing to save
  });
});
