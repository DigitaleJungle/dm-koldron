<script lang="ts">
  import type { CatalogEntry } from "$lib/api/catalog";
  import { strings } from "$lib/strings";
  import { classNames } from "./format";

  let { loaded }: { loaded: CatalogEntry } = $props();
  const t = strings.entry;
  const species = $derived(loaded.entry);
  const traits = $derived(loaded.related.filter((r) => r.kind === "traits"));
</script>

<dl class="facts">
  {#if species.type}<dt>{t.creatureType}</dt><dd>{species.type}</dd>{/if}
  {#if species.size}<dt>{t.size}</dt><dd>{species.size}</dd>{/if}
  {#if typeof species.speed === "number"}<dt>{t.speed}</dt><dd>{t.feet(species.speed)}</dd>{/if}
  {#if species.subspecies?.length}<dt>{t.subspecies}</dt><dd>{classNames(species.subspecies).join(", ")}</dd>{/if}
</dl>

{#if traits.length}
  <h2>{t.traits}</h2>
  {#each traits as trait (trait.entry.index)}
    <div class="named">
      <strong>{trait.entry.name}.</strong>
      <!-- Rendered in Rust from the pack's text, with raw HTML, links and images removed. -->
      {@html trait.html}
    </div>
  {/each}
{/if}
