<script lang="ts">
  // The search screen: every result for ?q=, grouped by type, with type chips (?type=) and a preview pane.
  import { untrack } from "svelte";
  import { afterNavigate, goto } from "$app/navigation";
  import AddonFrame from "$lib/components/AddonFrame.svelte";
  import { addons, loadAddons } from "$lib/stores/addons.svelte";
  import { page } from "$app/state";
  import BackButton from "$lib/components/BackButton.svelte";
  import SettingsButton from "$lib/components/SettingsButton.svelte";
  import { settingsClosed } from "$lib/components/SettingsDialog.svelte";
  import { searchAll, type SearchHit } from "$lib/api/search";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns, libraryHome, refreshCampaigns } from "$lib/stores/campaigns.svelte";
  import { gotoReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";
  import "$lib/styles/prose.css";
  import { loadPreview, type Preview as Loaded } from "./preview";
  import { campaignScreenAddress, groupByKind, hitAddress, hitMeta, kindLabel, libraryTabAddress } from "./search";

  const t = strings.search;
  const key = (hit: SearchHit) => `${hit.kind}/${hit.pack}/${hit.index}`;

  let query = $state(page.url.searchParams.get("q") ?? "");
  // Back (or Forward) can land on another search while this screen stays: take its text again.
  afterNavigate(({ type }) => {
    if (type === "popstate") query = page.url.searchParams.get("q") ?? "";
  });

  // ?campaign=: only that campaign's characters (and the packs); Search everything drops it.
  const campaign = $derived(page.url.searchParams.get("campaign") ?? undefined);
  const campaignName = $derived(campaigns.list?.campaigns.find((c) => c.id === campaign)?.name);
  $effect(() => {
    if (campaign && !campaigns.list) refreshCampaigns();
  });
  let hits = $state<SearchHit[] | null>(null); // null while searching
  let error = $state<string | null>(null);
  let asked = 0;
  let settingsOpen = $state(false);
  $effect(() => {
    const mine = ++asked;
    const text = query;
    const scope = campaign;
    void settingsClosed.count; // packs may have been switched on or off: search again
    if (!text.trim()) return void ([hits, error] = [[], null]);
    searchAll(text, undefined, scope).then(
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

  type Preview = Loaded & { label: string };
  let preview = $state<Preview | null>(null);
  let previewError = $state<string | null>(null);
  // Keyed, not by object: each keystroke's search returns new objects for the same results.
  const selectedKey = $derived(selected ? key(selected) : null);
  $effect(() => {
    const wanted = selectedKey;
    const hit = untrack(() => selected);
    [preview, previewError] = [null, null];
    const current = () => wanted === untrack(() => selectedKey);
    // An addon's result has no text preview: its addon's preview page, if it has one, shows it.
    if (hit && !hit.kind.startsWith("addon:")) load(hit).then((p) => current() && (preview = p), (e) => current() && (previewError = errorMessage(e)));
  });

  const load = async (hit: SearchHit): Promise<Preview> => ({ ...(await loadPreview(hit)), label: hitMeta(hit) });

  // An addon's result whose addon has a preview page shows that page, told which entry (`{ id, state }`).
  loadAddons();
  const previewAddon = $derived(selected?.kind.startsWith("addon:") ? addons.list.find((a) => a.id === selected.pack && a.preview) : undefined);
  const previewContext = $derived(selected ? { id: selected.index, state: selected.state } : undefined);
  /** Its page's `ui.openLibrary`: the addon's Library tab, at that state. */
  const openLibrary = (id: string) => (state: string | null) => goto(libraryTabAddress(id, state));
  /** Its page's `ui.openCampaignScreen`: the addon's campaign screen in a campaign, at that state. */
  const openCampaignScreen = (id: string) => (campaign: string, state: string | null) => goto(campaignScreenAddress(campaign, id, state));

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

  {#if campaign}
    <p class="scope">
      {#if campaignName}{t.inCampaign(campaignName)}{/if}
      <button class="btn small" onclick={() => refine({ campaign: null })}>{t.everything}</button>
    </p>
  {/if}

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
                    {#if hit.detail}<span class="meta">{hit.detail}</span>{/if}
                    <span class="meta">{hitMeta(hit, false)}</span>
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
            <a class="btn" href={hitAddress(selected, libraryHome())}>{strings.preview.open}</a>
          </header>
          <div class="body prose">
            {#if previewAddon}
              {#key previewAddon.id}
                <AddonFrame
                  addon={previewAddon}
                  entry={previewAddon.preview!}
                  context={previewContext}
                  onopenlibrary={previewAddon.library && previewAddon.libraryVisible ? openLibrary(previewAddon.id) : undefined}
                  onopencampaignscreen={openCampaignScreen(previewAddon.id)}
                />
              {/key}
            {:else if selected.kind.startsWith("addon:")}
              <!-- An addon without a preview page: the name and Open only. -->
            {:else if previewError}
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

  .scope {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin: 0;
    padding: var(--space-3) var(--space-5) 0;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .small {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
    font-size: var(--text-sm);
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

  /* An addon's preview page takes the pane's whole width (a frame is otherwise 300px, the browser's default). */
  .preview .body :global(iframe) {
    display: block;
    width: 100%;
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
    /* The pane's whole width: centred, it would shrink to what it holds (an addon's frame starts at 300px). */
    width: 100%;
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
