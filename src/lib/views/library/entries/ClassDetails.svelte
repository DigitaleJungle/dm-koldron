<script lang="ts">
  import type { CatalogEntry } from "$lib/api/catalog";
  import { strings } from "$lib/strings";
  import { classNames, levelColumns, signed } from "./format";

  let { loaded }: { loaded: CatalogEntry } = $props();
  const t = strings.entry;
  const cls = $derived(loaded.entry);
  const levels = $derived(loaded.related.filter((r) => r.kind === "levels").map((r) => r.entry));
  const extra = $derived(levelColumns(levels));
  // Every level that lists a feature (Ability Score Improvement: 4, 8, 12, 16).
  const featureLevels = $derived.by(() => {
    const map = new Map<string, number[]>();
    for (const l of levels) for (const f of l.features ?? []) map.set(f.index, [...(map.get(f.index) ?? []), l.level]);
    return map;
  });
  const features = $derived(loaded.related.filter((r) => r.kind === "features"));
  // From every switched-on pack; another pack's shows its name.
  const subclasses = $derived(loaded.related.filter((r) => r.kind === "subclasses"));
  const prerequisites = $derived(
    (cls.multi_classing?.prerequisites ?? []).map((p: any) => `${p.ability_score?.name ?? ""} ${p.minimum_score ?? ""}`.trim()).join(", "),
  );
</script>

<dl class="facts">
  <dt>{t.hitDie}</dt><dd>d{cls.hit_die}</dd>
  {#if cls.primary_ability?.desc}<dt>{t.primaryAbility}</dt><dd>{cls.primary_ability.desc}</dd>{/if}
  {#if cls.saving_throws?.length}<dt>{t.savingThrows}</dt><dd>{classNames(cls.saving_throws).join(", ")}</dd>{/if}
  {#if cls.proficiencies?.length}<dt>{t.proficiencies}</dt><dd>{classNames(cls.proficiencies).join(", ")}</dd>{/if}
  {#each cls.proficiency_choices ?? [] as choice, i (i)}
    {#if choice.desc}<dt>{i === 0 && !cls.proficiencies?.length ? t.proficiencies : ""}</dt><dd>{choice.desc}</dd>{/if}
  {/each}
  {#if prerequisites}<dt>{t.multiclass}</dt><dd>{prerequisites}</dd>{/if}
</dl>

{#if levels.length}
  <h2>{t.levelTable(cls.name)}</h2>
  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th scope="col">{t.level}</th>
          <th scope="col">{t.pbColumn}</th>
          <th scope="col">{t.features}</th>
          {#each extra as col (col.id)}<th scope="col">{col.label}</th>{/each}
        </tr>
      </thead>
      <tbody>
        {#each levels as level (level.index)}
          <tr>
            <td>{level.level}</td>
            <td>{typeof level.prof_bonus === "number" ? signed(level.prof_bonus) : "—"}</td>
            <td>{classNames(level.features).join(", ") || "—"}</td>
            {#each extra as col (col.id)}<td>{col.cell(level)}</td>{/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

{#each features as feature (feature.entry.index)}
  {@const at = featureLevels.get(feature.entry.index)}
  <h3>{at ? t.featureLevel(at, feature.entry.name) : feature.entry.name}</h3>
  <!-- Rendered in Rust from the pack's text, with raw HTML, links and images removed. -->
  {@html feature.html}
{/each}
{#if subclasses.length}
  <h2>{t.subclasses}</h2>
  {#each subclasses as sub (`${sub.source}/${sub.entry.index}`)}
    <h3>{sub.entry.name}{#if sub.source !== loaded.source}{" "}<span class="muted">({sub.source})</span>{/if}</h3>
    {#if sub.entry.summary}<p><em>{sub.entry.summary}</em></p>{/if}
    <!-- Rendered in Rust from the pack's text, with raw HTML, links and images removed. -->
    {@html sub.html}
    {#each sub.entry.features ?? [] as feature, i (i)}
      <h4>{typeof feature.level === "number" ? t.featureLevel([feature.level], feature.name) : feature.name}</h4>
      {@html sub.parts[i] ?? ""}
    {/each}
  {/each}
{/if}

<style>
  .table-wrap {
    overflow-x: auto;
  }

  .muted {
    color: var(--color-text-muted);
    font-weight: var(--weight-regular);
  }
</style>
