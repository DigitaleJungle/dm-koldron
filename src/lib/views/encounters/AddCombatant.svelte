<script lang="ts" module>
  import type { CatalogRow } from "$lib/api/catalog";
  import type { Character } from "$lib/api/characters";

  /** What the DM picked; Run builds the combatant (and rolls a monster's initiative when none is given). */
  export type Pick =
    | { kind: "monster"; row: CatalogRow; initiative: number | null }
    | { kind: "character"; character: Character; initiative: number | null }
    | { kind: "custom"; name: string; maxHp: number; ac: number | null; initiative: number | null };
</script>

<script lang="ts">
  // + Combatant: a monster from the packs, a campaign character not yet in the fight, or a custom one.
  import { strings } from "$lib/strings";
  import { crText } from "$lib/views/library/catalog";
  import { MAX_NUMBER } from "./run";

  let { monsters, characters, onpick }: { monsters: CatalogRow[]; characters: Character[]; onpick: (pick: Pick) => void } = $props();
  const t = strings.encounters.run.add;
  const KINDS = ["monster", "character", "custom"] as const;
  const MAX_SHOWN = 50;

  let dialog: HTMLDialogElement | undefined = $state();
  let kind = $state<(typeof KINDS)[number]>("monster");
  let search = $state("");
  let characterId = $state("");
  let name = $state("");
  let maxHp = $state<number | null>(null);
  let ac = $state<number | null>(null);
  let initiative = $state<number | null>(null);

  const found = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return monsters.filter((m) => !q || m.name.toLowerCase().includes(q)).slice(0, MAX_SHOWN);
  });

  const whole = (n: number | null) => (typeof n === "number" && Number.isInteger(n) && Math.abs(n) <= MAX_NUMBER ? n : null);

  export function open() {
    search = "";
    characterId = characters[0]?.id ?? "";
    name = "";
    maxHp = ac = initiative = null;
    dialog?.showModal();
  }

  function pick(p: Pick) {
    dialog?.close();
    onpick(p);
  }

  function addOther(event: SubmitEvent) {
    event.preventDefault();
    if (kind === "character") {
      const character = characters.find((c) => c.id === characterId);
      if (character) pick({ kind, character, initiative: whole(initiative) });
    } else if (name.trim()) {
      pick({ kind: "custom", name: name.trim(), maxHp: Math.max(0, whole(maxHp) ?? 0), ac: whole(ac), initiative: whole(initiative) });
    }
  }
</script>

<dialog bind:this={dialog} class="dialog" aria-labelledby="add-title">
  <h2 id="add-title">{t.title}</h2>
  <div class="kinds" role="group" aria-label={t.kind}>
    {#each KINDS as k (k)}
      <button class="btn" aria-pressed={kind === k} onclick={() => (kind = k)}>{t.kinds[k]}</button>
    {/each}
  </div>

  {#if kind === "monster"}
    <div class="monster-row">
      <label class="grow">
        <span class="label">{t.search}</span>
        <!-- svelte-ignore a11y_autofocus: the dialog's first job is the search -->
        <input class="text-field" type="search" autocomplete="off" bind:value={search} autofocus />
      </label>
      <!-- Empty: the monster rolls its initiative when it's added. -->
      <label class="init">
        <span class="label">{t.initiative}</span>
        <input class="text-field dice" type="number" min={-MAX_NUMBER} max={MAX_NUMBER} step="1" placeholder={t.roll} bind:value={initiative} />
      </label>
    </div>
    <ul class="found">
      {#each found as row (`${row.pack}/${row.index}`)}
        <li>
          <button class="pick" onclick={() => pick({ kind: "monster", row, initiative: whole(initiative) })}>
            <span>{row.name}</span>
            <span class="muted dice">CR {crText(row.fields.challenge_rating)}</span>
          </button>
        </li>
      {:else}
        <li class="muted">{t.noMatches}</li>
      {/each}
    </ul>
    <div class="actions">
      <button class="btn" onclick={() => dialog?.close()}>{t.cancel}</button>
    </div>
  {:else}
    <form onsubmit={addOther}>
      {#if kind === "character"}
        {#if characters.length === 0}
          <p class="muted">{t.noCharacters}</p>
        {:else}
          <label>
            <span class="label">{t.kinds.character}</span>
            <select bind:value={characterId}>
              {#each characters as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
            </select>
          </label>
        {/if}
      {:else}
        <label>
          <span class="label">{t.name}</span>
          <input class="text-field" maxlength="100" autocomplete="off" required bind:value={name} />
        </label>
        <div class="numbers">
          <label><span class="label">{t.maxHp}</span><input class="text-field dice" type="number" min="0" max={MAX_NUMBER} step="1" bind:value={maxHp} /></label>
          <label><span class="label">{t.ac}</span><input class="text-field dice" type="number" min="0" max={MAX_NUMBER} step="1" bind:value={ac} /></label>
        </div>
      {/if}
      <label class="init">
        <span class="label">{t.initiative}</span>
        <input class="text-field dice" type="number" min={-MAX_NUMBER} max={MAX_NUMBER} step="1" bind:value={initiative} />
      </label>
      <div class="actions">
        <button type="button" class="btn" onclick={() => dialog?.close()}>{t.cancel}</button>
        <button class="btn btn-primary" disabled={kind === "character" && characters.length === 0}>{t.add}</button>
      </div>
    </form>
  {/if}
</dialog>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .kinds,
  .numbers {
    display: flex;
    gap: var(--space-2);
  }

  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .numbers label,
  .grow {
    flex: 1;
  }

  .monster-row {
    display: flex;
    align-items: flex-end;
    gap: var(--space-2);
  }

  .monster-row .init input {
    width: var(--level-input-width);
  }

  .found {
    display: flex;
    flex-direction: column;
    max-height: var(--picklist-height);
    margin: 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
  }

  .pick {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .pick:hover {
    background: var(--color-surface-sunken);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>
