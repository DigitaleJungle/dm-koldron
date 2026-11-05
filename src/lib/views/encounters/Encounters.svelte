<script lang="ts">
  // The campaign's encounters, most recently saved first; each opens in Prepare. Create encounter opens a new one.
  import { page } from "$app/state";
  import { listEncounters, type EncounterList } from "$lib/api/encounters";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { keepScroll } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";

  const t = strings.encounters;
  const date = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });
  const campaign = $derived(campaigns.current!.id);
  const base = $derived(`/campaign/${campaign}/encounters`);

  let list = $state<EncounterList | null>(null);
  let loadError = $state<string | null>(null);

  $effect(() => {
    const id = campaign;
    let current = true; // ignore an answer for a campaign we've left
    [list, loadError] = [null, null];
    listEncounters(id).then(
      (l) => current && (list = l),
      (e) => current && (loadError = t.loadError(errorMessage(e))),
    );
    return () => {
      current = false;
    };
  });

  const count = (monsters: { quantity: number }[]) => monsters.reduce((n, m) => n + m.quantity, 0);
</script>

<svelte:head><title>{t.title}</title></svelte:head>

<div class="encounters" use:keepScroll={{ id: "encounters", url: page.url.href, ready: !!list }}>
  <div class="tools">
    <a class="btn btn-primary" href="{base}/new">
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
    {#if list.encounters.length === 0}
      <p class="message">{t.empty}</p>
    {:else}
      <ul class="list">
        {#each list.encounters as e (e.id)}
          <li>
            <a class="entry" href="{base}/{e.id}">
              <span class="name">{e.name}</span>
              <span class="muted">{t.monsterCount(count(e.monsters))} · {date.format(e.savedAt)}</span>
            </a>
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
    justify-content: flex-end;
    margin-bottom: var(--space-4);
  }

  .tools .btn {
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

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    max-width: var(--content-max-width);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .list li + li {
    border-top: var(--border-width) solid var(--color-border);
  }

  .entry {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: var(--space-2) var(--space-4);
    padding: var(--space-3) var(--space-4);
    color: var(--color-text);
    text-decoration: none;
  }

  .entry:hover {
    background: var(--color-surface-sunken);
  }

  .name {
    font-weight: var(--weight-semibold);
  }

  .muted {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }
</style>
