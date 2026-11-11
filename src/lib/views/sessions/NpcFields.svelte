<script lang="ts">
  // An NPC's own fields, in its editor, in the scene editor's tabs (SideTabs), as a location's (LocationFields):
  // Profile (race, gender, age, job and alignment in a table, the portrait beside it, the stats (AC, HP, speed, CR and
  // ability scores, copied from a catalog creature or typed) and the summary), Motivation and
  // traits (the two share the panel's height) and Backstory. The long ones are rich text, as in What happens. The
  // portrait is stored as soon as it's chosen; the rest saves with the world. The open tab is kept when another NPC opens.
  import { choosePictureFile, pictureSrc } from "$lib/api/campaigns";
  import { catalogRows, type CatalogRow } from "$lib/api/catalog";
  import { rollDice } from "$lib/api/dice";
  import { errorMessage } from "$lib/api/storage";
  import { setNpcPortrait, type Npc, type NpcStats } from "$lib/api/world";
  import RichText from "$lib/components/RichText.svelte";
  import SideTabs from "$lib/components/SideTabs.svelte";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import { strings } from "$lib/strings";
  import { UndoHistory } from "./sessions";
  import { crText } from "$lib/views/library/catalog";
  import { hitPoints, modifier, signed, speed } from "$lib/views/library/entries/format";

  let { npc = $bindable() }: { npc: Npc } = $props();

  const t = strings.sessions.npcFields;

  const TABS = [
    { id: "profile", icon: "ti-id", label: t.profile },
    { id: "personality", icon: "ti-mood-smile", label: t.personality },
    { id: "backstory", icon: "ti-book", label: t.backstory },
  ] as const;
  let current = $state<string>("profile");

  const SPECS = [
    ["race", t.race],
    ["gender", t.gender],
    ["age", t.age],
    ["job", t.job],
    ["alignment", t.alignment],
  ] as const;
  const TEXTS = {
    profile: [["summary", t.summary]],
    personality: [
      ["motivation", t.motivation],
      ["traits", t.traits],
    ],
    backstory: [["backstory", t.backstory]],
  } as const;

  // ---- Stats: typed, or copied from a creature in the catalog (Commoner unless another is picked) ----
  const e = strings.entry;
  const STATS = [
    ["ac", e.ac],
    ["hp", e.hp],
    ["speed", e.speed],
    ["cr", e.cr],
  ] as const;
  /** Each score and the creature field it comes from. */
  const SCORES = [
    ["str", "strength"],
    ["dex", "dexterity"],
    ["con", "constitution"],
    ["int", "intelligence"],
    ["wis", "wisdom"],
    ["cha", "charisma"],
  ] as const;

  let creatures = $state<CatalogRow[] | null>(null);
  let creaturesError = $state<string | null>(null);
  // Loaded each time an NPC editor opens, so a pack switched on since shows (as in the encounter builder).
  catalogRows("monsters", ["armor_class", "hit_points", "hit_dice", "speed", "challenge_rating", "strength", "dexterity", "constitution", "intelligence", "wisdom", "charisma"]).then(
    (rows) => (creatures = rows.toSorted((a, b) => a.name.localeCompare(b.name))),
    (err) => (creaturesError = t.creaturesError(errorMessage(err))),
  );
  let creature = $state(t.defaultCreature);

  // The search's suggestions: a list under the field, as the search bar's (arrow keys and Enter, or a click, pick one
  // into the field; Copy stats copies it; Escape closes the list). It floats above the page (a popover), as tall as the room under
  // the field allows, and scrolls inside; scrolling anything else closes it.
  const MAX_SHOWN = 50;
  const EDGE = 8;
  let listOpen = $state(false);
  let lookupField: HTMLInputElement | undefined = $state();
  let list: HTMLUListElement | undefined = $state();

  $effect(() => {
    if (!list || !lookupField) return;
    if (!showList) {
      if (list.matches(":popover-open")) list.hidePopover();
      return;
    }
    const at = lookupField.getBoundingClientRect();
    Object.assign(list.style, {
      left: `${at.left}px`,
      top: `${at.bottom + 4}px`,
      width: `${at.width}px`,
      maxHeight: `${Math.max(at.height * 3, innerHeight - at.bottom - 4 - EDGE)}px`,
    });
    if (!list.matches(":popover-open")) list.showPopover();
  });

  /** Keeps the highlighted one in view as the arrow keys move. */
  $effect(() => {
    if (active >= 0) list?.children[active]?.scrollIntoView({ block: "nearest" });
  });
  let active = $state(-1);
  const matches = $derived.by(() => {
    const q = creature.trim().toLowerCase();
    return q && creatures ? creatures.filter((r) => r.name.toLowerCase().includes(q)).slice(0, MAX_SHOWN) : [];
  });
  const showList = $derived(listOpen && matches.length > 0);

  function pick(row: CatalogRow) {
    creature = row.name;
    listOpen = false;
  }

  // ---- The dice: each score rolled the usual way, and the HP rolled from its hit dice when it shows them ("4 (1d8)") ----
  const SCORE_ROLL = "4d6kh3";
  let rolling = $state(false);

  async function rollStats() {
    if (rolling) return;
    rolling = true;
    const was = $state.snapshot(npc.stats);
    try {
      const scores = await Promise.all(SCORES.map(async ([key]) => [key, String((await rollDice(SCORE_ROLL)).total)] as const));
      const dice = /\((\d+d\d+(?:\s*[+-]\s*\d+)?)\)/.exec(npc.stats.hp)?.[1].replace(/\s+/g, "");
      const hp = dice ? `${(await rollDice(dice)).total} (${dice})` : npc.stats.hp;
      undo.push(was);
      npc.stats = { ...npc.stats, hp, ...Object.fromEntries(scores) };
      notify(t.rolled);
    } catch (err) {
      notify(t.rollError(errorMessage(err)));
    } finally {
      rolling = false;
    }
  }

  function onSearchKey(ev: KeyboardEvent) {
    if (ev.key === "ArrowDown" || ev.key === "ArrowUp") {
      if (!matches.length) return;
      ev.preventDefault();
      listOpen = true;
      active = Math.max(-1, Math.min(matches.length - 1, active + (ev.key === "ArrowDown" ? 1 : -1)));
    } else if (ev.key === "Enter" && listOpen && active >= 0) {
      ev.preventDefault();
      pick(matches[active]);
    } else if (ev.key === "Escape" && listOpen) {
      ev.preventDefault();
      ev.stopPropagation();
      listOpen = false;
    }
  }

  function copyStats() {
    listOpen = false;
    const name = creature.trim().toLowerCase();
    const row = creatures?.find((r) => r.name.toLowerCase() === name);
    if (!row) return notify(t.noCreature(creature.trim()));
    const m = row.fields;
    const score = (v: unknown) => (typeof v === "number" ? String(v) : "");
    undo.push($state.snapshot(npc.stats));
    npc.stats = {
      ac: String(m.armor_class?.[0]?.value ?? ""),
      hp: hitPoints(m),
      speed: speed(m.speed),
      cr: typeof m.challenge_rating === "number" ? crText(m.challenge_rating) : "",
      ...(Object.fromEntries(SCORES.map(([key, field]) => [key, score(m[field])])) as Record<(typeof SCORES)[number][0], string>),
    };
    notify(t.copiedFrom(row.name));
  }

  // ---- Undo (Ctrl+Z, Cmd+Z on macOS) in the stats: Copy stats, Roll stats and each finished edit of a stat (when its
  // field is left or Enter pressed) can be taken back, latest first, for the NPC open. While a stat's field has
  // unfinished typing, the field's own undo takes it back first. ----
  const undo = new UndoHistory<NpcStats>();
  /** The stats, and the stat's text, when a stat's field got focus. */
  let focused: { key: keyof NpcStats; text: string; stats: NpcStats } | null = null;

  $effect(() => {
    void npc.id; // another NPC: its own history
    undo.clear();
    focused = null;
  });

  function statFocused(key: keyof NpcStats) {
    focused = { key, text: npc.stats[key], stats: $state.snapshot(npc.stats) };
  }

  /** A stat's edit is finished: it can be taken back, and the next one starts from here. */
  function statChanged(key: keyof NpcStats) {
    if (!focused) return;
    undo.push(focused.stats);
    statFocused(key);
  }

  const isMac = navigator.userAgent.includes("Mac");

  function onStatsKey(ev: KeyboardEvent) {
    if (ev.key.toLowerCase() !== "z" || ev.shiftKey || ev.altKey || !(isMac ? ev.metaKey : ev.ctrlKey)) return;
    const field = ev.target instanceof HTMLInputElement ? ev.target : null;
    if (field && !field.classList.contains("value")) return; // the creature search: its own undo
    if (field && focused && field.value !== focused.text) return; // unfinished typing: the field's own undo
    const last = undo.pop();
    if (!last) return;
    ev.preventDefault();
    npc.stats = last;
    if (focused) focused = { key: focused.key, text: last[focused.key], stats: $state.snapshot(last) };
  }

  /** "+2" under a score that's a number. */
  const mod = (score: string) => (/^\d+$/.test(score.trim()) ? signed(modifier(Number(score))) : "");

  const campaign = campaigns.current!.id;
  let busy = $state(false);
  let portraitError = $state<string | null>(null);

  /** Chooses a picture (`remove`: none) and stores it at once. */
  async function setPortrait(remove = false) {
    if (busy) return;
    [busy, portraitError] = [true, null];
    try {
      const path = remove ? null : await choosePictureFile();
      if (!remove && typeof path !== "string") return;
      npc.portrait = await setNpcPortrait(campaign, npc.id, path);
    } catch (e) {
      portraitError = t.portraitError(errorMessage(e));
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onscrollcapture={(ev) => listOpen && !list?.contains(ev.target as Node) && (listOpen = false)} onresize={() => (listOpen = false)} />

<div class="fields">
  <SideTabs tabs={TABS} bind:current id="npc" label={t.parts}>
    {#if current === "profile"}
      <div class="profile">
        <table class="specs" aria-label={t.details}>
          <tbody>
            {#each SPECS as [key, label] (key)}
              <tr>
                <th scope="row"><label for="npc-{key}-{npc.id}" class="small-label">{label}</label></th>
                <td><input id="npc-{key}-{npc.id}" class="text-field" bind:value={npc[key]} maxlength="200" autocomplete="off" /></td>
              </tr>
            {/each}
          </tbody>
        </table>
        <div class="portrait">
          {#if npc.portrait}
            <img src={pictureSrc(npc.portrait)} alt={t.portrait} />
            <div class="portrait-actions">
              <button class="btn" disabled={busy} onclick={() => setPortrait()}>
                <i class="ti ti-photo-edit" aria-hidden="true"></i>{t.changePortrait}
              </button>
              <button class="btn icon" disabled={busy} aria-label={t.removePortrait} onclick={() => setPortrait(true)}>
                <i class="ti ti-trash" aria-hidden="true"></i>
              </button>
            </div>
          {:else}
            <button class="add" disabled={busy} onclick={() => setPortrait()}>
              <i class="ti ti-photo-plus" aria-hidden="true"></i>{t.addPortrait}
            </button>
          {/if}
        </div>
      </div>
      {#if portraitError}<p class="error" role="alert">{portraitError}</p>{/if}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions: only listens for Ctrl+Z from the fields and buttons in it -->
      <section class="stats" aria-labelledby="npc-stats-{npc.id}" onkeydown={onStatsKey}>
        <div class="stats-head">
          <h3 id="npc-stats-{npc.id}" class="small-label">{t.stats}</h3>
          <form
            class="copy"
            onsubmit={(ev) => {
              ev.preventDefault();
              copyStats();
            }}
          >
            <div class="lookup" onfocusout={(ev) => !ev.currentTarget.contains(ev.relatedTarget as Node | null) && (listOpen = false)}>
              <i class="ti ti-search" aria-hidden="true"></i>
              <input
                class="text-field"
                type="search"
                role="combobox"
                aria-label={t.copyFrom}
                aria-autocomplete="list"
                aria-controls="npc-creatures-{npc.id}"
                aria-expanded={showList}
                aria-activedescendant={listOpen && active >= 0 ? `npc-creature-${npc.id}-${active}` : undefined}
                bind:this={lookupField}
                bind:value={creature}
                autocomplete="off"
                spellcheck="false"
                oninput={() => ([listOpen, active] = [true, -1])}
                onkeydown={onSearchKey}
              />
              <ul bind:this={list} id="npc-creatures-{npc.id}" class="popover suggestions" role="listbox" aria-label={t.copyFrom} popover="manual" onmousedown={(ev) => ev.preventDefault()}>
                {#each matches as r, i (r.pack + r.index)}
                  <!-- Focus stays in the field (aria-activedescendant; the list keeps it on a press, its scrollbar too); a click picks it. -->
                  <!-- svelte-ignore a11y_click_events_have_key_events: the field's arrow keys and Enter drive the options -->
                  <li
                    id="npc-creature-{npc.id}-{i}"
                    role="option"
                    aria-selected={i === active}
                    onclick={() => pick(r)}
                    onmousemove={() => (active = i)}
                  >
                    <span>{r.name}</span>
                    <span class="cr">{e.cr} {crText(r.fields.challenge_rating)}</span>
                  </li>
                {/each}
              </ul>
            </div>
            <button class="btn" type="submit" disabled={!creatures || !creature.trim()}>
              <i class="ti ti-copy" aria-hidden="true"></i>{t.copy}
            </button>
            <button class="btn icon" type="button" aria-label={t.roll} disabled={rolling} onclick={rollStats}>
              <i class="ti ti-dice-5" aria-hidden="true"></i>
            </button>
          </form>
        </div>
        {#if creaturesError}<p class="error" role="alert">{creaturesError}</p>{/if}
        <!-- A stat block's box: AC, HP, Speed and CR in a line, then the scores, each with its modifier. -->
        <div class="statbox">
          <div class="top">
            {#each STATS as [key, label] (key)}
              <div class="pair">
                <label for="npc-{key}-{npc.id}">{label}</label>
                <input id="npc-{key}-{npc.id}" class="value" bind:value={npc.stats[key]} maxlength="200" autocomplete="off" onfocus={() => statFocused(key)} onchange={() => statChanged(key)} />
              </div>
            {/each}
          </div>
          <div class="scores">
            {#each SCORES as [key, field] (key)}
              <div class="score">
                <label for="npc-{key}-{npc.id}">{e.abilities[field]}</label>
                <input id="npc-{key}-{npc.id}" class="value" bind:value={npc.stats[key]} maxlength="200" autocomplete="off" onfocus={() => statFocused(key)} onchange={() => statChanged(key)} />
                <span class="mod" aria-hidden="true">{mod(npc.stats[key])}</span>
              </div>
            {/each}
          </div>
        </div>
      </section>
    {/if}
    <!-- Keyed: a rich text field fills itself once, so another NPC or tab needs fresh ones. -->
    {#key `${npc.id}-${current}`}
      {#each TEXTS[current as keyof typeof TEXTS] as [key, label] (key)}
        <section class="part text">
          <h3 class="small-label">{label}</h3>
          <RichText {label} extended bind:value={npc[key]} />
        </section>
      {/each}
    {/key}
  </SideTabs>
</div>

<style>
  .fields {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  /* Closer together than the panel's usual spacing: the profile holds a lot. */
  .fields :global(.panel) {
    gap: var(--space-3);
  }

  /* The details table on one half, the portrait on the other. */
  .profile {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-4);
  }

  .specs {
    width: 100%;
    border-collapse: collapse;
  }

  .specs tr + tr {
    border-top: var(--border-width) solid var(--color-border);
  }

  .specs th {
    width: 1%;
    padding: var(--space-1) var(--space-3) var(--space-1) 0;
    text-align: left;
    white-space: nowrap;
  }

  .specs td {
    padding: var(--space-1) 0;
  }

  .specs input {
    box-sizing: border-box;
    width: 100%;
  }

  /* As tall as the table: the picture fits inside, whole. */
  .portrait {
    position: relative;
    display: flex;
    min-height: var(--location-thumb-size);
    overflow: hidden;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
  }

  .portrait img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .portrait-actions {
    position: absolute;
    right: var(--space-2);
    bottom: var(--space-2);
    display: flex;
    gap: var(--space-1);
  }

  .btn.icon {
    justify-content: center;
    width: var(--control-height);
    padding: 0;
  }

  .add {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
    border: var(--border-width) dashed var(--color-border-strong);
    border-radius: inherit;
    background: transparent;
    color: var(--color-text-muted);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    cursor: pointer;
  }

  .add .ti {
    font-size: var(--text-xl);
  }

  .stats {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .stats-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
  }

  .copy {
    display: flex;
    gap: var(--space-2);
    margin-left: auto;
  }

  /* The creature search: an icon in the field, its suggestions in a list under it. */
  .lookup {
    position: relative;
  }

  .lookup .ti-search {
    position: absolute;
    left: var(--space-3);
    top: 50%;
    transform: translateY(-50%);
    color: var(--color-text-muted);
    pointer-events: none;
  }

  .lookup input {
    padding-left: calc(var(--space-3) + var(--space-5));
  }

  /* In the top layer, placed under the field by script. */
  .suggestions {
    position: fixed;
    inset: auto;
    box-sizing: border-box;
    margin: 0;
    overflow-y: auto;
    list-style: none;
  }

  .suggestions li {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .suggestions li[aria-selected="true"] {
    background: var(--color-accent-soft);
  }

  .cr {
    flex: none;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .statbox {
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  /* Each label, then its value; a divider between each stat, here and in the scores. */
  .top {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    padding: var(--space-2) 0;
    border-bottom: var(--border-width) solid var(--color-accent);
  }

  .pair {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    padding: 0 var(--space-3);
  }

  .pair + .pair,
  .score + .score {
    border-left: var(--border-width) solid var(--color-border);
  }

  .top label {
    font-size: var(--text-lg);
  }

  .top label,
  .score label {
    font-weight: var(--weight-bold);
  }

  .scores {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    padding: var(--space-2) 0;
  }

  .score {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
    padding: 0 var(--space-2);
    font-size: var(--text-sm);
  }

  /* The value reads as text, between two short lines under the label. */
  .score .value {
    max-width: 4em;
    font-size: var(--text-lg);
    border-top: var(--border-width) solid var(--color-accent);
    border-bottom: var(--border-width) solid var(--color-accent);
  }

  /* A field that looks like the text it holds. */
  .value {
    box-sizing: border-box;
    width: 100%;
    min-width: 0;
    min-height: var(--control-height-sm);
    padding: 0 var(--space-1);
    border: 0;
    border-radius: 0;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    text-align: center;
  }

  .value:hover {
    background: var(--color-surface-sunken);
  }

  .mod {
    min-height: 1lh;
    color: var(--color-text-muted);
    font-size: var(--text-xs);
  }

  .error {
    margin: 0;
    color: var(--color-danger);
    font-size: var(--text-sm);
  }

  .part {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }

  /* The rich text fields share the panel's height, each scrolling inside only when its text is longer. */
  .part.text {
    flex: 1 1 0;
    min-height: 0;
  }

  .part.text :global(.rich),
  .part.text :global(.host),
  .part.text :global(.host .editor) {
    min-height: 0;
  }

  .part.text :global(.host) {
    overflow-y: auto;
  }

  .small-label {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
  }

  @media (max-width: 600px) {
    .profile {
      grid-template-columns: minmax(0, 1fr);
    }

  }
</style>
