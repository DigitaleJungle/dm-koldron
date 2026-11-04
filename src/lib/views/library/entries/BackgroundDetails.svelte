<script lang="ts">
  import type { CatalogEntry } from "$lib/api/catalog";
  import { strings } from "$lib/strings";
  import RefLink from "../RefLink.svelte";
  import { classNames } from "./format";

  let { loaded }: { loaded: CatalogEntry } = $props();
  const t = strings.entry;
  const bg = $derived(loaded.entry);
</script>

<dl class="facts">
  {#if bg.ability_scores?.length}<dt>{strings.catalog.columns.abilityScores}</dt><dd>{classNames(bg.ability_scores).join(", ")}</dd>{/if}
  {#if bg.feat}
    <dt>{t.feat}</dt>
    <dd><RefLink ref={bg.feat} pack={loaded.pack} />{bg.feat.note ? ` (${bg.feat.note})` : ""}</dd>
  {/if}
  {#if bg.proficiencies?.length}<dt>{t.proficiencies}</dt><dd>{classNames(bg.proficiencies).join(", ")}</dd>{/if}
</dl>
<!-- Rendered in Rust from the pack's text, with raw HTML, links and images removed. -->
{@html loaded.html}
