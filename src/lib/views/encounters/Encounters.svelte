<script lang="ts">
  // The campaign's encounters as cards (EncounterCard). Search (?q=), difficulty filter (?band=) and sort
  // (?sort=name|xp) refine the list in place; a card opens the encounter in Prepare.
  import { afterNavigate } from "$app/navigation";
  import { flip } from "svelte/animate";
  import { scale } from "svelte/transition";
  import { page } from "$app/state";
  import { catalogRows, type CatalogRow } from "$lib/api/catalog";
  import { listCharacters, type Character } from "$lib/api/characters";
  import { encounterBudgets, listEncounters, type Budget, type EncounterList } from "$lib/api/encounters";
  import { errorMessage } from "$lib/api/storage";
  import { enterMotion, moveMotion } from "$lib/motion";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { gotoReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";
  import { BANDS } from "./difficulty";
  import { encounterCard } from "./card";
  import EncounterCard from "./EncounterCard.svelte";
  import EncounterPlaceholder from "./EncounterPlaceholder.svelte";
  import { rowsByKey } from "./roster";

  const t = strings.encounters;
  const campaign = $derived(campaigns.current!.id);
  const base = $derived(`/campaign/${campaign}/encounters`);

  let list = $state<EncounterList | null>(null);
  let loadError = $state<string | null>(null);
  // What the cards need besides the encounters; without it the cards show no rating (and letters for pictures).
  let monsters = $state.raw<CatalogRow[]>([]);
  let characters = $state<Character[] | null>(null); // null until loaded (or when they can't be)
  let budgets = $state<Budget[]>([]);
  let lookupErrors = $state<string[]>([]);

  $effect(() => {
    const id = campaign;
    let current = true; // ignore an answer for a campaign we've left
    [list, loadError, lookupErrors, characters] = [null, null, [], null];
    const failed = (text: (reason: string) => string) => (e: unknown) => current && lookupErrors.push(text(errorMessage(e)));
    listEncounters(id).then(
      (l) => current && (list = l),
      (e) => current && (loadError = t.loadError(errorMessage(e))),
    );
    catalogRows("monsters", ["xp", "image"]).then((rows) => current && (monsters = rows), failed(t.list.lookupError));
    listCharacters(id).then((l) => current && (characters = l.characters), failed(t.party.loadError));
    encounterBudgets().then((b) => current && (budgets = b), failed(t.meter.budgetError));
    return () => {
      current = false;
    };
  });

  const byKey = $derived(rowsByKey(monsters));
  const cards = $derived((list?.encounters ?? []).map((e) => encounterCard(e, byKey, characters, budgets)));

  // Search, filter and sort refine the list in place (no Back step).
  let search = $state(page.url.searchParams.get("q") ?? "");
  afterNavigate(({ type }) => {
    if (type === "popstate") search = page.url.searchParams.get("q") ?? "";
  });
  const band = $derived(BANDS.find((b) => b === page.url.searchParams.get("band")) ?? null);
  const sort = $derived(page.url.searchParams.get("sort") === "name" ? "name" : page.url.searchParams.get("sort") === "xp" ? "xp" : "edited");

  function refine(change: Record<string, string | null>) {
    const params = new URLSearchParams(page.url.searchParams);
    for (const [key, value] of Object.entries(change)) value ? params.set(key, value) : params.delete(key);
    const query = params.toString();
    return gotoReplacing(`${page.url.pathname}${query ? `?${query}` : ""}`, { keepFocus: true, noScroll: true });
  }

  const shown = $derived.by(() => {
    const q = search.trim().toLowerCase();
    const found = cards.filter(
      (c) => (!band || c.band === band) && (!q || [c.name, ...c.monsters.map((m) => m.name)].some((n) => n.toLowerCase().includes(q))),
    );
    // The list comes most recently saved first.
    if (sort === "name") return found.toSorted((a, b) => a.name.localeCompare(b.name));
    if (sort === "xp") return found.toSorted((a, b) => b.xp - a.xp);
    return found;
  });
</script>

<svelte:head><title>{t.title}</title></svelte:head>

<div class="encounters" use:keepScroll={{ id: "encounters", url: page.url.href, ready: !!list }}>
  <div class="tools">
    <label for="encounter-search" class="sr-only">{t.list.search}</label>
    <input id="encounter-search" type="search" placeholder={t.list.search} autocomplete="off" bind:value={search} oninput={() => refine({ q: search || null })} />
    <div class="chips" role="group" aria-label={t.list.difficulty}>
      <!-- Pressing the pressed one again turns the filter off (all encounters), as on the Characters screen. -->
      {#each BANDS as b (b)}
        <button class="chip" aria-pressed={band === b} onclick={() => refine({ band: band === b ? null : b })}>{t.meter.bands[b]}</button>
      {/each}
    </div>
    <label class="sort">
      <span>{t.list.sort}</span>
      <select value={sort} onchange={(e) => refine({ sort: e.currentTarget.value === "edited" ? null : e.currentTarget.value })}>
        <option value="edited">{t.list.byEdited}</option>
        <option value="name">{t.list.byName}</option>
        <option value="xp">{t.list.byXp}</option>
      </select>
    </label>
    <a class="btn btn-primary add" href="{base}/new">
      <i class="ti ti-plus" aria-hidden="true"></i>
      {t.create}
    </a>
  </div>

  {#if loadError}
    <p class="message error" role="alert">{loadError}</p>
  {:else if list === null}
    <p class="message" role="status">{t.loading}</p>
  {:else}
    {#if list.unreadable.length > 0}
      <p class="message error" role="alert">{t.unreadable(list.unreadable.join(", "))}</p>
    {/if}
    {#each lookupErrors as error (error)}
      <p class="message error" role="alert">{error}</p>
    {/each}
    {#if list.encounters.length === 0}
      <ul class="cards"><li><EncounterPlaceholder {base} /></li></ul>
    {:else if shown.length === 0}
      <p class="message" role="status">{t.list.noMatches}</p>
    {:else}
      <ul class="cards">
        {#each shown as c (c.id)}
          <li animate:flip={moveMotion()} in:scale={enterMotion()}>
            <EncounterCard card={c} {base} />
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>


<style>
  .encounters {
    height: 100%;
    overflow-y: auto;
    padding: var(--space-5) var(--space-5) var(--corner-clearance);
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }

  .tools input {
    flex: 0 1 var(--search-width);
    min-height: var(--control-height);
    padding: 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .tools input::placeholder {
    color: var(--color-text-muted);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .chips .chip {
    min-height: var(--control-height);
  }

  .sort {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .add {
    margin-left: auto;
    text-decoration: none;
  }

  .message {
    margin: 0 0 var(--space-3);
    color: var(--color-text-muted);
  }

  .message.error {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  .cards {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--tile-min-width), 1fr));
    gap: var(--space-4);
  }

  .cards li {
    display: flex;
  }
</style>
