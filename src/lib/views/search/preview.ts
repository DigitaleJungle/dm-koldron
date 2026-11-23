// What a preview of a result or a tag shows: a character's main numbers, a rule section's text, or a Catalog entry's
// facts and text. Used by the search screen's preview pane and the tag preview card.
import type { SearchHit } from "$lib/api/search";
import { catalogEntry } from "$lib/api/catalog";
import { listCharacters } from "$lib/api/characters";
import { rulesSection } from "$lib/api/rules";
import { campaigns } from "$lib/stores/campaigns.svelte";
import { strings } from "$lib/strings";
import { entryCategory, entryFacts } from "$lib/views/library/catalog";
import { signed } from "$lib/views/library/entries/format";
import { classLine, totalLevel } from "$lib/views/characters/characters";

export type Preview = { facts: { label: string; value: string }[]; html: string; source: string };

/** `scope`: the campaign whose packs it's read from (by default the one worked in); null for any switched-on pack. */
export async function loadPreview(hit: Pick<SearchHit, "kind" | "pack" | "index" | "campaign">, scope: string | null = campaigns.scope): Promise<Preview> {
  if (hit.kind === "characters") {
    const found = (await listCharacters(hit.campaign ?? "")).characters.find((c) => c.id === hit.index);
    if (!found) throw new Error(strings.characters.gone);
    const c = strings.characters.form;
    const stats: [string, number | null][] = [[c.maxHp, found.maxHp], [c.currentHp, found.currentHp], [c.tempHp, found.tempHp], [c.ac, found.ac], [c.initiative, found.initiative], [c.passivePerception, found.passivePerception], [c.passiveInsight, found.passiveInsight], [c.passiveInvestigation, found.passiveInvestigation]];
    const facts = [
      { label: c.player, value: found.player },
      { label: c.classes, value: classLine(found) },
      { label: strings.characters.byLevel, value: String(totalLevel(found)) },
      ...stats.filter(([, v]) => v !== null).map(([label, v]) => ({ label, value: label === c.initiative ? signed(v!) : String(v) })),
    ].filter((f) => f.value);
    return { facts, html: "", source: "" };
  }
  if (hit.kind === "rule-sections") {
    return { facts: [], html: (await rulesSection(hit.pack, hit.index, scope)).html, source: "" };
  }
  const entry = await catalogEntry(hit.kind, hit.pack, hit.index, scope);
  return { facts: entryFacts(entryCategory(hit.kind), entry).filter((f) => f.value !== "—"), html: entry.html, source: entry.source };
}
