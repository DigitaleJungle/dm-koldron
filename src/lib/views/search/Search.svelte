<script lang="ts">
  // The search screen: every result for ?q=, grouped by type, with type chips (?type=) and a preview pane.
  import { untrack } from "svelte";
  import { afterNavigate } from "$app/navigation";
  import { page } from "$app/state";
  import BackButton from "$lib/components/BackButton.svelte";
  import SettingsButton from "$lib/components/SettingsButton.svelte";
  import { catalogEntry } from "$lib/api/catalog";
  import { rulesSection } from "$lib/api/rules";
  import { searchAll, type SearchHit } from "$lib/api/search";
  import { errorMessage } from "$lib/api/storage";
  import { gotoReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";
  import { entryCategory, entryFacts } from "$lib/views/library/catalog";
  import "$lib/styles/prose.css";
  import { groupByKind, hitAddress, kindLabel, placeLabel } from "./search";

  const t = strings.search;
  const key = (hit: SearchHit) => `${hit.kind}/${hit.pack}/${hit.index}`;

  let query = $state(page.url.searchParams.get("q") ?? "");
  // Back (or Forward) can land on another search while this screen stays: take its text again.
  afterNavigate(({ type }) => {
    if (type === "popstate") query = page.url.searchParams.get("q") ?? "";
  });

  let hits = $state<SearchHit[] | null>(null); // null while searching
  let error = $state<string | null>(null);
  let asked = 0;
  // Packs may have been switched on or off in Settings, so the search runs again when it closes.
  let settingsOpen = $state(false);
  let reloadKey = $state(0);
  let settingsWasOpen = false;
  $effect(() => {
    if (settingsWasOpen && !settingsOpen) reloadKey++;
    settingsWasOpen = settingsOpen;
  });
  $effect(() => {
    const mine = ++asked;
    const text = query;
    void reloadKey;
    if (!text.trim()) return void ([hits, error] = [[], null]);
    searchAll(text).then(
      (found) => mine === asked && ([hits, error] = [found, null]),
      (e) => mine === asked && ([hits, error] = [[], t.error(errorMessage(e))]),
    );
  });

  const groups = $derived(groupByKind(hits ?? []));
  // A type the current results don't have (left in the address while typing) counts as All.
  const type = $derived(groups.find((g) => g.kind === page.url.searchParams.get("type"))?.kind ?? null);
  const shown = $derived(type ? groups.filter((g) => g.kind === type) : groups);

  // The preview shows the chosen result, else the first one shown.
  let chosen = $state<string | null>(null);
  const selected = $derived(shown.flatMap((g) => g.hits).find((h) => key(h) === chosen) ?? shown[0]?.hits[0]);

  type Preview = { label: string; facts: { label: string; value: string }[]; html: string };
  let preview = $state<Preview | null>(null);
  let previewError = $state<string | null>(null);
  // Keyed, not by object: each keystroke's search returns new objects for the same results.
  const selectedKey = $derived(selected ? key(selected) : null);
  $effect(() => {
    const wanted = selectedKey;
    const hit = untrack(() => selected);
    [preview, previewError] = [null, null];
    const current = () => wanted === untrack(() => selectedKey);
    if (hit) load(hit).then((p) => current() && (preview = p), (e) => current() && (previewError = errorMessage(e)));
  });

  async function load(hit: SearchHit): Promise<Preview> {
    const label = `${kindLabel(hit.kind)} · ${hit.source} · ${placeLabel(hit)}`;
    if (hit.kind === "rule-sections") {
      return { label, facts: [], html: (await rulesSection(hit.pack, hit.index)).html };
    }
    const entry = await catalogEntry(hit.kind, hit.pack, hit.index);
    return { label, facts: entryFacts(entryCategory(hit.kind), entry).filter((f) => f.value !== "—"), html: entry.html };
  }

  // Typing and the chips refine the results in place (no Back step).
  function address(changes: Record<string, string | null>) {
    const params = new URLSearchParams(page.url.searchParams);
    for (const [name, value] of Object.entries(changes)) value ? params.set(name, value) : params.delete(name);
    return `/search?${params}`;
  }
  const refine = (changes: Record<string, string | null>) => gotoReplacing(address(changes), { keepFocus: true, noScroll: true });
</script>

<svelte:head><title>{t.title}</title></svelte:head>

<div class="search">
  <header class="top">
    <BackButton />
    <label for="search-input" class="sr-only">{t.label}</label>
    <!-- svelte-ignore a11y_autofocus: the screen exists to type a search -->
    <input
      id="search-input"
      type="search"
      autocomplete="off"
      spellcheck="false"
      autofocus
      placeholder={t.placeholder}
      bind:value={query}
      oninput={() => {
        chosen = null;
        refine({ q: query.trim() || null });
      }}
    />
  </header>

  {#if !query.trim()}
    <p class="message">{t.typeToSearch}</p>
  {:else if error}
    <p class="message error" role="alert">{error}</p>
  {:else if hits === null}
    <p class="message" role="status">{t.loading}</p>
  {:else if hits.length === 0}
    <p class="message" role="status">{t.noResults(query.trim())}</p>
  {:else}
    <nav class="chips" aria-label={t.types}>
      <button class="chip" aria-pressed={!type} onclick={() => refine({ type: null })}>
        {t.all} <span class="count">{t.count(hits.length)}</span>
      </button>
      {#each groups as group (group.kind)}
        <button class="chip" aria-pressed={type === group.kind} onclick={() => refine({ type: group.kind })}>
          {kindLabel(group.kind)} <span class="count">{t.count(group.hits.length)}</span>
        </button>
      {/each}
    </nav>

    <div class="panes">
      <div class="results" use:keepScroll={{ id: "search", url: page.url.href, ready: hits !== null }}>
        {#each shown as group (group.kind)}
          <section aria-labelledby="group-{group.kind}">
            <h2 id="group-{group.kind}" class="label">{kindLabel(group.kind)}</h2>
            <ul>
              {#each group.hits as hit (key(hit))}
                <li>
                  <button class="result" aria-current={hit === selected ? "true" : undefined} onclick={() => (chosen = key(hit))}>
                    <span class="name">{hit.name}</span>
                    <span class="meta">{hit.source} · {placeLabel(hit)}</span>
                  </button>
                </li>
              {/each}
            </ul>
          </section>
        {/each}
      </div>

      {#if selected}
        <aside class="preview" aria-label={selected.name}>
          <header>
            <h2>{selected.name}</h2>
            <a class="btn" href={hitAddress(selected)}>{strings.preview.open}</a>
          </header>
          <div class="body prose">
            {#if previewError}
              <p class="error" role="alert">{previewError}</p>
            {:else if !preview}
              <p class="muted" role="status">{strings.preview.loading}</p>
            {:else}
              <p class="muted">{preview.label}</p>
              {#if preview.facts.length}
                <dl class="facts">
                  {#each preview.facts as fact (fact.label)}<dt>{fact.label}</dt><dd>{fact.value}</dd>{/each}
                </dl>
              {/if}
              <!-- Rendered in Rust from the pack's text, with raw HTML, links and images removed. -->
              {@html preview.html}
            {/if}
          </div>
        </aside>
      {/if}
    </div>
  {/if}

  <div class="corner"><SettingsButton bind:open={settingsOpen} /></div>
</div>

<style>
  .search {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .top {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-5);
    border-bottom: var(--border-width) solid var(--color-border);
    background: var(--color-surface);
  }

  .top input {
    flex: 1;
    max-width: var(--start-width);
    min-height: var(--control-height);
    padding: 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .top input::placeholder {
    color: var(--color-text-muted);
  }

  .message {
    margin: 0;
    padding: var(--space-5);
    color: var(--color-text-muted);
  }

  .chips {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-5);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .count {
    color: var(--color-text-muted);
  }

  .panes {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr);
    border-top: var(--border-width) solid var(--color-border);
  }

  .results {
    overflow-y: auto;
    padding: var(--space-2) var(--space-3) var(--corner-clearance);
    border-right: var(--border-width) solid var(--color-border);
  }

  .results h2 {
    margin: var(--space-3) var(--space-2) var(--space-1);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .result {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--color-text);
    text-align: left;
    cursor: pointer;
  }

  .result:hover {
    background: var(--color-surface-sunken);
  }

  .result[aria-current="true"] {
    background: var(--color-accent-soft);
  }

  .name {
    font-weight: var(--weight-medium);
  }

  .meta,
  .muted {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .preview {
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .preview header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-5);
    border-bottom: var(--border-width) solid var(--color-border);
  }

  .preview h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-semibold);
  }

  .preview .btn {
    color: var(--color-text);
    text-decoration: none;
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    max-width: none;
    padding: var(--space-3) var(--space-5) var(--space-5);
  }

  .error {
    color: var(--color-danger);
  }

  .corner {
    position: fixed;
    left: var(--space-4);
    bottom: var(--space-2);
  }

  @media (max-width: 900px) {
    .panes {
      grid-template-columns: 1fr;
    }

    .preview {
      display: none;
    }
  }
</style>
