<script lang="ts">
  import type { CatalogEntry } from "$lib/api/catalog";
  import { strings } from "$lib/strings";
  import RefLink from "../RefLink.svelte";
  import { castingTime, components, duration, spellHeader } from "./format";

  let { loaded }: { loaded: CatalogEntry } = $props();
  const t = strings.entry;
  const spell = $derived(loaded.entry);
</script>

<div class="subtitle">
  {spellHeader(spell)}
  {#if spell.classes?.length}
    ({#each spell.classes as cls, i (cls.index)}{#if i > 0},{" "}{/if}<RefLink ref={cls} pack={loaded.pack} />{/each})
  {/if}
</div>
<dl class="facts">
  <dt>{t.castingTime}</dt><dd>{castingTime(spell)}</dd>
  <dt>{t.range}</dt><dd>{spell.range ?? ""}</dd>
  <dt>{t.components}</dt><dd>{components(spell)}</dd>
  <dt>{t.duration}</dt><dd>{duration(spell)}</dd>
</dl>
<!-- Rendered in Rust from the pack's text, with raw HTML, links and images removed. -->
{@html loaded.html}
{#if loaded.parts.higher_level}
  <div class="named">
    <strong>{spell.level === 0 ? t.cantripUpgrade : t.higherLevel}</strong>
    {@html loaded.parts.higher_level}
  </div>
{/if}
