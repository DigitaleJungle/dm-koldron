<script lang="ts">
  // The campaign's characters (docs/Characters (light), without its page title, count line and dashed add card): the
  // party on a line-coloured band (its editable name and Add character; pictures, names and stats once anyone is in
  // it), then search by name, class or player (?q=), a party filter (?party=in|out), a class filter (?class=), sort by
  // name, level or class (?sort=level|class) at the right, and the cards. A card shows the picture (total level on a
  // hexagon) with, beside it from the top, the name, class line and HP; the name opens its form; in its bottom-right
  // corner a party button (users when in the party, turning into users-minus on hover; users-plus on card hover
  // otherwise). Delete is in the form. ?edit=<id> opens that character's form (from search).
  import { afterNavigate } from "$app/navigation";
  import { flip } from "svelte/animate";
  import { scale } from "svelte/transition";
  import { page } from "$app/state";
  import { catalogRows } from "$lib/api/catalog";
  import { pictureSrc } from "$lib/api/campaigns";
  import { listCharacters, saveCharacter, type Character, type CharacterList } from "$lib/api/characters";
  import { errorMessage } from "$lib/api/storage";
  import type { AddonSummary } from "$lib/api/addons";
  import AddonDialog from "$lib/components/AddonDialog.svelte";
  import HexCollage from "$lib/components/HexCollage.svelte";
  import Menu from "$lib/components/Menu.svelte";
  import { addons, loadAddons } from "$lib/stores/addons.svelte";
  import PartyIcon from "$lib/components/PartyIcon.svelte";
  import { campaigns, setPartyName } from "$lib/stores/campaigns.svelte";
  import { enterMotion, moveMotion } from "$lib/motion";
  import { gotoReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import { strings } from "$lib/strings";
  import CharacterCard from "./CharacterCard.svelte";
  import CharacterForm from "./CharacterForm.svelte";
  import { averageLevel, blankCharacter, classNamesOf, selectCharacters, type CharacterSort, type PartyFilter } from "./characters";

  const t = strings.characters;
  const campaign = $derived(campaigns.current!.id);

  let list = $state<CharacterList | null>(null);
  let loadError = $state<string | null>(null);
  let classNames = $state<string[]>([]);

  $effect(() => {
    const id = campaign;
    let current = true; // ignore an answer for a campaign we've left
    [list, loadError] = [null, null];
    listCharacters(id).then(
      (l) => current && (list = l),
      (e) => current && (loadError = t.loadError(errorMessage(e))),
    );
    return () => {
      current = false;
    };
  });

  // An addon saved a character (a D&D Beyond sync): read them again, without the loading state.
  $effect(() => {
    if (addons.charactersChanged === 0) return;
    listCharacters(campaign).then(
      (l) => (list = l),
      (e) => (loadError = t.loadError(errorMessage(e))),
    );
  });

  // Addons that add a way to add characters; with any, Add character becomes a menu.
  loadAddons();
  const adders = $derived(addons.list.filter((a) => a.characters));
  /** An addon's page open in a dialog: adding, or editing the link of a character linked to it. */
  let addonPage = $state<{ addon: AddonSummary; character: string | null } | null>(null);
  const addItems = $derived([
    { label: t.newCharacter, onselect: () => (editing = blankCharacter()) },
    ...adders.map((addon) => ({ label: addon.characters!.label, onselect: () => (addonPage = { addon, character: null }) })),
  ]);
  // A link counts only while its addon is loaded; without it the character is an ordinary one.
  const linkedAddon = (c: Character) => addons.list.find((a) => a.id === c.linked);

  function editLink(addon: AddonSummary, character: string) {
    editing = null;
    addonPage = { addon, character };
  }

  // The packs' classes, for the form's class field; without them a class is just typed.
  catalogRows("classes", []).then(
    (rows) => (classNames = [...new Set(rows.map((r) => r.name))].sort()),
    () => {},
  );

  let search = $state(page.url.searchParams.get("q") ?? "");
  afterNavigate(({ type }) => {
    if (type === "popstate") search = page.url.searchParams.get("q") ?? "";
  });
  const SORTS: CharacterSort[] = ["name", "level", "class"];
  const FILTERS: PartyFilter[] = ["all", "in", "out"];
  const sort = $derived<CharacterSort>(SORTS.find((s) => s === page.url.searchParams.get("sort")) ?? "name");
  const filter = $derived<PartyFilter>(FILTERS.find((f) => f === page.url.searchParams.get("party")) ?? "all");
  const classFilter = $derived(page.url.searchParams.get("class") ?? "");
  // The classes the campaign's characters have, plus the one asked for (?class=) should none have it any more.
  const classOptions = $derived.by(() => {
    const names = classNamesOf(list?.characters ?? []);
    return classFilter && !names.some((n) => n.toLowerCase() === classFilter.toLowerCase()) ? [classFilter, ...names] : names;
  });
  const shown = $derived(list ? selectCharacters(list.characters, search, sort, filter, classFilter) : []);
  const party = $derived((list?.characters ?? []).filter((c) => c.onHome).sort((a, b) => a.name.localeCompare(b.name)));

  // The party's name saves on Enter or leaving the field; Escape puts the saved one back. Empty goes back to the default.
  const partyName = $derived(campaigns.current?.partyName ?? t.defaultPartyName(campaigns.current?.name ?? ""));
  let partyError = $state<string | null>(null);

  async function renameParty(input: HTMLInputElement) {
    partyError = null;
    try {
      await setPartyName(campaign, input.value);
    } catch (e) {
      partyError = t.partyNameError(errorMessage(e));
    }
    input.value = partyName; // also when the shown name didn't change (e.g. cleared while already the default)
  }

  function revertParty(e: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    if (e.key !== "Escape") return;
    e.currentTarget.value = partyName;
    e.currentTarget.blur();
  }

  // Search, filter and sort refine the list in place (no Back step).
  function refine(changes: Record<string, string | null>) {
    const params = new URLSearchParams(page.url.searchParams);
    for (const [name, value] of Object.entries(changes)) value ? params.set(name, value) : params.delete(name);
    const query = params.toString();
    return gotoReplacing(`${page.url.pathname}${query ? `?${query}` : ""}`, { keepFocus: true, noScroll: true });
  }

  // The form: a character to edit or add. From search, ?edit=<id> opens it once the list is there.
  let editing = $state<Character | null>(null);
  $effect(() => {
    const wanted = page.url.searchParams.get("edit");
    if (wanted && list) {
      editing = list.characters.find((c) => c.id === wanted) ?? null;
      refine({ edit: null });
    }
  });

  function saved(character: Character) {
    if (!list) return;
    const others = list.characters.filter((c) => c.id !== character.id);
    list = { ...list, characters: [...others, character] };
  }

  let rowError = $state<{ id: string; text: string } | null>(null);
  let busy = $state(false);
  /** The character whose party button the pointer (or keyboard focus) is on: it then shows "remove". */
  let pointing = $state<string | null>(null);
  const partyIcon = (c: Character) => (!c.onHome ? "plus" : pointing === c.id ? "minus" : "users");

  async function run(character: Character, failed: (name: string, reason: string) => string, work: () => Promise<void>) {
    busy = true;
    rowError = null;
    try {
      await work();
    } catch (e) {
      rowError = { id: character.id, text: failed(character.name, errorMessage(e)) };
    } finally {
      busy = false;
    }
  }

  const toggleHome = (character: Character) =>
    run(character, t.homeError, async () => saved(await saveCharacter(campaign, { ...character, onHome: !character.onHome })));

  const formatNumber = new Intl.NumberFormat();

  // Deleted from its form: gone from the list.
  function deleted(character: Character) {
    if (list) list = { ...list, characters: list.characters.filter((c) => c.id !== character.id) };
    notify(t.deleted(character.name));
  }
</script>

<svelte:head><title>{t.title}</title></svelte:head>

<div class="characters" use:keepScroll={{ id: "characters", url: page.url.href, ready: !!list }}>
  <!-- Two layers of the same size: the background with the pictures, and on top of it all the text and controls. -->
  <section class="party" aria-label={t.party}>
    <div class="party-art" class:filled={party.length > 0}>
      {#if party.length > 0}
        <!-- The party's pictures as hexagons, placed like an encounter's monsters (the same each visit). -->
        <HexCollage
          tiles={party.map((c) => ({ key: c.id, picture: c.picture ? pictureSrc(c.picture) : null, letter: c.name.charAt(0) }))}
          seed={campaign}
          hexWidth="var(--party-hex-width)"
          maxRows={2.5}
        />
      {/if}
    </div>
    <div class="party-content">
      <div class="party-title">
        <label class="party-name">
          <span class="sr-only">{t.partyName}</span>
          <input value={partyName} maxlength="100" autocomplete="off" onchange={(e) => renameParty(e.currentTarget)} onkeydown={revertParty} />
        </label>
        {#if partyError}<p class="error" role="alert">{partyError}</p>{/if}
        {#if party.length > 0}
          <ul class="party-names">
            {#each party as c (c.id)}<li>{c.name}</li>{/each}
          </ul>
        {/if}
      </div>
      {#if party.length > 0}
        <dl class="party-stats">
          <div>
            <dt class="band-label">{t.members}</dt>
            <dd>{formatNumber.format(party.length)}</dd>
          </div>
          <div>
            <dt class="band-label">{t.averageLevel}</dt>
            <dd>{averageLevel(party)}</dd>
          </div>
        </dl>
      {/if}
      {#if adders.length > 0}
        <div class="add">
          <Menu id="add-character" label={t.add} items={addItems} triggerClass="btn btn-primary">
            {#snippet trigger()}
              <i class="ti ti-plus" aria-hidden="true"></i>
              {t.add}
              <i class="ti ti-chevron-down" aria-hidden="true"></i>
            {/snippet}
          </Menu>
        </div>
      {:else}
        <button class="btn btn-primary add" onclick={() => (editing = blankCharacter())}>
          <i class="ti ti-plus" aria-hidden="true"></i>
          {t.add}
        </button>
      {/if}
    </div>
  </section>

  <div class="tools">
    <label for="character-search" class="sr-only">{t.search}</label>
    <input id="character-search" class="text-field" type="search" placeholder={t.search} autocomplete="off" bind:value={search} oninput={() => refine({ q: search || null })} />
    <div class="chips" role="group" aria-label={t.filter}>
      <!-- Pressing the pressed one again turns the filter off (all characters). -->
      {#each ["in", "out"] as const as each (each)}
        <button class="chip" aria-pressed={filter === each} onclick={() => refine({ party: filter === each ? null : each })}>{t.filters[each]}</button>
      {/each}
    </div>
    <label class="select">
      <span>{t.classFilter}</span>
      <select value={classFilter} onchange={(e) => refine({ class: e.currentTarget.value || null })}>
        <option value="">{t.allClasses}</option>
        {#each classOptions as name (name)}<option value={name}>{name}</option>{/each}
      </select>
    </label>
    <label class="select sort">
      <span>{t.sort}</span>
      <select value={sort} onchange={(e) => refine({ sort: e.currentTarget.value === "name" ? null : e.currentTarget.value })}>
        <option value="name">{t.byName}</option>
        <option value="level">{t.byLevel}</option>
        <option value="class">{t.byClass}</option>
      </select>
    </label>
  </div>

  {#if loadError}
    <p class="message error" role="alert">{loadError}</p>
  {:else if list === null}
    <p class="message" role="status">{t.loading}</p>
  {:else}
    {#if list.unreadable.length > 0}
      <p class="message error" role="alert">{t.unreadable(list.unreadable.join(", "))}</p>
    {/if}
    {#if list.characters.length === 0}
      <p class="message">{t.empty}</p>
    {:else if shown.length === 0}
      <p class="message" role="status">{t.noMatches}</p>
    {:else}
      <ul class="cards">
        {#each shown as character (character.id)}
          <li animate:flip={moveMotion()} in:scale={enterMotion()}>
            <CharacterCard {character} onclick={() => (editing = character)}>
              {#snippet corner()}
                <button
                  class="icon-btn party-toggle"
                  class:in={character.onHome}
                  aria-label={character.onHome ? t.removeFromHome : t.addToHome}
                  aria-disabled={busy}
                  onclick={() => !busy && toggleHome(character)}
                  onpointerenter={() => (pointing = character.id)}
                  onpointerleave={() => (pointing = null)}
                  onfocus={(e) => e.currentTarget.matches(":focus-visible") && (pointing = character.id)}
                  onblur={() => (pointing = null)}
                >
                  <PartyIcon shape={partyIcon(character)} />
                </button>
              {/snippet}
            </CharacterCard>
            {#if rowError?.id === character.id}<p class="error" role="alert">{rowError.text}</p>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>

{#if editing}
  {@const source = linkedAddon(editing)}
  {@const id = editing.id}
  <CharacterForm
    {campaign}
    character={editing}
    {classNames}
    linkedTo={source ? (source.characters?.label ?? source.name) : undefined}
    oneditlink={source?.characters && id ? () => editLink(source, id) : undefined}
    onclose={() => (editing = null)}
    onsaved={saved}
    ondeleted={deleted}
  />
{/if}
{#if addonPage}
  <AddonDialog
    addon={addonPage.addon}
    entry={addonPage.addon.characters!.entry}
    title={addonPage.addon.characters!.label}
    context={{ campaign, character: addonPage.character }}
    onclose={() => (addonPage = null)}
  />
{/if}

<style>
  .characters {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    height: 100%;
    overflow-y: auto;
    padding: var(--space-5) var(--space-5) var(--corner-clearance);
  }

  /* The party: two layers in one grid cell, so both are exactly the band's size. The line colour is light beige in
     light, dark grey in dark. */
  .party {
    display: grid;
    border-radius: var(--radius-md);
    background: var(--line);
    color: var(--color-text);
  }

  .party-art,
  .party-content {
    grid-area: 1 / 1;
    padding: var(--space-4);
  }

  /* Exactly two and a half hexagons high (1.5 steps of a row and its gap, one hexagon and the collage's margin: 2.44
     hexagon widths), so the band never changes height: the collage grows sideways (maxRows), and shrinks only if it
     gets wider than the band. Without a party there's no collage and the text layer alone sets the height. */
  .party-art {
    display: flex;
    align-items: center;
  }

  .party-art.filled {
    height: calc(2.44 * var(--party-hex-width) + 2 * var(--space-4));
  }

  /* Equal side columns keep the middle (the party's name and the members' names) centred on the band; the right one
     grows only when the button wouldn't fit. The names in the middle of the band, stats top right, Add character
     bottom right. */
  .party-content {
    display: grid;
    grid-template-columns: 1fr minmax(0, 2fr) minmax(max-content, 1fr);
    grid-template-rows: auto 1fr auto;
    align-items: start;
    gap: var(--space-2) var(--space-3);
    pointer-events: none; /* the pictures underneath stay reachable */
  }

  .party-content > * {
    pointer-events: auto;
  }

  /* The party's name with the members' names right under it, together in the band's middle. */
  .party-title {
    grid-area: 1 / 2 / 4 / 3;
    align-self: center;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .party-stats {
    grid-area: 1 / 3;
    justify-self: end;
  }

  .party .error {
    margin: 0;
    text-align: center;
  }

  .add {
    grid-area: 3 / 3;
    justify-self: end;
    align-self: end;
  }

  /* Text until hovered or focused, like an encounter's name in Prepare. */
  .party-name input {
    width: 100%;
    min-height: var(--control-height);
    padding: 0 var(--space-2);
    border: var(--border-width) solid transparent;
    border-radius: var(--radius-md);
    background: none;
    color: inherit;
    font: var(--text-h2);
    text-align: center;
  }

  .party-name input:hover,
  .party-name input:focus {
    border-color: var(--color-border-strong);
    background: var(--color-surface);
  }

  /* The members' names, centred at the bottom, a dot between them. */
  .party-names {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0 var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
    font: var(--text-body-sm);
  }

  .party-names li + li::before {
    content: "·" / ""; /* decorative: screen readers hear a list of names */
    margin-right: var(--space-2);
    font-weight: var(--weight-bold);
  }

  .band-label {
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    color: color-mix(in srgb, var(--color-text) 80%, transparent); /* --color-text-muted is just under 4.5:1 on --line */
  }

  .party-stats {
    display: flex;
    gap: var(--space-6);
    margin: 0;
  }

  /* Each number centred under its label, in the party name's font. */
  .party-stats div {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .party-stats dd {
    margin: 0;
    font: var(--text-h2);
    font-variant-numeric: lining-nums tabular-nums;
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }

  .tools input {
    flex: 0 1 var(--search-width);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .chips .chip {
    min-height: var(--control-height);
    padding: 0 var(--space-3);
  }

  .select {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  /* Sort at the right end of the row. */
  .sort {
    margin-left: auto;
  }

  .message {
    margin: 0;
    color: var(--color-text-muted);
  }

  .cards {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--character-card-width), 1fr));
    gap: var(--space-4);
  }

  .cards li {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  /* The party button in the bottom-right corner, above the card's cover. */
  .party-toggle {
    position: absolute;
    right: var(--space-2);
    bottom: var(--space-2);
    z-index: 1;
  }

  .party-toggle.in {
    color: var(--color-accent);
  }

  /* Outside the party, the button shows on card hover, and while keyboard focus is in the card (so the keyboard finds
     it). Not on mere focus: a clicked button keeps focus, so it would stay on one card and vanish on the next click. */
  .party-toggle:not(.in) {
    opacity: 0;
    transition-property: opacity, color, background-color;
    transition-duration: var(--duration-hover);
    transition-timing-function: var(--ease-hover);
  }

  /* The card is CharacterCard's (its own styles); the button is ours, in its corner. */
  :global(.card:hover) .party-toggle,
  :global(.card:has(:focus-visible)) .party-toggle {
    opacity: 1;
  }

  :global([data-motion="reduce"]) .party-toggle {
    transition: none;
  }

  .error {
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  .message.error {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
  }
</style>
