<script lang="ts">
  // Home's Random encounter: one of the campaign's encounters per column of the widget's width, picked at random, as
  // the Encounters screen's cards (a name opens it in the builder; Run on a Ready one), each with a die over its top
  // right corner that picks another one not shown (while there's one). None yet: the dashed placeholder card. The characters come
  // from Home, shared with the Party widget, so a change to the party changes the difficulty at once.
  import { untrack } from "svelte";
  import { catalogRows, type CatalogRow } from "$lib/api/catalog";
  import type { Character } from "$lib/api/characters";
  import { encounterBudgets, listEncounters, type Budget, type Encounter } from "$lib/api/encounters";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
  import { encounterCard } from "$lib/views/encounters/card";
  import EncounterCard from "$lib/views/encounters/EncounterCard.svelte";
  import EncounterPlaceholder from "$lib/views/encounters/EncounterPlaceholder.svelte";
  import { rowsByKey } from "$lib/views/encounters/roster";
  import { fillRandom, pickRandom, type Width } from "./widgets";

  let { characters, width }: { characters: Character[] | null; width: Width } = $props();

  const t = strings.home.encounter;
  const campaign = campaigns.current!.id;
  const base = `/campaign/${campaign}/encounters`;

  let all = $state<Encounter[] | null>(null);
  let error = $state<string | null>(null);
  let shown = $state<Encounter[]>([]);
  listEncounters(campaign).then(
    (l) => (all = l.encounters),
    (e) => (error = t.loadError(errorMessage(e))),
  );
  // Grows or shrinks with the width, keeping the ones already shown.
  $effect(() => {
    const [list, n] = [all, width]; // the dependencies; `shown` is read untracked, as it's written here
    if (list) shown = untrack(() => fillRandom(list, shown, n));
  });

  function reroll(i: number) {
    const next = pickRandom(all ?? [], shown);
    if (next) shown[i] = next;
  }

  // For its difficulty and pictures; without them the card shows no rating, and says why.
  let monsters = $state<CatalogRow[]>([]);
  let budgets = $state<Budget[]>([]);
  let lookupError = $state<string | null>(null);
  const lookupFailed = (e: unknown) => (lookupError = strings.encounters.list.lookupError(errorMessage(e)));
  catalogRows("monsters", ["xp", "image"]).then((rows) => (monsters = rows), lookupFailed);
  encounterBudgets().then((b) => (budgets = b), lookupFailed);
  const byKey = $derived(rowsByKey(monsters));
  const cards = $derived(shown.map((e) => encounterCard(e, byKey, characters, budgets)));
</script>

{#if error}
  <p class="error" role="alert">{error}</p>
{:else if all === null}
  <p class="muted" role="status">{strings.home.loading}</p>
{:else if all.length === 0}
  <EncounterPlaceholder {base} />
{:else}
  {#if lookupError}<p class="error" role="alert">{lookupError}</p>{/if}
  <div class="cards" style:--count={width}>
    {#each cards as card, i (card.id)}
      <div class="picked">
        <EncounterCard {card} {base} />
        {#if all.length > cards.length}
          <button class="icon-btn reroll" aria-label={t.reroll} onclick={() => reroll(i)}>
            <i class="ti ti-dice-5" aria-hidden="true"></i>
          </button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  p {
    margin: 0;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(var(--count), minmax(0, 1fr));
    gap: var(--space-4);
  }

  /* The die sits over the card's top right corner, above the card's own link. */
  .picked {
    position: relative;
  }

  .reroll {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    z-index: 1;
    background: var(--color-surface);
  }

</style>
