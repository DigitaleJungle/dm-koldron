<script lang="ts">
  // The global search bar: while typing, a dropdown of the best 8 results. Arrow keys move, Enter opens the
  // highlighted result (or the search screen when none is), Escape closes. A combobox with a listbox (WAI-ARIA).
  import { goto } from "$app/navigation";
  import { searchAll, type SearchHit } from "$lib/api/search";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
  import { hitAddress, hitMeta, searchAddress } from "$lib/views/search/search";

  let { large = false }: { large?: boolean } = $props();

  const t = strings.search;
  const id = $props.id();
  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let error = $state<string | null>(null);
  let open = $state(false);
  let active = $state(-1);
  let asked = 0; // answers to older keystrokes are dropped
  let answered = $state(""); // the text the shown hits are for
  // Inside a campaign (also in the Library opened from it): its characters (and the packs), not other campaigns'.
  const campaign = $derived(campaigns.scope ?? undefined);

  async function find() {
    const mine = ++asked;
    active = -1;
    open = !!query.trim();
    if (!open) return void (hits = []);
    try {
      const found = await searchAll(query, 8, campaign);
      if (mine === asked) [hits, error, answered] = [found, null, query];
    } catch (e) {
      if (mine === asked) [hits, error, answered] = [[], t.error(errorMessage(e)), query];
    }
  }

  function go(address: string) {
    open = false;
    goto(address);
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      if (!open || !hits.length || answered !== query) return; // the hits are for older text
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      active = Math.max(-1, Math.min(hits.length - 1, active + step));
    } else if (event.key === "Enter" && query.trim()) {
      event.preventDefault();
      go(open && active >= 0 ? hitAddress(hits[active]) : searchAddress(query, campaign));
    } else if (event.key === "Escape" && open) {
      event.preventDefault();
      open = false;
    }
  }

  function onfocusout(event: FocusEvent) {
    if (!(event.currentTarget as HTMLElement).contains(event.relatedTarget as Node | null)) open = false;
  }
</script>

<div class="search-bar" class:large {onfocusout}>
  <label for="{id}-input" class="sr-only">{t.label}</label>
  <i class="ti ti-search" aria-hidden="true"></i>
  <input
    id="{id}-input"
    type="search"
    role="combobox"
    autocomplete="off"
    spellcheck="false"
    placeholder={t.placeholder}
    aria-autocomplete="list"
    aria-controls="{id}-list"
    aria-expanded={open}
    aria-activedescendant={open && active >= 0 ? `${id}-${active}` : undefined}
    bind:value={query}
    oninput={find}
    onfocus={() => (open = !!query.trim())}
    {onkeydown}
  />
  <div class="dropdown" hidden={!open}>
    {#if error}
      <p class="message error" role="alert">{error}</p>
    {:else if answered === query && hits.length === 0}
      <p class="message" role="status">{t.noResults(query.trim())}</p>
    {/if}
    <ul id="{id}-list" role="listbox" aria-label={t.results}>
      {#each hits as hit, i (`${hit.kind}/${hit.pack}/${hit.index}`)}
        <!-- Focus stays in the input (aria-activedescendant); a click opens the result. -->
        <!-- svelte-ignore a11y_click_events_have_key_events: the input's arrow keys and Enter drive the options -->
        <li
          id="{id}-{i}"
          role="option"
          aria-selected={i === active}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => go(hitAddress(hit))}
          onmousemove={() => (active = i)}
        >
          <span class="name">{hit.name}</span>
          <span class="meta">{hitMeta(hit)}</span>
        </li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .search-bar {
    position: relative;
    width: 100%;
  }

  .ti-search {
    position: absolute;
    left: var(--space-3);
    top: 50%;
    transform: translateY(-50%);
    color: var(--color-text-muted);
    pointer-events: none;
  }

  input {
    width: 100%;
    min-height: var(--control-height);
    padding: 0 var(--space-3) 0 calc(var(--space-3) + var(--space-5));
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .large input {
    min-height: var(--control-height-lg);
  }

  input::placeholder {
    color: var(--color-text-muted);
  }

  .dropdown {
    position: absolute;
    z-index: var(--z-menu);
    top: calc(100% + var(--space-1));
    left: 0;
    right: 0;
    padding: var(--space-1);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    box-shadow: var(--shadow-drawer);
  }

  .dropdown[hidden] {
    display: none;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    flex-direction: column;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  li[aria-selected="true"] {
    background: var(--color-accent-soft);
  }

  .name {
    font-weight: var(--weight-medium);
  }

  .meta,
  .message {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .message {
    margin: 0;
    padding: var(--space-2) var(--space-3);
  }

  .error {
    color: var(--color-danger);
  }
</style>
