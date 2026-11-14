<script lang="ts">
  // Home's Previous session: the latest Played session's summary, with arrows to step to earlier (and back to later)
  // ones.
  import { listSessions, type SessionRow } from "$lib/api/sessions";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
  import { overview } from "$lib/views/sessions/sessions";

  const t = strings.home.summary;
  const s = strings.sessions;
  const campaign = campaigns.current!.id;

  /** Highest number first. */
  let played = $state<SessionRow[] | null>(null);
  let error = $state<string | null>(null);
  let at = $state(0);
  listSessions(campaign).then(
    (l) => (played = overview(l.sessions).played),
    (e) => (error = s.loadError(errorMessage(e))),
  );
  const row = $derived(played?.[at] ?? null);
</script>

{#if error}
  <p class="error" role="alert">{error}</p>
{:else if played === null}
  <p class="muted" role="status">{strings.home.loading}</p>
{:else if !row}
  <p class="muted">{t.none}</p>
{:else}
  <!-- A refresher: the summary as a quote, the session it's from under it; the arrows just above it, at the left. -->
  <div class="row">
    <div class="steps">
      <button class="icon-btn" aria-label={t.older} disabled={at >= played.length - 1} onclick={() => at++}>
        <i class="ti ti-chevron-left" aria-hidden="true"></i>
      </button>
      <button class="icon-btn" aria-label={t.newer} disabled={at === 0} onclick={() => at--}>
        <i class="ti ti-chevron-right" aria-hidden="true"></i>
      </button>
    </div>
    <div class="text">
      {#if row.summary}
        <figure>
          <blockquote>{row.summary}</blockquote>
          <figcaption><a href="/campaign/{campaign}/sessions/{row.id}">{row.title}</a></figcaption>
        </figure>
      {:else}
        <h3><a href="/campaign/{campaign}/sessions/{row.id}">{row.title}</a></h3>
        <p class="muted">{t.noSummary}</p>
      {/if}
    </div>
  </div>
{/if}

<style>
  p,
  h3 {
    margin: 0;
  }

  .row {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-width: 0;
  }

  .steps {
    display: flex;
    gap: var(--space-1);
  }

  h3 {
    font: var(--text-h2);
  }

  h3 a {
    color: inherit;
    text-decoration: none;
  }

  h3 a:hover {
    text-decoration: underline;
  }

  figure {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
  }

  /* A quote: a green bar on the left, the text larger and in italics. */
  blockquote {
    margin: 0;
    padding: var(--space-1) 0 var(--space-1) var(--space-4);
    border-left: calc(3 * var(--border-width)) solid var(--color-accent);
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-style: italic;
    white-space: pre-wrap;
  }

  figcaption {
    padding-left: var(--space-4);
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  figcaption::before {
    content: "— " / "";
  }

  figcaption a {
    color: inherit;
    text-decoration: none;
  }

  figcaption a:hover {
    text-decoration: underline;
  }
</style>
