<script lang="ts">
  // Encounters → Prepare: the monster browser (left), the party and roster (middle), the difficulty meter (right).
  // Leaving with unsaved changes asks Save / Don't save / Cancel.
  import { beforeNavigate, goto } from "$app/navigation";
  import { flip } from "svelte/animate";
  import { fade, scale } from "svelte/transition";
  import { flushSync, untrack } from "svelte";
  import { reduceMotion } from "$lib/stores/theme.svelte";
  import BackButton from "$lib/components/BackButton.svelte";
  import { catalogEntry, catalogRows, type CatalogEntry, type CatalogRow } from "$lib/api/catalog";
  import { entryImage } from "$lib/api/images";
  import { listCharacters, type Character } from "$lib/api/characters";
  import { encounterBudgets, getEncounter, saveEncounter, type Budget, type Encounter, type RosterEntry } from "$lib/api/encounters";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { gotoReplacing } from "$lib/stores/history.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import { strings } from "$lib/strings";
  import "$lib/styles/prose.css";
  import { crText, title } from "$lib/views/library/catalog";
  import MonsterBlock from "$lib/views/library/entries/MonsterBlock.svelte";
  import DifficultyMeter from "./DifficultyMeter.svelte";
  import PartyPicker from "./PartyPicker.svelte";
  import { difficulty, monsterXp } from "./difficulty";
  import { addToRoster, characterLevel, entryKey, rowsByKey } from "./roster";

  /** The saved encounter to open, or null for a new one. */
  let { id }: { id: string | null } = $props();

  const t = strings.encounters;
  const campaign = campaigns.current!.id;
  const base = `/campaign/${campaign}/encounters`;

  // ---- Loading -----------------------------------------------------------------------------------
  let loadError = $state<string | null>(null);
  let encounter = $state<Encounter | null>(null);
  let characters = $state<Character[] | null>(null);
  let partyError = $state<string | null>(null);
  let monsters = $state<CatalogRow[] | null>(null);
  let monsterError = $state<string | null>(null);
  let budgets = $state<Budget[]>([]);
  let budgetError = $state<string | null>(null);

  let name = $state("");
  let roster = $state<RosterEntry[]>([]);
  let selected = $state(new Set<string>());
  /** What was last saved (or loaded), to tell whether there are changes. Null until everything has loaded. */
  let baseline = $state<string | null>(null);

  const snapshot = () => JSON.stringify({ name: name.trim(), roster, party: [...selected].sort() });

  // svelte-ignore state_referenced_locally
  const opening = id ? getEncounter(campaign, id) : Promise.resolve(null);
  const loadingCharacters = listCharacters(campaign).then((l) => l.characters);
  opening.then(
    (e) => {
      encounter = e;
      name = e?.name ?? t.newTitle;
      roster = e?.monsters ?? [];
    },
    (e) => (loadError = t.openError(errorMessage(e))),
  );
  loadingCharacters.then(
    (list) => (characters = list),
    (e) => (partyError = t.party.loadError(errorMessage(e))),
  );
  // A new encounter starts with the characters in the party ticked; a saved one with its own party.
  Promise.allSettled([opening, loadingCharacters]).then(([opened, chars]) => {
    if (opened.status === "rejected") return;
    const party = opened.value?.party ?? (chars.status === "fulfilled" ? chars.value.filter((c) => c.onHome).map((c) => c.id) : []);
    selected = new Set(party);
    baseline = snapshot();
  });
  catalogRows("monsters", ["challenge_rating", "type", "xp", "image"]).then(
    (rows) => (monsters = rows),
    (e) => (monsterError = t.browser.loadError(errorMessage(e))),
  );
  encounterBudgets().then(
    (b) => (budgets = b),
    (e) => (budgetError = t.meter.budgetError(errorMessage(e))),
  );

  // ---- Saving and leaving ------------------------------------------------------------------------
  let saving = $state(false);
  let saveError = $state<string | null>(null);
  const changed = $derived(baseline !== null && snapshot() !== baseline);
  let leaving = false;

  // Ready locks the encounter: nothing in it can change until the pencil puts it back to Draft.
  const locked = $derived(encounter?.status === "ready");

  /** Saves the changes, with `status` (Ready and back to Draft save at once). */
  async function save(status = encounter?.status ?? "draft"): Promise<boolean> {
    if (saving) return false;
    saving = true;
    saveError = null;
    const sent = snapshot();
    const switching = status !== (encounter?.status ?? "draft");
    try {
      const saved = await saveEncounter(campaign, {
        id: encounter?.id ?? "",
        name: name.trim() || t.newTitle,
        monsters: roster,
        party: [...selected],
        status,
        savedAt: 0,
      });
      encounter = saved;
      if (sent === snapshot()) {
        name = saved.name; // a blank name was saved as "New encounter"
        baseline = snapshot();
      } else baseline = sent; // typed on while it saved: those changes are still unsaved
      notify(switching ? (saved.status === "ready" ? t.nowReady : t.nowDraft)(saved.name) : t.saved(saved.name));
      // A new encounter gets its own address; nothing is unsaved, so leaving this one doesn't ask.
      if (!id) {
        leaving = true;
        await gotoReplacing(`${base}/${saved.id}`);
      }
      return true;
    } catch (e) {
      saveError = t.saveError(errorMessage(e));
      return false;
    } finally {
      saving = false;
    }
  }

  type LeaveAnswer = "save" | "discard" | "cancel";
  let leaveDialog: HTMLDialogElement | undefined = $state();
  let resolveLeave: ((answer: LeaveAnswer) => void) | null = null;

  function askLeave(): Promise<LeaveAnswer> {
    leaveDialog?.showModal();
    return new Promise((resolve) => (resolveLeave = resolve));
  }

  // Escape closes the dialog too, which counts as Cancel.
  function answerLeave(answer: LeaveAnswer) {
    const resolve = resolveLeave;
    resolveLeave = null;
    if (leaveDialog?.open) leaveDialog.close();
    resolve?.(answer);
  }

  // Hold the navigation (Back included), ask, then go on.
  beforeNavigate((navigation) => {
    if (leaving || !changed || !navigation.to || navigation.type === "leave") return;
    navigation.cancel();
    void askLeave().then(async (answer) => {
      if (answer === "cancel" || (answer === "save" && !(await save()))) return;
      leaving = true;
      if (navigation.type === "popstate") history.go(navigation.delta ?? -1);
      else void goto(navigation.to!.url);
    });
  });

  // ---- Party ------------------------------------------------------------------------------------
  const party = $derived((characters ?? []).filter((c) => selected.has(c.id)));
  const levels = $derived(party.map((c) => characterLevel(c.classes)));

  function toggleCharacter(characterId: string, on: boolean) {
    const next = new Set(selected);
    if (on) next.add(characterId);
    else next.delete(characterId);
    selected = next;
  }

  // ---- Monster browser --------------------------------------------------------------------------
  let search = $state("");
  let type = $state("");
  const CR_STEPS = [0, 1 / 8, 1 / 4, 1 / 2, ...Array.from({ length: 30 }, (_, i) => i + 1)];
  const CR_LAST = CR_STEPS.length - 1;
  let crMin = $state(0);
  let crMax = $state(CR_LAST);
  const crFiltered = $derived(crMin > 0 || crMax < CR_LAST);
  const anyFilter = $derived(crFiltered || type !== "");

  const crOf = (row: CatalogRow | undefined): number | null => (typeof row?.fields.challenge_rating === "number" ? row.fields.challenge_rating : null);
  const typeOf = (row: CatalogRow) => (typeof row.fields.type === "string" ? title(row.fields.type) : "");
  const types = $derived([...new Set((monsters ?? []).map(typeOf).filter(Boolean))].sort());

  type Sort = "name" | "cr" | "type";
  let sort = $state<Sort>("name");
  let descending = $state(false);
  function sortBy(column: Sort) {
    if (sort === column) descending = !descending;
    else [sort, descending] = [column, false];
  }
  const ariaSort = (column: Sort) => (sort === column ? (descending ? "descending" : "ascending") : undefined);

  const matching = $derived.by(() => {
    const words = search.toLowerCase().split(/\s+/).filter(Boolean);
    const rows = (monsters ?? []).filter((m) => {
      const cr = crOf(m);
      // A monster without a CR only shows while the range covers everything.
      const inRange = !crFiltered || (cr !== null && cr >= CR_STEPS[crMin] && cr <= CR_STEPS[crMax]);
      return words.every((w) => m.name.toLowerCase().includes(w)) && inRange && (!type || typeOf(m) === type);
    });
    const byName = (a: CatalogRow, b: CatalogRow) => a.name.localeCompare(b.name);
    const compare = (a: CatalogRow, b: CatalogRow) =>
      sort === "cr" ? (crOf(a) ?? -1) - (crOf(b) ?? -1) : sort === "type" ? typeOf(a).localeCompare(typeOf(b)) : 0;
    const sign = descending ? -1 : 1;
    return rows.sort((a, b) => sign * (compare(a, b) || byName(a, b)));
  });

  // 50 monsters a page; any change to the search, filters or sort starts again at page 1.
  const PAGE_SIZE = 50;
  let pageNumber = $state(1);
  const pages = $derived(Math.max(1, Math.ceil(matching.length / PAGE_SIZE)));
  const onPage = $derived(matching.slice((pageNumber - 1) * PAGE_SIZE, pageNumber * PAGE_SIZE));
  $effect(() => {
    void [search, type, crMin, crMax, sort, descending];
    pageNumber = 1;
  });
  let monstersScroll: HTMLElement | undefined = $state();
  function toPage(n: number) {
    pageNumber = Math.min(pages, Math.max(1, n));
    hidePreview();
    if (monstersScroll) monstersScroll.scrollTop = 0;
  }

  function clearFilters() {
    [type, crMin, crMax] = ["", 0, CR_LAST];
  }

  // Hovering or focusing a monster shows its stat block beside the browser.
  let preview = $state<{ row: CatalogRow; top: number; left: number } | null>(null);
  let previewEntry = $state<CatalogEntry | null>(null);
  let previewError = $state<string | null>(null);
  const loadedEntries = new Map<string, Promise<CatalogEntry>>();
  // The monster's picture when its pack's images are on (Settings → Rules); null offline or when off. Asked on every
  // hover (instant once kept), so turning images off or deleting them shows at once.
  let previewImage = $state<string | null>(null);
  let browserPanel: HTMLElement | undefined = $state();

  function showPreview(row: CatalogRow, target: HTMLElement) {
    const at = target.closest("tr")!.getBoundingClientRect();
    const panel = browserPanel!.getBoundingClientRect();
    preview = { row, top: at.top, left: panel.right + 8 };
    const key = entryKey(row);
    if (!loadedEntries.has(key)) loadedEntries.set(key, catalogEntry("monsters", row.pack, row.index));
    [previewEntry, previewError, previewImage] = [null, null, null];
    loadedEntries.get(key)!.then(
      (e) => {
        if (preview?.row !== row) return;
        previewEntry = e;
        if (typeof e.entry.image !== "string") return;
        // A picture that can't be kept is reported, but the stat block still shows.
        entryImage(row.pack, e.entry.image).then(
          (src) => preview?.row === row && (previewImage = src),
          (err) => notify(errorMessage(err)),
        );
      },
      (e) => {
        loadedEntries.delete(key); // try again next time
        if (preview?.row === row) previewError = errorMessage(e);
      },
    );
  }

  const hidePreview = () => (preview = null);

  // Kept inside the window once it's drawn.
  let card: HTMLElement | undefined = $state();
  $effect(() => {
    if (!card || !preview || !previewEntry) return;
    void previewImage; // the picture makes the card taller
    card.style.top = `${Math.max(8, Math.min(preview.top, innerHeight - card.offsetHeight - 8))}px`;
  });

  // ---- Roster and difficulty ----------------------------------------------------------------------
  const byKey = $derived(rowsByKey(monsters ?? []));

  // Clicking a tile opens its full stat block in a card under the tiles; again (or ✕) closes it.
  let detailsKey = $state<string | null>(null);
  const detailsRow = $derived(roster.find((r) => entryKey(r) === detailsKey) ?? null); // gone when removed
  // Only which monster is open matters to the stat block: + and − replace its roster entry but mustn't reload it.
  const detailsOpenKey = $derived(detailsRow ? entryKey(detailsRow) : null);
  let detailsEntry = $state<CatalogEntry | null>(null);
  let detailsError = $state<string | null>(null);
  $effect(() => {
    const key = detailsOpenKey;
    const row = untrack(() => detailsRow);
    [detailsEntry, detailsError] = [null, null];
    if (!key || !row) return;
    if (!loadedEntries.has(key)) loadedEntries.set(key, catalogEntry("monsters", row.pack, row.index));
    let stale = false;
    loadedEntries.get(key)!.then(
      (e) => !stale && (detailsEntry = e),
      (e) => {
        loadedEntries.delete(key); // try again next time
        if (!stale) detailsError = errorMessage(e);
      },
    );
    return () => (stale = true);
  });
  const toggleDetails = (row: RosterEntry) => (detailsKey = detailsKey === entryKey(row) ? null : entryKey(row));
  // Minimized: only the pictures, in one row, so an open stat block gets the rest of the column. The switch is a view
  // transition: both panels grow or shrink into their new size. (Tiles get no view-transition-name of their own: named
  // tiles are drawn above the page, unclipped, so scrolled-away ones would fly across the screen.)
  let minimized = $state(false);
  function toggleMinimized() {
    const apply = () => flushSync(() => (minimized = !minimized));
    if (reduceMotion() || !document.startViewTransition) apply();
    else document.startViewTransition(apply);
  }
  // Appearing and disappearing parts (stat block, popup, notes) fade.
  const fadeMotion = () => ({ duration: reduceMotion() ? 0 : 150 });

  // A small picture on each roster card, when its pack's images are on (Settings → Rules); asked once per monster.
  let pictures = $state<Record<string, string | null>>({});
  const askedPictures = new Set<string>();
  $effect(() => {
    for (const row of roster) {
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
  const rosterXp = $derived(roster.map((r) => monsterXp(byKey.get(entryKey(r))?.fields.xp)));
  const result = $derived(difficulty(levels, roster.map((r, i) => ({ quantity: r.quantity, xp: rosterXp[i] })), budgets));

  const cardMotion = () => ({ start: 0.85, duration: reduceMotion() ? 0 : 200 });

  function setQuantity(row: RosterEntry, quantity: number) {
    roster = roster.map((r) => (r === row ? { ...r, quantity } : r));
  }

</script>

<svelte:head><title>{name.trim() || t.newTitle}</title></svelte:head>

{#if loadError}
  <div class="status">
    <p class="error" role="alert">{loadError}</p>
    <BackButton fallback={base} />
  </div>
{:else if baseline === null}
  <p class="status muted" role="status">{t.loading}</p>
{:else}
  <div class="prepare">
    <header class="top">
      <BackButton fallback={base} />
      {#if locked}
        <h1 class="name-field name-text">{name}</h1>
      {:else}
        <label class="name-field">
          <span class="sr-only">{t.name}</span>
          <input bind:value={name} maxlength="100" autocomplete="off" placeholder={t.newTitle} />
        </label>
      {/if}
      {#if saveError}<span class="error small" role="alert">{saveError}</span>{/if}
      {#if locked}
        <button class="icon-btn icon-btn-lg" aria-label={t.toDraft} disabled={saving} onclick={() => save("draft")}>
          <i class="ti ti-pencil" aria-hidden="true"></i>
        </button>
        <!-- Run is TODO 10. -->
        <button class="btn btn-primary" disabled>
          {t.start}
          <i class="ti ti-player-play" aria-hidden="true"></i>
        </button>
      {:else}
        <button class="icon-btn icon-btn-lg" aria-label={saving ? t.saving : t.save} disabled={saving || !changed} onclick={() => save()}>
          <i class="ti ti-device-floppy" aria-hidden="true"></i>
        </button>
        <button class="btn btn-primary" disabled={saving} onclick={() => save("ready")}>
          {t.ready}
          <i class="ti ti-check" aria-hidden="true"></i>
        </button>
      {/if}
    </header>

    <div class="columns">
      <section class="panel browser" aria-label={t.browser.title} bind:this={browserPanel}>
        {#if monsterError}
          <p class="error" role="alert">{monsterError}</p>
        {:else if monsters === null}
          <p class="muted" role="status">{t.browser.loading}</p>
        {:else if monsters.length === 0}
          <p class="muted">{t.browser.noMonsters}</p>
        {:else}
          <label for="monster-search" class="sr-only">{t.browser.search}</label>
          <input id="monster-search" class="search" type="search" bind:value={search} placeholder={t.browser.search} autocomplete="off" />
          <div class="filters">
            <div class="cr-range">
              <span class="cr-text">{t.browser.crRange(crText(CR_STEPS[crMin]), crText(CR_STEPS[crMax]))}</span>
              <div class="cr-sliders" style:--from="{(100 * crMin) / CR_LAST}%" style:--to="{(100 * crMax) / CR_LAST}%">
                <input
                  type="range"
                  min="0"
                  max={CR_LAST}
                  bind:value={() => crMin, (v) => (crMin = Math.min(v, crMax))}
                  aria-label={t.browser.crMin}
                  aria-valuetext={crText(CR_STEPS[crMin])}
                />
                <input
                  type="range"
                  min="0"
                  max={CR_LAST}
                  bind:value={() => crMax, (v) => (crMax = Math.max(v, crMin))}
                  aria-label={t.browser.crMax}
                  aria-valuetext={crText(CR_STEPS[crMax])}
                />
              </div>
            </div>
            <select aria-label={t.browser.type} bind:value={type}>
              <option value="">{t.browser.allTypes}</option>
              {#each types as option (option)}<option value={option}>{option}</option>{/each}
            </select>
            {#if anyFilter}<button class="link" onclick={clearFilters}>{t.browser.clearFilters}</button>{/if}
          </div>
          {#if matching.length === 0}
            <p class="muted">{t.browser.noMatches}</p>
          {:else}
            <!-- svelte-ignore a11y_no_static_element_interactions: hover only adds the preview; focus does the same from the keyboard -->
            <div class="monsters-scroll" bind:this={monstersScroll} onmouseleave={hidePreview} onscroll={hidePreview}>
              <table class="monsters">
                <colgroup><col /><col class="col-cr" /><col class="col-type" /><col class="col-add" /></colgroup>
                <thead>
                  <tr>
                    {#each [["name", t.browser.name], ["cr", t.browser.cr], ["type", t.browser.type]] as [column, label] (column)}
                      <th scope="col" aria-sort={ariaSort(column as Sort)}>
                        <button class="sort" onclick={() => sortBy(column as Sort)} aria-label={t.browser.sortBy(label)}>
                          {label}<span class="arrow" aria-hidden="true">{sort === column ? (descending ? "↓" : "↑") : ""}</span>
                        </button>
                      </th>
                    {/each}
                    <th scope="col"><span class="sr-only">{t.browser.add}</span></th>
                  </tr>
                </thead>
                <tbody>
                  {#each onPage as m (entryKey(m))}
                    <tr onmouseenter={(e) => showPreview(m, e.currentTarget)}>
                      <td>{m.name}</td>
                      <td class="muted small">{crText(crOf(m))}</td>
                      <td class="muted small">{typeOf(m)}</td>
                      <td>
                        <button
                          class="icon-btn add"
                          disabled={locked}
                          aria-label={t.browser.addLabel(m.name, m.source)}
                          onfocus={(e) => showPreview(m, e.currentTarget)}
                          onblur={hidePreview}
                          onkeydown={(e) => e.key === "Escape" && hidePreview()}
                          onclick={() => (roster = addToRoster(roster, m))}
                        >
                          <i class="ti ti-plus" aria-hidden="true"></i>
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
            {#if pages > 1}
              <div class="paging">
                <button class="btn" aria-label={strings.catalog.previous} disabled={pageNumber === 1} onclick={() => toPage(pageNumber - 1)}>
                  <i class="ti ti-chevron-left" aria-hidden="true"></i>
                </button>
                <span role="status">{strings.catalog.range((pageNumber - 1) * PAGE_SIZE + 1, Math.min(pageNumber * PAGE_SIZE, matching.length), matching.length)}</span>
                <button class="btn" aria-label={strings.catalog.next} disabled={pageNumber === pages} onclick={() => toPage(pageNumber + 1)}>
                  <i class="ti ti-chevron-right" aria-hidden="true"></i>
                </button>
              </div>
            {/if}
          {/if}
        {/if}
      </section>

      <div class="rest">
        <div class="editor">
          <section class="panel monsters" class:minimized aria-label={t.roster.title}>
            {#if roster.length === 0}
              <p class="empty-pot" in:fade={fadeMotion()}>{t.roster.empty}</p>
            {:else}
              <button class="icon-btn minimize" transition:fade={fadeMotion()} aria-label={minimized ? t.roster.expand : t.roster.minimize} aria-expanded={!minimized} onclick={toggleMinimized}>
                <i class="ti {minimized ? 'ti-arrows-maximize' : 'ti-arrows-minimize'}" aria-hidden="true"></i>
              </button>
            {/if}
            <!-- Always there, so the first tile added and the last one removed animate too. -->
            <ul class="roster">
              {#each roster as row, i (entryKey(row))}
                {@const known = byKey.get(entryKey(row))}
                {@const picture = pictures[entryKey(row)]}
                {@const xp = rosterXp[i]}
                <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions: the name is the keyboard's button; the whole tile is clickable for the mouse -->
                <li
                  class="tile"
                  class:open={detailsKey === entryKey(row)}
                  transition:scale={cardMotion()}
                  animate:flip={{ duration: cardMotion().duration }}
                  onclick={(e) => !(e.target as Element).closest("button") && toggleDetails(row)}
                >
                  {#if !locked}
                    <button class="tile-remove" aria-label={t.roster.remove(row.name)} onclick={() => (roster = roster.filter((r) => r !== row))}>
                      <i class="ti ti-x" aria-hidden="true"></i>
                    </button>
                  {/if}
                  <div class="tile-portrait">
                    {#if picture}
                      <img class="hex" src={picture} alt="" />
                    {:else}
                      <span class="hex no-picture" aria-hidden="true">{row.name.charAt(0)}</span>
                    {/if}
                    <div class="count-row">
                      {#if !locked}
                        <button class="hex-step less" aria-label={t.roster.less(row.name)} disabled={row.quantity <= 1} onclick={() => setQuantity(row, row.quantity - 1)}>
                          <i class="ti ti-minus" aria-hidden="true"></i>
                        </button>
                      {/if}
                      <output class="badge" aria-live="polite" aria-label={t.roster.quantity(row.name)}>×{row.quantity}</output>
                      {#if !locked}
                        <button class="hex-step more" aria-label={t.roster.more(row.name)} disabled={row.quantity >= 999} onclick={() => setQuantity(row, row.quantity + 1)}>
                          <i class="ti ti-plus" aria-hidden="true"></i>
                        </button>
                      {/if}
                    </div>
                  </div>
                  <div class="tile-who">
                    <h3>
                      <button class="tile-name" aria-expanded={detailsKey === entryKey(row)} aria-controls="monster-details" onclick={() => toggleDetails(row)}>{row.name}</button>
                    </h3>
                    <p class="tile-meta">
                      {#if known}
                        {#if typeOf(known)}<span>{typeOf(known)}</span><span aria-hidden="true">·</span>{/if}
                        <span class="mono strong">{t.roster.cr(crText(crOf(known)))}</span>
                      {:else}
                        <span>{row.source}</span>
                      {/if}
                    </p>
                    <!-- Without XP the monster doesn't count towards difficulty, so that stays visible. -->
                    {#if xp === null}
                      <p class="tile-meta warn">{known ? t.roster.unknownXp : `${t.roster.unknownXp} · ${t.roster.packOff}`}</p>
                    {/if}
                  </div>
                </li>
              {/each}
            </ul>
          </section>
          {#if detailsRow}
            <section id="monster-details" class="panel details prose" transition:fade={fadeMotion()} aria-label={t.details.label(detailsRow.name)}>
              <button class="icon-btn details-close" aria-label={t.details.close} onclick={() => (detailsKey = null)}>
                <i class="ti ti-x" aria-hidden="true"></i>
              </button>
              <h3 class="details-name">{detailsRow.name}</h3>
              {#if detailsError}
                <p class="error">{detailsError}</p>
              {:else if !detailsEntry}
                <p class="muted">{t.browser.previewLoading}</p>
              {:else}
                <MonsterBlock loaded={detailsEntry} />
              {/if}
            </section>
          {/if}
        </div>

        <aside class="side">
          <DifficultyMeter {result} partySize={party.length} monsterCount={roster.length} hasBudgets={budgets.length > 0} {budgetError}>
            <PartyPicker {characters} {selected} error={partyError} ontoggle={toggleCharacter} disabled={locked} />
          </DifficultyMeter>
        </aside>
      </div>
    </div>
  </div>

  {#if preview}
    <div bind:this={card} class="preview prose" transition:fade={fadeMotion()} role="dialog" aria-label={t.browser.preview(preview.row.name)} style:top="{preview.top}px" style:left="{preview.left}px">
      {#if previewImage}
        <img class="portrait" src={previewImage} alt="" />
      {/if}
      <h3>{preview.row.name}</h3>
      {#if previewError}
        <p class="error">{previewError}</p>
      {:else if !previewEntry}
        <p class="muted">{t.browser.previewLoading}</p>
      {:else}
        <MonsterBlock loaded={previewEntry} brief />
      {/if}
    </div>
  {/if}
{/if}

<dialog bind:this={leaveDialog} class="leave" aria-labelledby="leave-title" aria-describedby="leave-question" onclose={() => answerLeave("cancel")}>
  <h2 id="leave-title">{t.leave.title}</h2>
  <p id="leave-question">{t.leave.question(name.trim() || t.newTitle)}</p>
  <div class="actions">
    <button class="btn" onclick={() => answerLeave("cancel")}>{t.leave.cancel}</button>
    <button class="btn" onclick={() => answerLeave("discard")}>{t.leave.discard}</button>
    <!-- svelte-ignore a11y_autofocus: the safe answer is the default -->
    <button class="btn btn-primary" autofocus onclick={() => answerLeave("save")}>{t.leave.save}</button>
  </div>
</dialog>

<style>
  .status {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-5);
  }

  /* Fills the screen: the browser takes the full height on the left, the rest scrolls on the right. */
  .prepare {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    height: 100%;
    padding: var(--space-4) var(--space-5) var(--space-5);
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .name-field {
    flex: 1;
    min-width: 0;
  }

  .name-field input,
  .name-text {
    width: 100%;
    min-height: var(--control-height);
    padding: 0 var(--space-2);
    border: var(--border-width) solid transparent;
    border-radius: var(--radius-md);
    background: none;
    font-family: var(--font-sans);
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
  }

  /* A Ready encounter's name: text in the input's place, so nothing moves when it locks. */
  .name-text {
    margin: 0;
    line-height: calc(var(--control-height) - 2 * var(--border-width));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name-field input:hover,
  .name-field input:focus {
    border-color: var(--color-border-strong);
    background: var(--color-surface);
  }

  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: var(--encounter-browser-width) minmax(0, 1fr);
    gap: var(--space-5);
  }

  /* Nothing goes below the fold: each column is the window's height, and what's too long scrolls inside it. */
  .rest {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-rows: minmax(0, 1fr); /* a fixed height, so the details card's 50% has something to be half of */
    gap: var(--space-5);
    min-height: 0;
  }

  .side {
    min-height: 0;
    overflow-y: auto;
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    min-width: 0;
    min-height: 0;
  }

  .panel.monsters {
    position: relative;
    flex: 0 1 auto;
    overflow-y: auto;
    view-transition-name: monsters-panel;
  }

  /* With a stat block open, the tiles keep room for two rows (more scroll) and the stat block takes the rest. */
  .editor:has(.details) .panel.monsters {
    flex: none;
    max-height: calc(2 * var(--roster-tile-height) + var(--space-4) + 2 * var(--space-6) + 2 * var(--border-width));
  }

  /* While minimizing, each panel's old and new picture stay pinned to its top and are cut off at its edges, so nothing
     shows outside the panel as it grows or shrinks. */
  :global(::view-transition-group(monsters-panel)),
  :global(::view-transition-group(details-panel)) {
    overflow: clip;
  }

  :global(::view-transition-old(monsters-panel)),
  :global(::view-transition-new(monsters-panel)),
  :global(::view-transition-old(details-panel)),
  :global(::view-transition-new(details-panel)) {
    height: 100%;
    object-fit: none;
    object-position: top;
  }

  .minimize {
    position: absolute;
    top: var(--space-1);
    right: var(--space-1);
    z-index: 1;
  }

  /* Minimized: one row of pictures (scrolling sideways when it's long); name, stats, + / − and ✕ are hidden but the
     name stays a button for the keyboard. */
  .panel.monsters.minimized {
    flex: none;
    padding: var(--space-3);
    padding-right: calc(var(--space-3) + var(--space-7)); /* the row stops before the minimize button */
  }

  .minimized .roster {
    display: flex;
    gap: var(--space-2);
    overflow-x: auto;
  }

  /* Six pictures fit across, whatever the width; a seventh scrolls. */
  .minimized .tile {
    flex: 0 0 calc((100% - 5 * var(--space-2)) / 6);
    min-width: 0;
    min-height: 0;
    padding: var(--space-1) var(--space-4) var(--space-2) var(--space-1);
    border-color: transparent;
    background: transparent;
  }

  .minimized .tile-portrait {
    width: 100%;
    max-width: var(--roster-portrait-width);
    height: auto;
    aspect-ratio: 22 / 19; /* the portrait's 88 × 76 */
  }

  /* The same proportions on the picture itself, since a percentage height doesn't resolve here. */
  .minimized .hex {
    height: auto;
    aspect-ratio: 22 / 19;
  }

  .minimized .tile.open {
    border-color: var(--color-accent);
  }

  .minimized .tile:focus-within {
    outline: var(--focus-ring) solid var(--color-focus);
  }

  .minimized .hex-step {
    display: none;
  }

  .minimized .tile-remove {
    top: 0;
    right: 0;
    width: var(--roster-mini-remove-size);
    height: var(--roster-mini-remove-size);
    border-radius: 50%;
    background: var(--color-surface);
  }

  .minimized .tile-remove .ti {
    font-size: var(--roster-step-icon);
  }

  .minimized .tile-who {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .panel.details {
    position: relative;
    view-transition-name: details-panel;
    flex: 1 1 0;
    min-height: 0;
    max-width: none;
    margin: 0;
    overflow-y: auto;
  }

  .details-name {
    margin: 0;
    padding-right: var(--space-7);
    font: var(--text-h2);
  }

  .details-close {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-height: 0;
    padding: var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  h2 {
    margin: 0;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  /* The monsters in the encounter: portrait tiles (docs/C · Portrait tiles). */
  .panel.monsters {
    gap: var(--space-6);
    padding: var(--space-6);
    border-radius: var(--radius-lg);
  }

  .empty-pot {
    padding: var(--space-10) var(--space-4);
    border: var(--border-width) dashed var(--color-border);
    border-radius: var(--radius-md);
    color: var(--color-text-muted);
    text-align: center;
  }

  .roster {
    display: grid;
    grid-template-columns: repeat(auto-fill, var(--roster-card-width));
    gap: var(--space-4);
  }

  .roster:empty {
    display: none;
  }

  .tile {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .tile-remove {
    position: absolute;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--roster-touch-size);
    height: var(--roster-touch-size);
    padding: 0;
    border: none;
    background: transparent;
    cursor: pointer;
    top: var(--space-1);
    right: var(--space-1);
    border-radius: var(--radius-md);
    color: var(--color-text-muted);
  }

  .tile-remove:hover {
    background: var(--color-surface-sunken);
    color: var(--color-danger);
  }

  .tile-remove:focus-visible {
    outline: var(--focus-ring) solid var(--color-focus);
    outline-offset: 2px;
  }

  /* A hexagon portrait with the count on a small hexagon at its lower right. */
  .tile-portrait {
    position: relative;
    width: var(--roster-portrait-width);
    height: var(--roster-portrait-height);
  }

  .hex {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    clip-path: polygon(25% 0, 75% 0, 100% 50%, 75% 100%, 25% 100%, 0 50%);
  }

  .no-picture {
    display: grid;
    place-items: center;
    background: var(--color-border);
    color: var(--color-text-muted);
    font: var(--text-h1);
  }

  /* The ×count on a hexagon at the portrait's lower right (the design's place), with a small + hexagon at its top
     right and a small − at its bottom right. */
  .count-row {
    position: absolute;
    right: calc(-1 * var(--space-3));
    bottom: calc(-1 * var(--space-1) - 2px);
    width: var(--roster-badge-width);
    height: var(--roster-badge-height);
  }

  .badge,
  .hex-step {
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    font: var(--text-dice);
    clip-path: polygon(25% 0, 75% 0, 100% 50%, 75% 100%, 25% 100%, 0 50%);
  }

  .badge {
    width: 100%;
    height: 100%;
    background: var(--potion-glow);
    color: var(--charcoal);
  }

  .hex-step {
    position: absolute;
    left: calc(100% - var(--space-1));
    width: var(--roster-step-width);
    height: var(--roster-step-height);
    background: var(--color-border);
    color: var(--color-text);
    cursor: pointer;
  }

  .hex-step .ti {
    font-size: var(--roster-step-icon);
  }

  /* One just above the middle, one just below it. */
  .hex-step.more {
    bottom: calc(50% + 1px);
  }

  .hex-step.less {
    top: calc(50% + 1px);
  }

  .hex-step:hover:not(:disabled) {
    background: var(--potion-glow);
    color: var(--charcoal);
  }

  .hex-step:disabled {
    color: var(--color-text-muted);
    cursor: not-allowed;
  }

  /* Inside, since the hexagon cuts off anything outside it. */
  .hex-step:focus-visible {
    outline: var(--focus-ring) solid var(--color-focus);
    outline-offset: calc(-1 * var(--focus-ring) - 2px);
  }

  .tile-who {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    text-align: center;
  }

  .tile {
    min-height: var(--roster-tile-height);
    cursor: pointer;
  }

  .tile.open {
    border-color: var(--color-accent);
    box-shadow: inset 0 0 0 var(--border-width) var(--color-accent);
  }

  .tile-who h3 {
    display: -webkit-box;
    margin: 0;
    overflow: hidden;
    font: var(--text-body);
    font-weight: var(--weight-bold);
    overflow-wrap: anywhere;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }

  .tile-name {
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    text-align: center;
    cursor: pointer;
  }

  .tile-name:focus-visible {
    outline: var(--focus-ring) solid var(--color-focus);
    outline-offset: 2px;
  }

  .tile-meta {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0 var(--space-2);
    font: var(--text-body-sm);
    color: var(--color-text-muted);
  }

  .mono {
    font: var(--text-dice);
  }

  .strong {
    color: var(--color-text);
  }

  .search,
  .filters select {
    min-height: var(--control-height);
    padding: 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .filters {
    display: flex;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    font-size: var(--text-sm);
  }

  .cr-range {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  /* Wide enough for the longest range ("CR 1/8–1/2"), so the slider doesn't jump as it changes. */
  .cr-text {
    min-width: 10ch;
    font-weight: var(--weight-semibold);
    font-variant-numeric: tabular-nums;
  }

  /* Two native sliders on one track: the inputs ignore the pointer, only their thumbs take it. */
  .cr-sliders {
    position: relative;
    flex: 1;
    height: var(--space-5);
  }

  .cr-sliders::before {
    content: "";
    position: absolute;
    inset: 50% 0 auto;
    height: var(--space-1);
    transform: translateY(-50%);
    border-radius: var(--radius-sm);
    background: linear-gradient(to right, var(--color-border) var(--from), var(--color-link) var(--from) var(--to), var(--color-border) var(--to));
  }

  .cr-sliders input {
    position: absolute;
    inset: 0;
    width: 100%;
    margin: 0;
    background: none;
    appearance: none;
    pointer-events: none;
  }

  .cr-sliders input::-webkit-slider-runnable-track {
    background: none;
  }

  .cr-sliders input::-webkit-slider-thumb {
    appearance: none;
    width: var(--space-4);
    height: var(--space-4);
    border: 2px solid var(--color-surface);
    border-radius: 50%;
    background: var(--color-link);
    pointer-events: auto;
    cursor: pointer;
  }

  .cr-sliders input::-moz-range-thumb {
    width: var(--space-4);
    height: var(--space-4);
    border: 2px solid var(--color-surface);
    border-radius: 50%;
    background: var(--color-link);
    pointer-events: auto;
    cursor: pointer;
  }

  .cr-sliders input:focus-visible::-webkit-slider-thumb {
    outline: var(--focus-ring) solid var(--color-focus);
    outline-offset: 2px;
  }

  .paging {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    font-size: var(--text-sm);
  }

  .monsters-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  /* Fixed column widths, so they don't jump as the rows change; the name takes the rest. */
  .monsters {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
  }

  .col-cr {
    width: 15%;
  }

  .col-type {
    width: 28%;
  }

  .col-add {
    width: calc(var(--control-height-sm) + 2 * var(--space-2));
  }

  .monsters th {
    position: sticky;
    top: 0;
    z-index: 1;
    border-bottom: var(--border-width) solid var(--color-border-strong);
    background: var(--color-surface);
    text-align: left;
  }

  .monsters td {
    overflow: hidden;
    padding: var(--space-1) var(--space-2);
    border-top: var(--border-width) solid var(--color-border);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .monsters tbody tr:hover {
    background: var(--color-surface-sunken);
  }

  .monsters td:last-child {
    text-align: right;
  }

  .sort {
    display: inline-flex;
    gap: var(--space-1);
    padding: var(--space-2);
    border: none;
    background: none;
    color: var(--color-text-muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    cursor: pointer;
  }

  .arrow {
    min-width: 1ch;
    color: var(--color-link);
  }

  .add {
    background: var(--color-success-fill);
    color: var(--color-accent-text);
  }

  .add:hover {
    background: var(--color-success-fill-hover);
    color: var(--color-accent-text);
  }

  /* The stat block beside the browser; it only shows, so the pointer goes through it. */
  .preview {
    position: fixed;
    z-index: var(--z-menu);
    width: min(var(--preview-width), calc(100vw - 2 * var(--space-4)));
    max-height: calc(100vh - 2 * var(--space-4));
    overflow: hidden;
    padding: var(--space-3) var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-drawer);
    font-size: var(--text-sm);
    pointer-events: none;
  }

  /* Fixed size, so the card's height is known before the picture has loaded. */
  .portrait {
    float: right;
    width: var(--monster-portrait-size);
    height: var(--monster-portrait-size);
    margin: 0 0 var(--space-2) var(--space-3);
    object-fit: contain;
  }

  .preview h3 {
    margin: 0 0 var(--space-1);
    font-size: var(--text-md);
  }

  .leave {
    width: min(100% - 2 * var(--space-4), var(--dialog-width));
    padding: var(--space-5);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    color: var(--color-text);
  }

  .leave[open] {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .leave::backdrop {
    background: var(--color-backdrop);
  }

  .leave h2 {
    font-size: var(--text-lg);
  }

  .leave p {
    margin: 0;
  }

  .leave .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--color-link);
    text-decoration: underline;
    cursor: pointer;
  }

  .muted {
    color: var(--color-text-muted);
  }

  .small {
    font-size: var(--text-sm);
  }

  .warn,
  .error {
    color: var(--color-danger);
  }

  @media (max-width: 900px) {
    .prepare {
      height: auto;
    }

    .columns,
    .rest {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: none;
    }

    .rest {
      overflow: visible;
    }

    .side,
    .panel.monsters,
    .details {
      overflow: visible;
    }

    .details {
      min-height: 0;
    }

    .monsters-scroll {
      flex: none;
      height: var(--encounter-monsters-height);
    }

    .side {
      position: static;
    }

    .preview {
      display: none;
    }
  }
</style>
