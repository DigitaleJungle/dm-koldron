// Builds the SRD pack's manifest from packs/srd/source/: pack.json (pointing at the source files, so the SRD is stored
// once) and encounter-budgets.json (parsed from the Combat Encounters section). Checks the entry counts against the SRD.
//   node scripts/build-srd-pack.mjs
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const PACK = "packs/srd";
const DATA = "source/2024/en";
// The 5e-database release the source data was copied from (docs/SOURCE.md); Update rules compares against it.
const DATA_VERSION = "7.9.1";
const ATTRIBUTION =
  'This work includes material from the System Reference Document 5.2.1 ("SRD 5.2.1") by Wizards of the Coast LLC, available at https://www.dndbeyond.com/srd. The SRD 5.2.1 is licensed under the Creative Commons Attribution 4.0 International License, available at https://creativecommons.org/licenses/by/4.0/legalcode.';

/** The entry counts of the SRD 5.2.1 data (docs/SPEC.md); a different count means the source changed. */
export const EXPECTED = {
  rules: 7,
  "rule-sections": 192,
  spells: 339,
  monsters: 341,
  classes: 12,
  species: 9,
  backgrounds: 4,
  feats: 17,
  conditions: 15,
  equipment: 182,
  "magic-items": 262,
};

/** "5e-SRD-Magic-Items.json" → "magic-items". */
export const typeOf = (file) => file.replace(/^5e-SRD-/, "").replace(/\.json$/, "").toLowerCase();

/** The rows of the "XP Budget per Character" Markdown table: `[{ level, low, moderate, high }]`. */
export function parseBudgets(markdown) {
  const at = markdown.indexOf("**XP Budget per Character**");
  if (at === -1) throw new Error('no "XP Budget per Character" table');
  const lines = markdown.slice(at).split("\n").slice(1).map((l) => l.trim()).filter(Boolean);
  const cells = (line) => line.replace(/^\||\|$/g, "").split("|").map((c) => c.trim());
  const head = cells(lines[0]).map((c) => c.toLowerCase());
  if (head.join() !== "party's level,low,moderate,high") throw new Error(`unexpected table header: ${lines[0]}`);
  const rows = [];
  for (const line of lines.slice(2)) {
    if (!line.startsWith("|")) break;
    const [level, low, moderate, high] = cells(line).map((c) => {
      const n = Number(c.replace(/,/g, ""));
      if (!Number.isInteger(n) || n < 0) throw new Error(`not a whole number in the budget table: "${c}"`);
      return n;
    });
    rows.push({ level, low, moderate, high });
  }
  if (rows.length === 0) throw new Error("the budget table has no rows");
  rows.forEach((r, i) => {
    if (r.level !== i + 1) throw new Error(`budget table levels are not 1, 2, 3…: row ${i + 1} is level ${r.level}`);
  });
  return rows;
}

const json = (value) => JSON.stringify(value, null, 2) + "\n";

function build() {
  const files = readdirSync(`${PACK}/${DATA}`).filter((f) => f.endsWith(".json")).sort();
  const counts = {};
  for (const f of files) counts[typeOf(f)] = JSON.parse(readFileSync(`${PACK}/${DATA}/${f}`, "utf8")).length;
  const wrong = Object.entries(EXPECTED).filter(([type, n]) => counts[type] !== n);
  if (wrong.length) throw new Error(wrong.map(([t, n]) => `${t}: ${counts[t] ?? "missing"}, expected ${n}`).join("; "));

  const sections = JSON.parse(readFileSync(`${PACK}/${DATA}/5e-SRD-Rule-Sections.json`, "utf8"));
  const encounters = sections.find((s) => s.index === "combat-encounters");
  if (!encounters) throw new Error("no combat-encounters rule section");
  const budgets = parseBudgets(encounters.desc);

  const manifest = {
    schema: 1,
    id: "srd",
    name: "SRD 5.2.1",
    version: DATA_VERSION,
    license: "CC-BY-4.0 AND MIT",
    attribution: ATTRIBUTION,
    ruleset: "2024",
    files: [
      ...files.map((f) => ({ type: typeOf(f), path: `${DATA}/${f}` })),
      { type: "encounter-budgets", path: "encounter-budgets.json" },
    ],
  };
  writeFileSync(`${PACK}/encounter-budgets.json`, json(budgets));
  writeFileSync(`${PACK}/pack.json`, json(manifest));
  for (const [type, n] of Object.entries(counts)) console.log(`${type}: ${n}`);
  console.log(`encounter-budgets: ${budgets.length}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    build();
  } catch (e) {
    console.error(`build-srd-pack: ${e.message}`);
    process.exit(1);
  }
}
