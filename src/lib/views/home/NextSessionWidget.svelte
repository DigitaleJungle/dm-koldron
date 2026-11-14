<script lang="ts">
  // Home's Next session: the lowest-numbered session not played yet (as the Sessions overview's Up next): its
  // date (nothing without one), title and premise, with Continue prep and Run session. None left: New session.
  import { listSessions, type SessionRow } from "$lib/api/sessions";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
  import { overview, parseDate, parseTime } from "$lib/views/sessions/sessions";

  const t = strings.home.nextSession;
  const s = strings.sessions;
  const base = `/campaign/${campaigns.current!.id}/sessions`;
  const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "full" });
  const timeFormat = new Intl.DateTimeFormat(undefined, { timeStyle: "short" });

  let next = $state<SessionRow | null | undefined>(undefined); // undefined while loading
  let error = $state<string | null>(null);
  listSessions(campaigns.current!.id).then(
    (l) => (next = overview(l.sessions).next),
    (e) => (error = s.loadError(errorMessage(e))),
  );

  function when(row: SessionRow): string {
    const d = parseDate(row.date);
    const at = parseTime(row.time);
    return d ? [dateFormat.format(d), at && timeFormat.format(at)].filter(Boolean).join(" · ") : "";
  }
</script>

{#if error}
  <p class="error" role="alert">{error}</p>
{:else if next === undefined}
  <p class="muted" role="status">{strings.home.loading}</p>
{:else if next === null}
  <h3>{t.none}</h3>
  <div class="actions"><a class="btn btn-primary" href="{base}/new"><i class="ti ti-plus" aria-hidden="true"></i>{t.create}</a></div>
{:else}
  {#if parseDate(next.date)}<span class="muted small">{when(next)}</span>{/if}
  <h3>{next.title}</h3>
  {#if next.premise}<p class="premise">{next.premise}</p>{/if}
  <div class="actions">
    <a class="btn" href="{base}/{next.id}">{s.overview.continuePrep}</a>
    <a class="btn btn-primary" href="{base}/{next.id}/run"><i class="ti ti-player-play" aria-hidden="true"></i>{s.run}</a>
  </div>
{/if}

<style>
  p,
  h3 {
    margin: 0;
  }

  h3 {
    font: var(--text-h2);
  }

  .premise {
    white-space: pre-wrap;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  a.btn {
    text-decoration: none;
  }
</style>
