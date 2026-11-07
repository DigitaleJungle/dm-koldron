<script lang="ts">
  // The campaign's characters (docs/Characters (light), without its page title, count line and dashed add card): the
  // party on a line-coloured band with its editable name, then search by name, class or player (?q=), a party filter (?party=in|out), sort by name,
  // level or class (?sort=level|class), and the cards. A card's name opens its form; in its bottom-right corner a party
  // button (users when in the party, turning into users-minus on hover; users-plus on card hover otherwise); its menu
  // (top right) has Edit, Add to / Remove from party and Delete. ?edit=<id> opens that character's form (from search).
  import { tick } from "svelte";
  import { afterNavigate } from "$app/navigation";
  import { page } from "$app/state";
  import { catalogRows } from "$lib/api/catalog";
  import { pictureSrc } from "$lib/api/campaigns";
  import { deleteCharacter, listCharacters, saveCharacter, type Character, type CharacterList } from "$lib/api/characters";
  import { errorMessage } from "$lib/api/storage";
  import HexCollage from "$lib/components/HexCollage.svelte";
  import Menu, { type MenuItem } from "$lib/components/Menu.svelte";
  import PartyIcon from "$lib/components/PartyIcon.svelte";
  import { campaigns, setPartyName } from "$lib/stores/campaigns.svelte";
  import { gotoReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import { strings } from "$lib/strings";
  import CharacterForm from "./CharacterForm.svelte";
  import { averageLevel, blankCharacter, classLine, selectCharacters, totalLevel, type CharacterSort, type PartyFilter } from "./characters";

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
  const shown = $derived(list ? selectCharacters(list.characters, search, sort, filter) : []);
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

  let deletingId = $state<string | null>(null);
  let rowError = $state<{ id: string; text: string } | null>(null);
  let busy = $state(false);
  /** The character whose party button the pointer (or keyboard focus) is on: it then shows "remove". */
  let pointing = $state<string | null>(null);
  const partyIcon = (c: Character) => (!c.onHome ? "plus" : pointing === c.id ? "minus" : "users");

  async function focusMenu(id: string) {
    await tick();
    document.getElementById(`character-menu-${id}`)?.focus();
  }

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

  async function askDelete(character: Character) {
    deletingId = character.id;
    await tick();
    document.getElementById(`cancel-delete-${character.id}`)?.focus();
  }

  function cancelDelete(character: Character) {
    deletingId = null;
    focusMenu(character.id);
  }

  const confirmDelete = (character: Character) =>
    run(character, t.deleteError, async () => {
      await deleteCharacter(campaign, character.id);
      if (list) list = { ...list, characters: list.characters.filter((c) => c.id !== character.id) };
      deletingId = null;
      notify(t.deleted(character.name));
    });

  const itemsFor = (character: Character): MenuItem[] => [
    { label: t.edit, onselect: () => (editing = character) },
    { label: character.onHome ? t.removeFromHome : t.addToHome, onselect: () => toggleHome(character) },
    { label: t.delete, onselect: () => askDelete(character) },
  ];
</script>

<svelte:head><title>{t.title}</title></svelte:head>

<div class="characters" use:keepScroll={{ id: "characters", url: page.url.href, ready: !!list }}>
  {#if party.length > 0}
    <section class="party" aria-label={t.party}>
      <!-- The party's pictures as hexagons, placed like an encounter's monsters (the same each visit). -->
      <div class="party-art">
        <HexCollage
          tiles={party.map((c) => ({ key: c.id, picture: c.picture ? pictureSrc(c.picture) : null, letter: c.name.charAt(0) }))}
          seed={campaign}
          hexWidth="var(--party-hex-width)"
          maxRows={2}
        />
      </div>
      <div class="party-middle">
        <label class="party-name">
          <span class="sr-only">{t.partyName}</span>
          <input value={partyName} maxlength="100" autocomplete="off" onchange={(e) => renameParty(e.currentTarget)} onkeydown={revertParty} />
        </label>
        {#if partyError}<p class="error" role="alert">{partyError}</p>{/if}
        <p class="party-names">{party.map((c) => c.name).join(", ")}</p>
      </div>
      <dl class="party-stats">
        <div>
          <dt class="band-label">{t.members}</dt>
          <dd class="dice">{new Intl.NumberFormat().format(party.length)}</dd>
        </div>
        <div>
          <dt class="band-label">{t.averageLevel}</dt>
          <dd class="dice">{averageLevel(party)}</dd>
        </div>
      </dl>
    </section>
  {/if}

  <div class="tools">
    <label for="character-search" class="sr-only">{t.search}</label>
    <input id="character-search" class="text-field" type="search" placeholder={t.search} autocomplete="off" bind:value={search} oninput={() => refine({ q: search || null })} />
    <div class="chips" role="group" aria-label={t.filter}>
      {#each FILTERS as each (each)}
        <button class="chip" aria-pressed={filter === each} onclick={() => refine({ party: each === "all" ? null : each })}>{t.filters[each]}</button>
      {/each}
    </div>
    <label class="sort">
      <span>{t.sort}</span>
      <select value={sort} onchange={(e) => refine({ sort: e.currentTarget.value === "name" ? null : e.currentTarget.value })}>
        <option value="name">{t.byName}</option>
        <option value="level">{t.byLevel}</option>
        <option value="class">{t.byClass}</option>
      </select>
    </label>
    <button class="btn btn-primary add" onclick={() => (editing = blankCharacter())}>
      <i class="ti ti-plus" aria-hidden="true"></i>
      {t.add}
    </button>
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
          <li>
            {#if deletingId === character.id}
              <div class="confirm" role="alertdialog" aria-labelledby="delete-text-{character.id}" tabindex="-1" onkeydown={(e) => e.key === "Escape" && cancelDelete(character)}>
                <p id="delete-text-{character.id}">{t.confirmDelete(character.name)}</p>
                <div class="actions">
                  <button id="cancel-delete-{character.id}" class="btn small" onclick={() => cancelDelete(character)}>{t.cancel}</button>
                  <button class="btn btn-danger small" disabled={busy} onclick={() => confirmDelete(character)}>{t.delete}</button>
                </div>
              </div>
            {:else}
              <article class="card">
                <div class="head">
                  <!-- A square picture with the total level on a dark hexagon at its lower right. -->
                  <div class="portrait">
                    {#if character.picture}
                      <img class="picture" src={pictureSrc(character.picture)} alt="" />
                    {:else}
                      <span class="picture no-picture" aria-hidden="true"><i class="ti ti-user"></i></span>
                    {/if}
                    <span class="hex level" aria-hidden="true">{totalLevel(character)}</span>
                  </div>
                  <div class="info">
                    <!-- The name's button covers the whole card: anywhere on it opens the form. -->
                    <h2><button class="name" onclick={() => (editing = character)}>{character.name}</button></h2>
                    <p>{classLine(character)}</p>
                    <p class="label">{t.levelLabel} <span class="dice">{totalLevel(character)}</span></p>
                  </div>
                </div>
                <div class="menu">
                  <Menu id="character-menu-{character.id}" label={t.actionsFor(character.name)} items={itemsFor(character)} triggerClass="icon-btn">
                    {#snippet trigger()}<i class="ti ti-dots" aria-hidden="true"></i>{/snippet}
                  </Menu>
                </div>
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
              </article>
            {/if}
            {#if rowError?.id === character.id}<p class="error" role="alert">{rowError.text}</p>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>

{#if editing}
  <CharacterForm {campaign} character={editing} {classNames} onclose={() => (editing = null)} onsaved={saved} />
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

  /* The party, on a band in the line colour (light beige in light, dark grey in dark). */
  .party {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-6);
    padding: var(--space-6);
    border-radius: var(--radius-md);
    background: var(--line);
    color: var(--color-text);
  }

  /* Exactly two hexagons high (two rows, the gap between them and the collage's margin: 1.982 hexagon widths), so the
     band never changes height: the collage grows sideways (maxRows), and shrinks only if it gets wider than the band. */
  .party-art {
    display: flex;
    align-items: center;
    height: calc(1.982 * var(--party-hex-width));
  }

  /* Between the pictures and the stats: the party's name at the top, the members' names at the bottom, both centred. */
  .party-middle {
    flex: 1;
    align-self: stretch;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: var(--space-2);
    text-align: center;
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

  .party-middle .error {
    margin: 0;
  }

  .party-names {
    margin: 0;
    font: var(--text-body-sm);
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
    margin: 0 0 0 auto;
  }

  .party-stats div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .party-stats dd {
    margin: 0;
    font-size: var(--text-md);
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

  .sort {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .add {
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

  .card {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .card:hover {
    border-color: var(--color-border-strong);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .portrait {
    position: relative;
    flex: none;
    width: var(--character-portrait-size);
    height: var(--character-portrait-size);
  }

  /* Pictures are square (they're saved square), with the card's corners. */
  .picture {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    border-radius: var(--radius-md);
    object-fit: cover;
  }

  .hex {
    display: grid;
    place-items: center;
    clip-path: polygon(25% 0, 75% 0, 100% 50%, 75% 100%, 25% 100%, 0 50%);
  }

  .no-picture {
    background: var(--color-border);
    color: var(--color-text-muted);
    font-size: var(--icon-size-lg);
  }

  /* The total level on a dark hexagon at the portrait's lower right, like the ×count on an encounter's monster. */
  .hex.level {
    position: absolute;
    right: calc(-1 * var(--space-2));
    bottom: calc(-1 * var(--space-1));
    width: var(--character-level-width);
    height: var(--character-level-height);
    background: var(--iron);
    color: var(--bone);
    font: var(--text-dice);
  }

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding-right: var(--control-height-sm); /* clear of the menu in the corner */
  }

  .info p {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .label .dice {
    letter-spacing: 0;
  }

  h2 {
    margin: 0;
    overflow: hidden;
    font: var(--text-h2);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name {
    padding: 0;
    border: none;
    background: none;
    color: var(--color-text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  /* Covers the card, under the menu and the party button. */
  .name::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: var(--radius-md);
  }

  .name:focus-visible {
    outline: none;
  }

  .name:focus-visible::after {
    outline: var(--focus-ring) solid var(--color-focus);
    outline-offset: 2px;
  }

  .card:hover .name {
    text-decoration: underline;
    text-decoration-thickness: 1px;
    text-underline-offset: 4px;
  }

  /* The menu in the top-right corner, the party button in the bottom-right one; both above the card's cover. */
  /* One above the party button (on this card and the next), so the open menu is drawn over it. */
  .menu {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    z-index: 2;
  }

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
    transition: opacity 150ms ease;
  }

  .card:hover .party-toggle,
  .card:has(:focus-visible) .party-toggle {
    opacity: 1;
  }

  :global([data-motion="reduce"]) .party-toggle {
    transition: none;
  }

  .confirm {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: var(--border-width) solid var(--color-danger);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    font-size: var(--text-sm);
  }

  .confirm p {
    margin: 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .small {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
    font-size: var(--text-sm);
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
