<script lang="ts">
  // The campaign's characters as cards: search by name or player (?q=), sort by name or level (?sort=level), each
  // with a menu (Edit, Add to / Remove from party, Delete). ?edit=<id> opens that character's form (from search).
  import { tick } from "svelte";
  import { fade } from "svelte/transition";
  import { afterNavigate } from "$app/navigation";
  import { page } from "$app/state";
  import { catalogRows } from "$lib/api/catalog";
  import { pictureSrc } from "$lib/api/campaigns";
  import { deleteCharacter, listCharacters, saveCharacter, type Character, type CharacterList } from "$lib/api/characters";
  import { errorMessage } from "$lib/api/storage";
  import Menu, { type MenuItem } from "$lib/components/Menu.svelte";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { gotoReplacing, keepScroll } from "$lib/stores/history.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import { reduceMotion } from "$lib/stores/theme.svelte";
  import { strings } from "$lib/strings";
  import CharacterForm from "./CharacterForm.svelte";
  import { blankCharacter, classLine, selectCharacters, totalLevel, type CharacterSort } from "./characters";

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
  const sort = $derived<CharacterSort>(page.url.searchParams.get("sort") === "level" ? "level" : "name");
  const shown = $derived(list ? selectCharacters(list.characters, search, sort) : []);

  // Search and sort refine the list in place (no Back step).
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
  const partyIcon = (c: Character) => (!c.onHome ? "ti-users-plus" : pointing === c.id ? "ti-users-minus" : "ti-users");
  /** Each half of the icon swap (the new one in, then the old one out); none with reduced motion. */
  const swapMs = () => (reduceMotion() ? 0 : 120);

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
  <div class="tools">
    <label for="character-search" class="sr-only">{t.search}</label>
    <input id="character-search" type="search" placeholder={t.search} autocomplete="off" bind:value={search} oninput={() => refine({ q: search || null })} />
    <label class="sort">
      <span>{t.sort}</span>
      <select value={sort} onchange={(e) => refine({ sort: e.currentTarget.value === "level" ? "level" : null })}>
        <option value="name">{t.byName}</option>
        <option value="level">{t.byLevel}</option>
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
                {#if character.picture}
                  <img class="avatar" src={pictureSrc(character.picture)} alt="" />
                {:else}
                  <span class="avatar" aria-hidden="true"><i class="ti ti-user"></i></span>
                {/if}
                <div class="info">
                  <h2><button class="name" onclick={() => (editing = character)}>{character.name}</button></h2>
                  <p class="muted">{t.level(totalLevel(character))}</p>
                  <p>{classLine(character)}</p>
                </div>
                <Menu id="character-menu-{character.id}" label={t.actionsFor(character.name)} items={itemsFor(character)} triggerClass="icon-btn">
                  {#snippet trigger()}<i class="ti ti-dots" aria-hidden="true"></i>{/snippet}
                </Menu>
                <!-- Bottom-right: in the party, a users icon that turns into "remove" on hover; otherwise "add", on card hover. -->
                <button
                  class="icon-btn party"
                  class:in={character.onHome}
                  aria-label={character.onHome ? t.removeFromHome : t.addToHome}
                  aria-disabled={busy}
                  onclick={() => !busy && toggleHome(character)}
                  onpointerenter={() => (pointing = character.id)}
                  onpointerleave={() => (pointing = null)}
                  onfocus={(e) => e.currentTarget.matches(":focus-visible") && (pointing = character.id)}
                  onblur={() => (pointing = null)}
                >
                  <!-- Keyed on the icon. The new one fades in first, then the old one fades out: never both half-faded at
                       once, so the users figure they share stays solid and only the + or − sign fades. -->
                  {#key partyIcon(character)}
                    <i class="ti {partyIcon(character)}" aria-hidden="true" in:fade={{ duration: swapMs() }} out:fade={{ delay: swapMs(), duration: swapMs() }}></i>
                  {/key}
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
    margin: 0 0 var(--space-3);
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

  .card {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-3);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .avatar {
    flex: none;
    width: var(--avatar-size);
    height: var(--avatar-size);
    display: grid;
    place-items: center;
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
    color: var(--color-text-muted);
    font-size: calc(var(--avatar-size) / 2);
    object-fit: cover;
  }

  .info {
    flex: 1;
    min-width: 0;
    font-size: var(--text-sm);
  }

  .info p {
    margin: var(--space-1) 0 0;
    overflow-wrap: anywhere;
  }

  h2 {
    margin: 0;
    font-size: var(--text-md);
  }

  /* The menu shows on hover, while focus is in the card (so the keyboard still finds it), and while it's open. */
  .card :global(.menu-root > .icon-btn),
  .party:not(.in) {
    opacity: 0;
  }

  .card:hover :global(.menu-root > .icon-btn),
  .card:focus-within :global(.menu-root > .icon-btn),
  .card:hover .party,
  .card:focus-within .party,
  .card :global(.menu-root > .icon-btn[aria-expanded="true"]) {
    opacity: 1;
  }

  /* Bottom-right corner of the card. */
  .party {
    position: absolute;
    right: var(--space-2);
    bottom: var(--space-2);
    display: inline-grid;
    place-items: center;
  }

  .party.in {
    color: var(--color-accent);
  }

  /* While one icon fades into the next, both sit in the same spot. */
  .party .ti {
    grid-area: 1 / 1;
  }

  /* The menu and the party button ease in and out on card hover; Settings → Animations can turn it off.
     (The icon swap fades through Svelte's fade, which follows the same setting.) */
  .card :global(.menu-root > .icon-btn),
  .party {
    transition:
      opacity 150ms ease,
      color 150ms ease;
  }

  :global([data-motion="reduce"]) .card :global(.menu-root > .icon-btn),
  :global([data-motion="reduce"]) .party {
    transition: none;
  }

  .name {
    padding: 0;
    border: none;
    background: none;
    color: var(--color-text);
    font: inherit;
    font-weight: var(--weight-semibold);
    text-align: left;
    overflow-wrap: anywhere;
    cursor: pointer;
  }

  .name:hover {
    color: var(--color-link);
  }

  .muted {
    color: var(--color-text-muted);
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
