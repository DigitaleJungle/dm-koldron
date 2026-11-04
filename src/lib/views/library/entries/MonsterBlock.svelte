<script lang="ts">
  import type { CatalogEntry } from "$lib/api/catalog";
  import { strings } from "$lib/strings";
  import RefLink from "../RefLink.svelte";
  import { ABILITIES, ACTION_LISTS, challenge, damages, hitPoints, modifier, monsterHeader, proficiencies, senses, signed, speed } from "./format";

  let { loaded }: { loaded: CatalogEntry } = $props();
  const t = strings.entry;
  const m = $derived(loaded.entry);
  // Lines with nothing to say are left out. Condition immunities are references after the damage immunities.
  type Line = { label: string; value: string; refs?: { index: string; name: string; url?: string }[] };
  const lines = $derived(
    ([
      { label: t.savingThrows, value: proficiencies(m, "Saving Throw") },
      { label: t.skills, value: proficiencies(m, "Skill") },
      { label: t.vulnerabilities, value: damages(m.damage_vulnerabilities) },
      { label: t.resistances, value: damages(m.damage_resistances) },
      { label: t.immunities, value: damages(m.damage_immunities), refs: m.condition_immunities ?? [] },
      { label: t.senses, value: senses(m.senses) },
      { label: t.languages, value: m.languages ?? "" },
      { label: t.cr, value: challenge(m) },
    ] as Line[]).filter((line) => line.value || line.refs?.length),
  );
</script>

<p class="subtitle">{monsterHeader(m)}</p>
<dl class="facts">
  <dt>{t.ac}</dt><dd>{m.armor_class?.[0]?.value ?? ""}</dd>
  <dt>{t.hp}</dt><dd>{hitPoints(m)}</dd>
  <dt>{t.speed}</dt><dd>{speed(m.speed)}</dd>
</dl>

<table class="abilities" aria-label={t.abilityTable}>
  <thead>
    <tr>
      {#each ABILITIES as ability (ability)}<th scope="col">{t.abilities[ability]}</th>{/each}
    </tr>
  </thead>
  <tbody>
    <tr>
      {#each ABILITIES as ability (ability)}
        <td>{m[ability] ?? "—"} {#if typeof m[ability] === "number"}<span class="mod">({signed(modifier(m[ability]))})</span>{/if}</td>
      {/each}
    </tr>
  </tbody>
</table>

<dl class="facts">
  {#each lines as line (line.label)}
    <dt>{line.label}</dt>
    <dd>
      {line.value}{#if line.value && line.refs?.length};{" "}{/if}
      {#each line.refs ?? [] as ref, i (ref.index)}{#if i > 0},{" "}{/if}<RefLink {ref} pack={loaded.pack} />{/each}
    </dd>
  {/each}
</dl>

{#each ACTION_LISTS as list (list.id)}
  {#if m[list.id]?.length}
    <h2>{list.label}</h2>
    {#each m[list.id] as action, i (i)}
      <div class="named">
        <strong>{action.name}.</strong>
        <!-- Rendered in Rust from the pack's text, with raw HTML, links and images removed. -->
        {@html loaded.parts[`${list.id}.${i}`] ?? ""}
      </div>
    {/each}
  {/if}
{/each}

<style>
  .abilities {
    width: auto;
    text-align: center;
  }

  .abilities th,
  .abilities td {
    min-width: calc(var(--control-height) * 2);
    text-align: center;
  }

  .mod {
    color: var(--color-text-muted);
  }
</style>
