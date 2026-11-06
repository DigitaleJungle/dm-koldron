<script lang="ts">
  // The campaign's encounters as cards: a hexagon collage of the monsters (one per roster row, placed at random but
  // the same each visit), name, Draft / Ready, the monsters, difficulty, XP and party. Search (?q=), difficulty
  // filter (?band=) and sort (?sort=name|xp) refine the list in place; a card opens the encounter in Prepare.
  import { afterNavigate } from "$app/navigation";
  import { page } from "$app/state";
  import { catalogRows, type CatalogRow } from "$lib/api/catalog";
  import { listCharacters, type Character } from "$lib/api/characters";
  import { encounterBudgets, listEncounters, type Budget, type Encounter, type EncounterList } from "$lib/api/encounters";
  import { entryImage } from "$lib/api/images";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { gotoReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import { strings } from "$lib/strings";
  import { BANDS, difficulty, monsterXp, type Band } from "./difficulty";
  import { HEX_HEIGHT, combSize, hexPoints, hexcomb } from "./hexcomb";
  import { characterLevel, entryKey, rowsByKey } from "./roster";

  const t = strings.encounters;
  const fmt = new Intl.NumberFormat();
  const campaign = $derived(campaigns.current!.id);
  const base = $derived(`/campaign/${campaign}/encounters`);

  let list = $state<EncounterList | null>(null);
  let loadError = $state<string | null>(null);
  // What the cards need besides the encounters; without it the cards show no rating (and letters for pictures).
  let monsters = $state<CatalogRow[]>([]);
  let characters = $state<Character[]>([]);
  let budgets = $state<Budget[]>([]);
  let lookupErrors = $state<string[]>([]);

  $effect(() => {
    const id = campaign;
    let current = true; // ignore an answer for a campaign we've left
    [list, loadError, lookupErrors] = [null, null, []];
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
  const characterById = $derived(new Map(characters.map((c) => [c.id, c])));

  // Pictures, asked once per monster, when its pack's images are on.
  let pictures = $state<Record<string, string | null>>({});
  const askedPictures = new Set<string>();
  $effect(() => {
    for (const row of list?.encounters.flatMap((e) => e.monsters) ?? []) {
      const key = entryKey(row);
      const image = byKey.get(key)?.fields.image;
      if (askedPictures.has(key) || typeof image !== "string") continue;
      askedPictures.add(key);
      entryImage(row.pack, image).then(
        (src) => (pictures[key] = src),
        (e) => notify(errorMessage(e)),
      );
    }
  });

  type Card = Encounter & { count: number; band: Band | null; xp: number; partyLine: string; tiles: ReturnType<typeof tilesOf>; size: { width: number; height: number } };

  // The strongest monster in the middle, the rest outward in placing order.
  function tilesOf(e: Encounter, xps: (number | null)[]) {
    const rows = e.monsters.map((m, i) => ({ ...m, xp: xps[i] ?? 0 })).sort((a, b) => b.xp - a.xp);
    return hexcomb(rows.length, e.id).map((cell, i) => ({ ...cell, row: rows[i] }));
  }

  function partyText(ids: string[]) {
    if (ids.length === 0) return t.list.noParty;
    const levels = ids.map((id) => characterLevel(characterById.get(id)?.classes));
    const known = levels.every((l) => l !== null) ? (levels as number[]) : null;
    return t.list.party(ids.length, known && Math.min(...known), known && Math.max(...known));
  }

  const cards = $derived<Card[]>(
    (list?.encounters ?? []).map((e) => {
      const xps = e.monsters.map((m) => monsterXp(byKey.get(entryKey(m))?.fields.xp));
      const levels = e.party.map((id) => characterLevel(characterById.get(id)?.classes));
      const result = difficulty(levels, e.monsters.map((m, i) => ({ quantity: m.quantity, xp: xps[i] })), budgets);
      const tiles = tilesOf(e, xps);
      return {
        ...e,
        count: e.monsters.reduce((n, m) => n + m.quantity, 0),
        band: result.band,
        xp: result.total,
        partyLine: partyText(e.party),
        tiles,
        size: combSize(tiles),
      };
    }),
  );

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

  const monsterLine = (c: Card) =>
    `${t.monsterCount(c.count)} · ${c.monsters.map((m) => (m.quantity > 1 ? `${m.name} ×${fmt.format(m.quantity)}` : m.name)).join(", ")}`;
</script>

<svelte:head><title>{t.title}</title></svelte:head>

<div class="encounters" use:keepScroll={{ id: "encounters", url: page.url.href, ready: !!list }}>
  <div class="tools">
    <label for="encounter-search" class="sr-only">{t.list.search}</label>
    <input id="encounter-search" type="search" placeholder={t.list.search} autocomplete="off" bind:value={search} oninput={() => refine({ q: search || null })} />
    <div class="chips" role="group" aria-label={t.list.difficulty}>
      <button class="chip" aria-pressed={band === null} onclick={() => refine({ band: null })}>{t.list.all}</button>
      {#each BANDS as b (b)}
        <button class="chip" aria-pressed={band === b} onclick={() => refine({ band: b })}>{t.meter.bands[b]}</button>
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
      <p class="message">{t.empty}</p>
    {:else if shown.length === 0}
      <p class="message" role="status">{t.list.noMatches}</p>
    {:else}
      <ul class="cards">
        {#each shown as c (c.id)}
          <li>
            <a class="card" href="{base}/{c.id}">
              <div class="art">
                <!-- One unit is one hexagon's width; --hex-width sets its size on screen, shrinking to fit the card. -->
                <svg
                  viewBox="-0.1 -0.1 {c.size.width + 0.2} {c.size.height + 0.2}"
                  style:width="calc({c.size.width + 0.2} * var(--encounter-hex-width))"
                  style:height="calc({c.size.height + 0.2} * var(--encounter-hex-width))"
                  aria-hidden="true"
                >
                  {#if c.tiles.length === 0}
                    <polygon class="blank" points={hexPoints(0, 0)} />
                  {/if}
                  {#each c.tiles as tile (entryKey(tile.row))}
                    {@const picture = pictures[entryKey(tile.row)]}
                    {#if picture}
                      <image href={picture} x={tile.x} y={tile.y} width="1" height={HEX_HEIGHT} preserveAspectRatio="xMidYMid slice" clip-path="url(#encounter-hex)" />
                    {:else}
                      <polygon class="letter-bg" points={hexPoints(tile.x, tile.y)} />
                      <text class="letter" x={tile.x + 0.5} y={tile.y + HEX_HEIGHT / 2}>{tile.row.name.charAt(0)}</text>
                    {/if}
                    {#if tile.row.quantity > 1}
                      <!-- The ×count on a small hexagon at the lower right, as on the Prepare tiles. -->
                      <polygon class="badge" points={hexPoints(tile.x + 0.6, tile.y + HEX_HEIGHT * 0.64, 0.44)} />
                      <text class="badge-text" x={tile.x + 0.82} y={tile.y + HEX_HEIGHT * 0.86}>×{tile.row.quantity}</text>
                    {/if}
                  {/each}
                </svg>
              </div>
              <div class="body">
                <div class="title">
                  <h2 class="name">{c.name}</h2>
                  <span class="status" class:ready={c.count > 0}>{c.count > 0 ? t.list.ready : t.list.draft}</span>
                </div>
                <p class="muted">{c.count > 0 ? monsterLine(c) : t.list.noMonsters}</p>
                <div class="stats">
                  <span class="band band-{c.band ?? 'none'}">
                    <i class="ti ti-hexagon" aria-hidden="true"></i>
                    {c.band ? t.meter.bands[c.band] : t.meter.noRating}
                  </span>
                  <span>{t.meter.xp(fmt.format(c.xp))}</span>
                  <span class="party muted">{c.partyLine}</span>
                </div>
              </div>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>

<!-- The hexagon every picture is cut to, in the picture's own box. -->
<svg class="defs" aria-hidden="true">
  <clipPath id="encounter-hex" clipPathUnits="objectBoundingBox">
    <polygon points="0.25,0 0.75,0 1,0.5 0.75,1 0.25,1 0,0.5" />
  </clipPath>
</svg>

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

  .card {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: var(--color-text);
    text-decoration: none;
  }

  .card:hover {
    border-color: var(--color-border-strong);
  }

  .card:hover .name {
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .art {
    height: var(--encounter-art-height);
    padding: var(--space-4);
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
    border-bottom: var(--border-width) solid var(--color-border);
    background: var(--color-bg);
  }

  .art svg {
    max-width: 100%;
    max-height: 100%;
  }

  .blank {
    fill: var(--color-surface-sunken);
  }

  .letter-bg {
    fill: var(--color-border);
  }

  .letter,
  .badge-text {
    text-anchor: middle;
    dominant-baseline: central;
  }

  .letter {
    fill: var(--color-text-muted);
    font-family: var(--font-display);
    font-size: 0.4px;
    font-weight: var(--weight-display);
  }

  .badge {
    fill: var(--potion-glow);
  }

  .badge-text {
    fill: var(--charcoal);
    font-family: var(--font-mono);
    font-size: 0.19px;
  }

  .defs {
    position: absolute;
    width: 0;
    height: 0;
  }

  .body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4);
  }

  .title {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .name {
    margin: 0;
    font: var(--text-body);
    font-weight: var(--weight-bold);
  }

  .status {
    flex-shrink: 0;
    padding: 2px var(--space-2);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .status.ready {
    border-color: var(--color-success);
    color: var(--color-success);
  }

  .muted {
    margin: 0;
    color: var(--color-text-muted);
    font: var(--text-body-sm);
  }

  .stats {
    margin-top: auto;
    padding-top: var(--space-2);
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-4);
    border-top: var(--border-width) solid var(--color-border);
    font: var(--text-dice);
  }

  .band {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--band);
    font: var(--text-body-sm);
    font-weight: var(--weight-bold);
  }

  .band .ti {
    font-size: var(--icon-size-sm);
  }

  .party {
    margin-left: auto;
  }
</style>
