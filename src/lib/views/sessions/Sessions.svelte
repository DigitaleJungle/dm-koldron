<script lang="ts">
  // The campaign's sessions as rows, the highest number first. Search by title (?q=) refines the list in place.
  import { afterNavigate } from "$app/navigation";
  import { page } from "$app/state";
  import { listSessions, type SessionList } from "$lib/api/sessions";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { gotoReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";
  import { filterSessions, parseDate, parseTime } from "./sessions";

  const t = strings.sessions;
  const campaign = $derived(campaigns.current!.id);
  const base = $derived(`/campaign/${campaign}/sessions`);
  const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });
  const timeFormat = new Intl.DateTimeFormat(undefined, { timeStyle: "short" });

  let list = $state<SessionList | null>(null);
  let loadError = $state<string | null>(null);

  $effect(() => {
    const id = campaign;
    let current = true; // ignore an answer for a campaign we've left
    [list, loadError] = [null, null];
    listSessions(id).then(
      (l) => current && (list = l),
      (e) => current && (loadError = t.loadError(errorMessage(e))),
    );
    return () => {
      current = false;
    };
  });

  let search = $state(page.url.searchParams.get("q") ?? "");
  afterNavigate(({ type }) => {
    if (type === "popstate") search = page.url.searchParams.get("q") ?? "";
  });

  function refine() {
    const params = new URLSearchParams(page.url.searchParams);
    if (search) params.set("q", search);
    else params.delete("q");
    const query = params.toString();
    return gotoReplacing(`${page.url.pathname}${query ? `?${query}` : ""}`, { keepFocus: true, noScroll: true });
  }

  const shown = $derived(filterSessions(list?.sessions ?? [], search));
  /** The date, then the start time when there is one; "No date" without a date. */
  const whenText = (date: string | null, time: string | null) => {
    const d = parseDate(date);
    const at = parseTime(time);
    return d ? [dateFormat.format(d), at && timeFormat.format(at)].filter(Boolean).join(" · ") : t.noDate;
  };
</script>

<svelte:head><title>{t.title}</title></svelte:head>

<div class="sessions" use:keepScroll={{ id: "sessions", url: page.url.href, ready: !!list }}>
  <div class="tools">
    <label for="session-search" class="sr-only">{t.search}</label>
    <input id="session-search" type="search" placeholder={t.search} autocomplete="off" bind:value={search} oninput={refine} />
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
    {#if list.sessions.length === 0}
      <p class="message">{t.empty}</p>
    {:else if shown.length === 0}
      <p class="message" role="status">{t.noMatches}</p>
    {:else}
      <ul class="rows">
        {#each shown as s (s.id)}
          <li>
            <a class="row" href="{base}/{s.id}">
              <span class="name">{s.number} · {s.title}</span>
              <span class="muted">{whenText(s.date, s.time)}</span>
              <span class="muted count">{t.readyOf(s.ready, s.steps)}</span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>

<style>
  .sessions {
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

  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    min-height: var(--control-height);
    padding: var(--space-2) var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: var(--color-text);
    text-decoration: none;
  }

  .row:hover {
    border-color: var(--color-border-strong);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: var(--weight-bold);
  }

  .muted {
    color: var(--color-text-muted);
    font: var(--text-body-sm);
  }

  .count {
    text-align: right;
  }
</style>
