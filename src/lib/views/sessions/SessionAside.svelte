<script lang="ts">
  // The right column: the party (the campaign's party), a note on it for this session, the campaign-wide Open
  // threads (saved at once) and this session's To bring.
  import { pictureSrc } from "$lib/api/campaigns";
  import type { Character } from "$lib/api/characters";
  import { getThreads, saveThreads } from "$lib/api/sessions";
  import { errorMessage } from "$lib/api/storage";
  import { strings } from "$lib/strings";
  import { classLine } from "$lib/views/characters/characters";
  import LineList from "./LineList.svelte";

  let {
    campaign,
    party,
    partyNote = $bindable(),
    toBring = $bindable(),
  }: { campaign: string; party: Character[]; partyNote: string; toBring: string[] } = $props();

  const t = strings.sessions.aside;
  /** Null until they have loaded: editing before (or after a failed load) would save over the saved ones. */
  let threads = $state<string[] | null>(null);
  let threadsError = $state<string | null>(null);

  // svelte-ignore state_referenced_locally
  getThreads(campaign).then(
    (list) => (threads = list),
    (e) => (threadsError = t.threadsError(errorMessage(e))),
  );

  async function changeThreads(next: string[]) {
    const before = threads;
    if (before === null) return;
    threads = next;
    threadsError = null;
    try {
      threads = await saveThreads(campaign, next);
    } catch (e) {
      threads = before;
      threadsError = t.threadsError(errorMessage(e));
    }
  }
</script>

<aside>
  <section>
    <h2 class="sr-only">{t.party}</h2>
    {#each party as c (c.id)}
      <div class="member">
        {#if c.picture}
          <img class="hex" src={pictureSrc(c.picture)} alt="" />
        {:else}
          <span class="hex letter" aria-hidden="true">{c.name.charAt(0)}</span>
        {/if}
        <span class="who">
          <strong>{c.name}</strong>
          <span class="muted">{[classLine(c), c.player].filter(Boolean).join(" · ")}</span>
        </span>
        <span class="stats">
          <span>{t.ac(c.ac)}</span>
          <span class="muted">{t.pp(c.passivePerception)}</span>
        </span>
      </div>
    {:else}
      <p class="muted">{t.noParty}</p>
    {/each}
    <textarea class="text-field note" bind:value={partyNote} rows="3" maxlength="20000" aria-label={t.partyNote} placeholder={t.partyNote}></textarea>
  </section>

  <section>
    <h2 class="sr-only">{t.threads}</h2>
    {#if threadsError}<p class="error" role="alert">{threadsError}</p>{/if}
    {#if threads !== null}<LineList label={t.threads} lines={threads} onchange={changeThreads} />{/if}
  </section>

  <section>
    <h2 class="sr-only">{t.toBring}</h2>
    <LineList label={t.toBring} lines={toBring} onchange={(next) => (toBring = next)} />
  </section>
</aside>

<style>
  aside {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  h2 {
    margin: 0;
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .member {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-1) 0;
    border-bottom: var(--border-width) solid var(--color-border);
  }

  .hex {
    flex: none;
    width: var(--roster-badge-width);
    height: var(--roster-badge-height);
    object-fit: cover;
    clip-path: polygon(25% 0, 75% 0, 100% 50%, 75% 100%, 25% 100%, 0 50%);
  }

  .letter {
    display: grid;
    place-items: center;
    background: var(--charcoal);
    color: var(--on-fill);
    font: var(--text-dice);
  }

  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .stats {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    font: var(--text-dice);
  }

  .muted {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .error {
    margin: 0;
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  .note {
    padding: var(--space-2) var(--space-3);
    font: inherit;
    resize: vertical;
  }
</style>
