<script lang="ts">
  import type { CatalogEntry } from "$lib/api/catalog";
  import { strings } from "$lib/strings";
  import { ABILITIES, ACTION_LISTS, challenge, damages, hitPoints, immunities, modifier, monsterHeader, proficiencies, senses, signed, speed } from "./format";

  let { loaded }: { loaded: CatalogEntry } = $props();
  const t = strings.entry;
  const m = $derived(loaded.entry);
  // Lines with nothing to say are left out.
  const lines = $derived(
    [
      [t.savingThrows, proficiencies(m, "Saving Throw")],
      [t.skills, proficiencies(m, "Skill")],
      [t.vulnerabilities, damages(m.damage_vulnerabilities)],
      [t.resistances, damages(m.damage_resistances)],
      [t.immunities, immunities(m)],
      [t.senses, senses(m.senses)],
      [t.languages, m.languages ?? ""],
      [t.cr, challenge(m)],
    ].filter(([, value]) => value),
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
  {#each lines as [label, value] (label)}
    <dt>{label}</dt><dd>{value}</dd>
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
