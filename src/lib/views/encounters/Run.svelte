<script lang="ts">
  // Encounters → Run (docs/Run encounter-html): roll initiative (round 0), then the turn bar, the initiative order,
  // the selected combatant (tracker and stat block), and the DM notes with the encounter log.
  // Every change is saved to the encounter's run file, so a crash loses nothing and reopening resumes the run.
  import { tick } from "svelte";
  import { catalogEntry, catalogRows, type CatalogEntry, type CatalogRow } from "$lib/api/catalog";
  import { listCharacters, saveCharacter, type Character } from "$lib/api/characters";
  import { addons, goLive } from "$lib/stores/addons.svelte";
  import { rollDice } from "$lib/api/dice";
  import { endRun, getEncounter, getRun, saveRun, type Combatant, type Encounter, type Run } from "$lib/api/encounters";
  import { errorMessage } from "$lib/api/storage";
  import { pictureSrc } from "$lib/api/campaigns";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { gotoReplacing } from "$lib/stores/history.svelte";
  import { showHeaderBack } from "$lib/stores/header.svelte";
  import { reduceMotion } from "$lib/stores/theme.svelte";
  import { strings } from "$lib/strings";
  import "$lib/styles/prose.css";
  import { classLine } from "$lib/views/characters/characters";
  import { crText } from "$lib/views/library/catalog";
  import { modifier, signed } from "$lib/views/library/entries/format";
  import AddCombatant, { type Pick } from "./AddCombatant.svelte";
  import NotesPanel from "./NotesPanel.svelte";
  import { monsterPicture } from "$lib/stores/monsterPictures";
  import MonsterBlock from "$lib/views/library/entries/MonsterBlock.svelte";
  import { encounterParty, entryKey, rowsByKey } from "./roster";
  import * as R from "./run";

  let { id }: { id: string } = $props();

  const t = strings.encounters.run;
  const campaign = campaigns.current!.id;
  // svelte-ignore state_referenced_locally
  const prepare = `/campaign/${campaign}/encounters/${id}`;
  showHeaderBack(() => prepare);
  const fmt = new Intl.NumberFormat();

  // ---- Loading -----------------------------------------------------------------------------------
  let loadError = $state<string | null>(null);
  let encounter = $state<Encounter | null>(null);
  let characters = $state<Character[]>([]);
  let run = $state<Run | null>(null);
  let monsters = $state<CatalogRow[]>([]);
  let conditions = $state<CatalogRow[]>([]);
  let lookupError = $state<string | null>(null);

  // A monster's whole entry, asked once per pack entry; rejects (and is asked again next time) when it can't load,
  // e.g. its pack is off.
  const entries = new Map<string, Promise<CatalogEntry>>();
  function entryOf(pack: string, index: string): Promise<CatalogEntry> {
    const key = entryKey({ pack, index });
    if (!entries.has(key)) entries.set(key, catalogEntry("monsters", pack, index).catch((e) => (entries.delete(key), Promise.reject(e))));
    return entries.get(key)!;
  }

  catalogRows("monsters", ["challenge_rating", "type", "image"]).then(
    (rows) => (monsters = rows),
    (e) => (lookupError = t.lookupError(errorMessage(e))),
  );
  catalogRows("conditions", []).then(
    (rows) => (conditions = rows.toSorted((a, b) => a.name.localeCompare(b.name))),
    (e) => (lookupError = t.lookupError(errorMessage(e))),
  );

  // svelte-ignore state_referenced_locally
  Promise.all([getEncounter(campaign, id), getRun(campaign, id), listCharacters(campaign)]).then(
    async ([e, saved, list]) => {
      encounter = e;
      characters = list.characters;
      // Saved after the run: its monsters or party may have changed since.
      const stale = !!saved && e.savedAt > saved.savedAt;
      if (saved && saved.round === 0 && stale && e.status === "ready") {
        // Still rolling initiative: start again from the encounter, keeping initiatives and whoever was added.
        return commit(R.carryOver(await newRun(e, list.characters), saved, e.monsters));
      }
      if (saved) {
        run = saved;
        const synced = R.followLinked(saved, list.characters);
        if (synced) commit(synced);
        changedSince = stale;
        // A fight already under way: continue it, or start it over.
        if (saved.round > 0) await tick().then(() => resumeDialog?.showModal());
        return;
      }
      if (e.status !== "ready") return void (loadError = t.notReady);
      commit(await newRun(e, list.characters));
    },
    (e) => (loadError = strings.encounters.openError(errorMessage(e))),
  );

  // ---- Building combatants -----------------------------------------------------------------------
  const blank = {
    pack: "",
    index: "",
    character: "",
    group: "",
    initiative: null,
    initiativeBonus: 0,
    tempHp: 0,
    concentrating: false,
    conditions: [],
    deathSuccesses: 0,
    deathFailures: 0,
  };

  /** A monster from its entry: its HP, AC and the DEX modifier as initiative bonus (when its pack is on). */
  async function monsterCombatant(pack: string, index: string, name: string, group: string): Promise<Combatant> {
    const m = (await entryOf(pack, index).catch(() => null))?.entry;
    const hp = typeof m?.hit_points === "number" ? m.hit_points : 0;
    const ac = m?.armor_class?.[0]?.value;
    return {
      ...blank,
      id: crypto.randomUUID(),
      kind: "monster",
      pack,
      index,
      group,
      name,
      initiativeBonus: typeof m?.dexterity === "number" ? modifier(m.dexterity) : 0,
      hp,
      maxHp: hp,
      ac: typeof ac === "number" ? ac : null,
    };
  }

  /** A character from its sheet: current, max and temp HP and AC; initiative is typed in. */
  const characterCombatant = (c: Character, initiative: number | null = null): Combatant => ({
    ...blank,
    id: crypto.randomUUID(),
    kind: "character",
    character: c.id,
    name: c.name,
    initiative,
    hp: Math.max(0, c.currentHp ?? c.maxHp ?? 0),
    maxHp: Math.max(0, c.maxHp ?? 0),
    tempHp: Math.max(0, c.tempHp ?? 0),
    deathSuccesses: c.deathSaves?.successes ?? 0,
    deathFailures: c.deathSaves?.failures ?? 0,
    ac: c.ac,
    initiativeBonus: c.initiative ?? 0, // breaks initiative ties, as a monster's DEX modifier does
  });

  async function roll(bonus: number): Promise<number | null> {
    try {
      return (await rollDice(R.initiativeNotation(bonus))).total;
    } catch (e) {
      saveError = t.rollError(errorMessage(e));
      return null;
    }
  }

  /** Round 0: the roster's monsters (one roll per entry) and the encounter's party. */
  async function newRun(e: Encounter, list: Character[]): Promise<Run> {
    const combatants: Combatant[] = [];
    for (const m of R.rosterMonsters(e.monsters)) combatants.push(await monsterCombatant(m.pack, m.index, m.name, m.group));
    // One roll per group.
    for (const group of new Set(combatants.map((c) => c.group))) {
      const members = combatants.filter((c) => c.group === group);
      const total = await roll(members[0].initiativeBonus);
      members.forEach((c) => (c.initiative = total));
    }
    for (const charId of encounterParty(e.partyMode, e.party, list)) {
      const c = list.find((x) => x.id === charId);
      if (c) combatants.push(characterCombatant(c));
    }
    return { round: 0, turn: 0, combatants, log: [], savedAt: 0 };
  }

  // ---- Saving ------------------------------------------------------------------------------------
  let saveError = $state<string | null>(null);
  let saving: Promise<unknown> = Promise.resolve();

  // A running encounter is live: the D&D Beyond addon syncs every 30 seconds while it's shown. When an addon saved a
  // character, linked PCs take its HP and death saves.
  $effect(goLive);
  $effect(() => {
    if (addons.charactersChanged === 0) return;
    listCharacters(campaign).then(
      (l) => {
        characters = l.characters;
        const synced = run && R.followLinked(run, l.characters);
        if (synced) commit(synced);
      },
      () => {}, // the fight goes on with the HP it has; the next sync tries again
    );
  });
  /** A character D&D Beyond keeps up to date: its HP and death saves are read-only here. */
  const linkedPc = (c: Combatant) => c.kind === "character" && !!characterById.get(c.character)?.linked;

  /** Shows the change at once and saves it after the saves before it. */
  function commit(next: Run) {
    run = next;
    const copy = $state.snapshot(next) as Run;
    saving = saving.then(() =>
      saveRun(campaign, id, copy).then(
        () => (saveError = null),
        (e) => (saveError = t.saveError(errorMessage(e))),
      ),
    );
  }

  // ---- Rolling initiative (round 0) --------------------------------------------------------------
  type Group = { key: string; members: Combatant[] };
  const groups = $derived.by(() => {
    const out: Group[] = [];
    for (const c of run?.combatants ?? []) {
      if (c.kind !== "monster") continue;
      const found = out.find((g) => g.key === c.group);
      if (found) found.members.push(c);
      else out.push({ key: c.group, members: [c] });
    }
    return out;
  });
  const pcs = $derived((run?.combatants ?? []).filter((c) => c.kind !== "monster"));
  /** Groups rolled separately: their members' initiatives differ, or the DM asked. */
  let separate = $state(new Set<string>());
  const isSeparate = (g: Group) => separate.has(g.key) || new Set(g.members.map((m) => m.initiative)).size > 1;
  const missing = $derived((run?.combatants ?? []).filter((c) => c.initiative === null).length);

  const setInitiative = (ids: string[], value: number | null) =>
    run && commit({ ...run, combatants: run.combatants.map((c) => (ids.includes(c.id) ? { ...c, initiative: value } : c)) });

  const typed = (event: Event) => {
    const n = (event.currentTarget as HTMLInputElement).valueAsNumber;
    return Number.isInteger(n) && Math.abs(n) <= R.MAX_NUMBER ? n : null;
  };

  /** "Goblin", or for a group "Goblin 1–3 ×3" (the numbers tell split groups of the same monster apart). */
  function groupName(g: Group): string {
    const first = g.members[0].name;
    if (g.members.length === 1) return first;
    const last = g.members[g.members.length - 1].name.match(/ (\d+)$/)?.[1];
    return `${last ? `${first}–${last}` : first} ×${fmt.format(g.members.length)}`;
  }

  async function rerollGroup(g: Group) {
    if (isSeparate(g)) for (const m of g.members) setInitiative([m.id], await roll(m.initiativeBonus));
    else setInitiative(g.members.map((m) => m.id), await roll(g.members[0].initiativeBonus));
  }

  async function toggleSeparate(g: Group) {
    const next = new Set(separate);
    if (isSeparate(g)) next.delete(g.key);
    else next.add(g.key);
    separate = next;
    if (!next.has(g.key)) setInitiative(g.members.map((m) => m.id), await roll(g.members[0].initiativeBonus));
    else for (const m of g.members) setInitiative([m.id], await roll(m.initiativeBonus));
  }

  async function rerollAll() {
    for (const g of groups) await rerollGroup(g);
  }

  function begin() {
    if (!run || missing > 0 || run.combatants.length === 0) return;
    const combatants = R.inOrder(run.combatants);
    commit({ ...run, round: 1, turn: 0, combatants });
    selectedId = combatants[0]?.id ?? null;
  }

  // ---- Running -----------------------------------------------------------------------------------
  let selectedId = $state<string | null>(null);
  const current = $derived(run && run.round > 0 ? (run.combatants[run.turn] ?? null) : null);
  const selected = $derived(run?.combatants.find((c) => c.id === selectedId) ?? current);

  function step(direction: 1 | -1) {
    if (!run || run.round === 0) return;
    const moved = R.stepTurn(run, direction);
    if (moved === run) return;
    commit(moved);
    selectedId = moved.combatants[moved.turn]?.id ?? null;
    // The order scrolls inside its column: keep whoever acts in view.
    tick().then(() => orderList?.querySelector(".row.active")?.scrollIntoView({ block: "nearest", behavior: reduceMotion() ? "auto" : "smooth" }));
  }

  let orderList: HTMLElement | undefined = $state();
  let mainPanel: HTMLElement | undefined = $state();
  // Another combatant's tracker and stat block start at the top.
  // (by id: the combatant object changes with every HP change, which mustn't jump the panel back up)
  const selectedKey = $derived(selected?.id ?? null);
  $effect(() => {
    void selectedKey;
    if (mainPanel) mainPanel.scrollTop = 0;
  });

  // Space: next turn, except where Space types or presses something.
  function onKey(event: KeyboardEvent) {
    if (event.key !== " " || event.repeat || event.ctrlKey || event.metaKey || event.altKey) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest("input, textarea, select, button, a, [contenteditable], dialog, [role='button']")) return;
    event.preventDefault();
    step(1);
  }

  const characterById = $derived(new Map(characters.map((c) => [c.id, c])));
  const rowByKey = $derived(rowsByKey(monsters));

  // Monster pictures, asked once per pack entry, when its pack's images are on; characters use their own picture.
  let pictures = $state<Record<string, string | null>>({});
  $effect(() => {
    for (const c of run?.combatants ?? []) {
      const key = entryKey(c);
      const image = rowByKey.get(key)?.fields.image;
      if (c.kind === "monster" && typeof image === "string") monsterPicture(c.pack, image).then((src) => (pictures[key] = src));
    }
  });

  function pictureOf(c: Combatant): string | null {
    if (c.kind === "monster") return pictures[entryKey(c)] ?? null;
    const picture = c.kind === "character" ? characterById.get(c.character)?.picture : undefined;
    return picture ? pictureSrc(picture) : null;
  }

  /** The line under a name in the order: "Fighter 5 · PC", "Undead · CR 5", or nothing. */
  function subLine(c: Combatant): string {
    if (c.kind === "character") {
      const line = classLine(characterById.get(c.character) ?? { classes: [] });
      return line ? `${line} · ${t.pc}` : t.pc;
    }
    if (c.kind === "monster") {
      const row = rowByKey.get(entryKey(c));
      const type = typeof row?.fields.type === "string" ? row.fields.type.charAt(0).toUpperCase() + row.fields.type.slice(1) : "";
      const cr = row?.fields.challenge_rating;
      return [type, typeof cr === "number" ? `CR ${crText(cr)}` : ""].filter(Boolean).join(" · ");
    }
    return "";
  }

  /** Under a character on the Roll initiative step: its class line and PC, then its initiative modifier (to roll with). */
  function setupLine(c: Combatant): string {
    const bonus = c.kind === "character" ? characterById.get(c.character)?.initiative : null;
    return [subLine(c), typeof bonus === "number" ? t.bonus(signed(bonus)) : ""].filter(Boolean).join(" · ");
  }

  /** The selected combatant's line: "Fighter 5 · Player character · AC 18 · Init 19", "Undead · CR 5 · Init 16". */
  function selectedLine(c: Combatant): string {
    const init = t.initLine(c.initiative === null ? "–" : String(c.initiative));
    if (c.kind === "character") {
      const line = classLine(characterById.get(c.character) ?? { classes: [] });
      return [line, t.playerCharacter, t.acLine(c.ac === null ? "–" : String(c.ac)), init].filter(Boolean).join(" · ");
    }
    if (c.kind === "monster") return [subLine(c), init].filter(Boolean).join(" · ");
    return [t.acLine(c.ac === null ? "–" : String(c.ac)), init].join(" · ");
  }

  const hpShare = (c: Combatant) => (c.maxHp > 0 ? Math.min(1, c.hp / c.maxHp) : 0);
  const hpLevel = (c: Combatant) => (hpShare(c) > 0.5 ? "high" : hpShare(c) > 0.25 ? "mid" : "low");
  const hpText = (c: Combatant) => (c.maxHp > 0 ? `${c.hp}/${c.maxHp}` : String(c.hp));

  // ---- The selected combatant --------------------------------------------------------------------
  let amount = $state<number | null>(null);

  const changeById = (cid: string, fn: (c: Combatant) => Combatant) => run && commit(R.update(run, cid, fn));

  function changeSelected(fn: (c: Combatant) => Combatant, log?: string) {
    if (!run || !selected) return;
    const changed = R.update(run, selected.id, fn);
    commit(log ? R.addLog(changed, log) : changed);
  }

  function applyHp(kind: "damage" | "heal" | "temp") {
    const n = amount;
    if (!selected || typeof n !== "number" || !Number.isInteger(n) || n < 0 || n > R.MAX_NUMBER) return;
    const c = selected;
    const shown = fmt.format(n);
    if (kind === "damage") {
      const after = R.damage(c, n);
      const end = R.isDown(after) && !R.isDown(c) ? ` ${c.kind === "character" ? t.logged.down : t.logged.defeated}` : "";
      changeSelected(() => after, t.logged.damage(c.name, shown) + end);
    } else if (kind === "heal") changeSelected((x) => R.heal(x, n), t.logged.heal(c.name, shown));
    else changeSelected((x) => R.giveTempHp(x, n), t.logged.temp(c.name, shown));
  }

  function toggleCondition(c: Combatant, condition: { index: string; name: string }) {
    if (!run) return;
    const had = c.conditions.some((x) => x.index === condition.index);
    const changed = R.update(run, c.id, (x) => R.toggleCondition(x, condition));
    commit(R.addLog(changed, (had ? t.logged.conditionOff : t.logged.conditionOn)(c.name, condition.name)));
  }

  function addCondition(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    const row = conditions.find((x) => x.index === select.value);
    select.value = "";
    if (row && selected) toggleCondition(selected, { index: row.index, name: row.name });
  }

  // The selected monster's stat block, and the conditions it's immune to.
  let statBlock = $state<{ key: string; entry: CatalogEntry | null; error: string | null } | null>(null);
  $effect(() => {
    const c = selected;
    if (!c || c.kind !== "monster") return void (statBlock = null);
    const key = entryKey(c);
    if (statBlock?.key === key) return;
    statBlock = { key, entry: null, error: null };
    entryOf(c.pack, c.index).then(
      (entry) => statBlock?.key === key && (statBlock = { key, entry, error: null }),
      (e) => statBlock?.key === key && (statBlock = { key, entry: null, error: t.statBlockError(errorMessage(e)) }),
    );
  });
  const immune = $derived(new Set<string>((statBlock?.entry?.entry.condition_immunities ?? []).map((x: { index: string }) => x.index)));

  // ---- DM notes and the log ----------------------------------------------------------------------
  let note = $state("");

  function addNote() {
    const text = note.trim();
    if (!run || !text) return;
    commit(R.addLog(run, text));
    note = "";
  }

  // ---- The map -----------------------------------------------------------------------------------
  let mapDialog: HTMLDialogElement | undefined = $state();
  let mapOpen = $state(false);

  function openMap() {
    mapOpen = true;
    mapDialog?.showModal();
  }

  // ---- + Combatant -------------------------------------------------------------------------------
  let adder: AddCombatant | undefined = $state();
  const absent = $derived(characters.filter((c) => !run?.combatants.some((x) => x.kind === "character" && x.character === c.id)));

  async function addPicked(pick: Pick) {
    if (!run) return;
    let c: Combatant;
    if (pick.kind === "monster") {
      const same = run.combatants.filter((x) => x.kind === "monster" && entryKey(x) === entryKey(pick.row)).length;
      c = await monsterCombatant(pick.row.pack, pick.row.index, same ? `${pick.row.name} ${same + 1}` : pick.row.name, crypto.randomUUID());
      c.initiative = pick.initiative ?? (await roll(c.initiativeBonus));
    } else if (pick.kind === "character") {
      // Its own group marks it as added (the party's characters have none), so a rebuilt run keeps it.
      c = { ...characterCombatant(pick.character, pick.initiative), group: crypto.randomUUID() };
    } else {
      c = { ...blank, id: crypto.randomUUID(), kind: "custom", group: crypto.randomUUID(), name: pick.name, initiative: pick.initiative, hp: pick.maxHp, maxHp: pick.maxHp, ac: pick.ac };
    }
    if (!run) return;
    commit(run.round === 0 ? { ...run, combatants: [...run.combatants, c] } : R.addCombatant(run, c));
  }

  // ---- Continue or start over ------------------------------------------------------------------
  let resumeDialog: HTMLDialogElement | undefined = $state();
  let restarting = $state(false);
  /** The encounter was saved after the run (monsters or party may differ): Start over picks the changes up. */
  let changedSince = $state(false);

  /** Throws the saved run away (no HP written back) and rolls initiative afresh. */
  async function restart() {
    if (!encounter || restarting) return;
    restarting = true;
    try {
      await saving;
      selectedId = null;
      separate = new Set();
      commit(await newRun(encounter, characters));
      changedSince = false;
      resumeDialog?.close();
    } finally {
      restarting = false;
    }
  }

  // ---- End encounter -----------------------------------------------------------------------------
  let endDialog: HTMLDialogElement | undefined = $state();
  let writeBack = $state(true);
  let ending = $state(false);
  let endError = $state<string | null>(null);
  // Linked characters aren't written back: D&D Beyond has their HP.
  const fighting = $derived((run?.combatants ?? []).filter((c) => c.kind === "character" && characterById.has(c.character) && !linkedPc(c)));

  async function end() {
    if (!run || ending) return;
    ending = true;
    endError = null;
    try {
      await saving;
      if (writeBack) {
        for (const c of fighting) {
          const character = characterById.get(c.character)!;
          await saveCharacter(campaign, { ...$state.snapshot(character), currentHp: c.hp, tempHp: c.tempHp });
        }
      }
      await endRun(campaign, id);
      endDialog?.close();
      await gotoReplacing(prepare);
    } catch (e) {
      endError = t.ending.error(errorMessage(e));
    } finally {
      ending = false;
    }
  }
</script>

<svelte:window onkeydown={onKey} />
<svelte:head><title>{encounter?.name ?? strings.encounters.title}</title></svelte:head>

<!-- A combatant's picture in a hexagon, else its first letter (ember for monsters and custom ones). -->
{#snippet hex(c: Combatant, extra = "")}
  {@const src = pictureOf(c)}
  {#if src}
    <img class="hex {extra}" {src} alt="" />
  {:else}
    <span class="hex {extra}" class:monster={c.kind !== "character"} aria-hidden="true">{c.name.charAt(0)}</span>
  {/if}
{/snippet}

<!-- Three pips of one kind of death save; the same pip again takes one back. -->
{#snippet pips(c: Combatant, kind: "deathSuccesses" | "deathFailures")}
  {@const success = kind === "deathSuccesses"}
  <span class="pips" role="group" aria-label={success ? t.deathSuccesses : t.deathFailures}>
    <i class="ti {success ? 'ti-check accent' : 'ti-x danger'}" aria-hidden="true"></i>
    {#each [1, 2, 3] as n (n)}
      <button
        class="pip"
        aria-label={success ? t.deathSuccess(n) : t.deathFailure(n)}
        aria-pressed={c[kind] >= n}
        disabled={linkedPc(c)}
        onclick={() => changeById(c.id, (x) => R.setDeathSave(x, kind, n))}
      >
        <span class="dot" class:success class:on={c[kind] >= n}></span>
      </button>
    {/each}
  </span>
{/snippet}

{#if loadError}
  <div class="status">
    <p class="error" role="alert">{loadError}</p>
  </div>
{:else if !run || !encounter}
  <p class="status muted" role="status">{t.loading}</p>
{:else if run.round === 0}
  <div class="run">
    <header class="bar">
      <h1 class="title">{encounter.name}</h1>
      <button class="btn" onclick={() => adder?.open()}><i class="ti ti-plus" aria-hidden="true"></i>{t.addCombatant}</button>
      <button class="btn" onclick={() => endDialog?.showModal()}>{t.end}</button>
    </header>
    {#if saveError}<p class="error" role="alert">{saveError}</p>{/if}
    {#if lookupError}<p class="error" role="alert">{lookupError}</p>{/if}

    <section class="setup panel" aria-labelledby="setup-title">
      <div class="setup-head">
        <h2 id="setup-title" class="label">{t.setup}</h2>
        {#if groups.length > 0}<button class="btn" onclick={rerollAll}><i class="ti ti-dice-5" aria-hidden="true"></i>{t.reroll}</button>{/if}
      </div>
      {#if run.combatants.length === 0}
        <p class="muted">{t.nobody}</p>
      {/if}
      <ul class="setup-list">
        {#each groups as g (g.key)}
          {@const first = g.members[0]}
          {@const apart = isSeparate(g)}
          <li class="setup-row">
            {@render hex(first)}
            <span class="setup-name">
              <strong>{groupName(g)}</strong>
              <span class="muted small">{t.bonus(signed(first.initiativeBonus))}</span>
            </span>
            {#if g.members.length > 1}
              <label class="check">
                <input type="checkbox" checked={apart} onchange={() => toggleSeparate(g)} />
                {t.separately}
              </label>
            {/if}
            {#if !apart}
              <input
                class="text-field dice init-field"
                type="number"
                aria-label={t.initiativeOf(first.name)}
                value={first.initiative ?? ""}
                oninput={(e) => setInitiative(g.members.map((m) => m.id), typed(e))}
              />
              <button class="icon-btn icon-btn-lg" aria-label={t.rerollOne(first.name)} onclick={() => rerollGroup(g)}>
                <i class="ti ti-dice-5" aria-hidden="true"></i>
              </button>
            {/if}
          </li>
          {#if apart}
            {#each g.members as m (m.id)}
              <li class="setup-row sub">
                <span class="setup-name">{m.name}</span>
                <input class="text-field dice init-field" type="number" min={-R.MAX_NUMBER} max={R.MAX_NUMBER} aria-label={t.initiativeOf(m.name)} value={m.initiative ?? ""} oninput={(e) => setInitiative([m.id], typed(e))} />
                <button class="icon-btn icon-btn-lg" aria-label={t.rerollOne(m.name)} onclick={async () => setInitiative([m.id], await roll(m.initiativeBonus))}>
                  <i class="ti ti-dice-5" aria-hidden="true"></i>
                </button>
              </li>
            {/each}
          {/if}
        {/each}
        {#each pcs as c (c.id)}
          <li class="setup-row">
            {@render hex(c)}
            <span class="setup-name">
              <strong>{c.name}</strong>
              <span class="muted small">{setupLine(c)}</span>
            </span>
            <input class="text-field dice init-field" type="number" min={-R.MAX_NUMBER} max={R.MAX_NUMBER} aria-label={t.initiativeOf(c.name)} value={c.initiative ?? ""} oninput={(e) => setInitiative([c.id], typed(e))} />
            <span class="icon-btn-lg" aria-hidden="true"></span>
          </li>
        {/each}
      </ul>
      <div class="setup-foot">
        {#if missing > 0}<span class="muted" role="status">{t.missing(missing)}</span>{/if}
        <button class="btn btn-primary" disabled={missing > 0 || run.combatants.length === 0} onclick={begin}>
          {t.begin}
          <i class="ti ti-player-play" aria-hidden="true"></i>
        </button>
      </div>
    </section>
  </div>
{:else}
  <div class="run">
    <header class="bar turn-bar">
      <div class="stat">
        <span class="label">{t.round}</span>
        <span class="round dice">{fmt.format(run.round)}</span>
      </div>
      <div class="acting">
        <span class="label accent">{t.acting}</span>
        <h1 class="title">{current?.name ?? ""}</h1>
      </div>
      <div class="actions">
        {#if encounter.map}
          <button class="btn btn-lg" onclick={openMap}>
            <i class="ti ti-map" aria-hidden="true"></i>{t.map}
          </button>
        {/if}
        <button class="btn btn-lg" disabled={run.round === 1 && run.turn === 0} onclick={() => step(-1)}>
          <i class="ti ti-chevron-left" aria-hidden="true"></i>{t.previous}
        </button>
        <button class="btn btn-primary" onclick={() => step(1)}>
          {t.next}<i class="ti ti-chevron-right" aria-hidden="true"></i>
        </button>
        <button class="btn btn-lg" onclick={() => adder?.open()}><i class="ti ti-plus" aria-hidden="true"></i>{t.addCombatant}</button>
        <button class="btn btn-lg btn-quiet" onclick={() => endDialog?.showModal()}>{t.end}</button>
      </div>
    </header>
    {#if saveError}<p class="error" role="alert">{saveError}</p>{/if}
    {#if lookupError}<p class="error" role="alert">{lookupError}</p>{/if}

    <div class="columns">
      <!-- 1. Initiative -->
      <section class="order" aria-label={t.order} bind:this={orderList}>
        {#each run.combatants as c, i (c.id)}
          {@const active = i === run.turn}
          {@const state = R.health(c)}
          {@const dying = R.isDying(c)}
          <div
            class="row"
            class:active
            class:selected={c.id === selected?.id}
            class:gone={state === "defeated"}
            class:bloodied={state === "bloodied"}
            aria-current={active ? "step" : undefined}
          >
            <button class="pick" aria-label={t.select(c.name)} aria-pressed={c.id === selected?.id} onclick={() => (selectedId = c.id)}>
              <span class="init dice muted">{c.initiative ?? "–"}</span>
              {@render hex(c, active ? "now" : "")}
              <span class="who">
                <span class="name-line">
                  <strong>{c.name}</strong>
                  {#if active}<span class="tag tag-turn">{t.turn}</span>{/if}
                </span>
                {#if subLine(c)}<span class="muted smaller">{subLine(c)}</span>{/if}
              </span>
              <span class="ac dice strong"><i class="ti ti-shield muted" role="img" aria-label={t.armorClass}></i>{c.ac ?? "–"}</span>
            </button>
            {#if dying}
              <div class="indent saves">
                {@render pips(c, "deathSuccesses")}
                {@render pips(c, "deathFailures")}
                {#if state === "dead" || state === "stable"}<span class="label" class:danger={state === "dead"} class:accent={state === "stable"}>{t.health[state]}</span>{/if}
              </div>
            {:else}
              <div class="indent hp-line">
                <span class="dice strong hp-text">{hpText(c)}</span>
                <div class="hp-bar"><div class="hp-fill {hpLevel(c)}" style:width="{hpShare(c) * 100}%"></div></div>
                {#if c.tempHp > 0}<span class="dice smaller accent">+{fmt.format(c.tempHp)}</span>{/if}
              </div>
            {/if}
            {#if c.concentrating || c.conditions.length > 0}
              <div class="indent tags">
                {#if c.concentrating}<span class="small-tag conc">{t.concentrating}</span>{/if}
                {#each c.conditions as cond (cond.index)}
                  <span class="small-tag cond">
                    {cond.name}
                    <button class="tag-x" aria-label={t.removeCondition(cond.name, c.name)} onclick={() => toggleCondition(c, cond)}>
                      <i class="ti ti-x" aria-hidden="true"></i>
                    </button>
                  </span>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </section>

      <!-- 2. Selected combatant: tracker, then stat block or quick reference -->
      {#if selected}
        {@const state = R.health(selected)}
        <main class="panel main" aria-label={t.selected} bind:this={mainPanel}>
          <div class="main-head">
            {@render hex(selected, "big-hex")}
            <div class="who">
              <h2 class="main-name">{selected.name}</h2>
              <span class="muted small">{selectedLine(selected)}</span>
            </div>
          </div>

          <section class="tracker" aria-label={t.tracker}>
            <div class="hp-row">
              <div class="hp-sum">
                <div class="hp-numbers">
                  <span class="dice hp-big">{selected.hp}</span>
                  {#if selected.maxHp > 0}<span class="dice muted hp-max">/ {selected.maxHp}</span>{/if}
                  {#if selected.tempHp > 0}<span class="dice small accent">{t.tempShort(fmt.format(selected.tempHp))}</span>{/if}
                  {#if state}
                    <span class="label" class:danger={state !== "bloodied" && state !== "stable"} class:ember={state === "bloodied"} class:accent={state === "stable"}>
                      {t.health[state]}
                    </span>
                  {/if}
                </div>
                <div class="hp-bar thick"><div class="hp-fill {hpLevel(selected)}" style:width="{hpShare(selected) * 100}%"></div></div>
              </div>
              <div class="hp-controls">
                <label class="amount">
                  <span class="label">{t.amount}</span>
                  <input class="text-field dice amount-field" type="number" min="0" max={R.MAX_NUMBER} inputmode="numeric" bind:value={amount} />
                </label>
                <button class="btn hp-btn hp-damage" disabled={linkedPc(selected)} onclick={() => applyHp("damage")}>{t.damage}</button>
                <button class="btn hp-btn hp-heal" disabled={linkedPc(selected)} onclick={() => applyHp("heal")}>{t.heal}</button>
                <button class="btn hp-btn" disabled={linkedPc(selected)} onclick={() => applyHp("temp")}>{t.temp}</button>
              </div>
            </div>
            <div class="cond-row">
              <label for="add-condition" class="label">{t.conditions}</label>
              {#each selected.conditions as cond (cond.index)}
                <span class="cond-chip">
                  {cond.name}
                  <button class="tag-x" aria-label={t.removeCondition(cond.name, selected.name)} onclick={() => selected && toggleCondition(selected, cond)}>
                    <i class="ti ti-x" aria-hidden="true"></i>
                  </button>
                </span>
              {/each}
              {#if conditions.length > 0}
                <select id="add-condition" class="add-condition" value="" onchange={addCondition}>
                  <option value="">{t.addCondition}</option>
                  {#each conditions.filter((x) => !selected?.conditions.some((y) => y.index === x.index)) as row (row.index)}
                    <option value={row.index} disabled={immune.has(row.index)}>{immune.has(row.index) ? t.immune(row.name) : row.name}</option>
                  {/each}
                </select>
              {/if}
              <span class="grow"></span>
              <button class="btn conc-toggle" aria-pressed={selected.concentrating} onclick={() => changeSelected((c) => ({ ...c, concentrating: !c.concentrating }))}>
                {selected.concentrating ? t.concentrating : t.concentration}
              </button>
            </div>
          </section>

          {#if selected.kind === "monster" && statBlock}
            {#if statBlock.error}
              <p class="error">{statBlock.error}</p>
            {:else if statBlock.entry}
              <!-- The same stat block as Prepare's details card. -->
              <div class="prose stat-block"><MonsterBlock loaded={statBlock.entry} /></div>
            {:else}
              <p class="muted">{strings.encounters.browser.previewLoading}</p>
            {/if}
          {:else if selected.kind === "character"}
            {@const sheet = characterById.get(selected.character)}
            {#if sheet}
              <section class="quick" aria-labelledby="quick-title">
                <h3 id="quick-title" class="label">{t.quickReference}</h3>
                <div class="quick-tiles">
                  {#each [[t.passivePerception, sheet.passivePerception], [t.passiveInsight, sheet.passiveInsight], [t.passiveInvestigation, sheet.passiveInvestigation]] as [label, value] (label)}
                    <div class="tile"><span class="label">{label}</span><span class="dice tile-value">{value ?? "–"}</span></div>
                  {/each}
                </div>
              </section>
            {/if}
          {/if}
        </main>
      {/if}

      <!-- 3. DM notes and the encounter log -->
      <aside class="panel notes" aria-labelledby="notes-title">
        <h2 id="notes-title" class="notes-title">{t.dmNotes}</h2>
        <div class="notes-text">
          <NotesPanel topic="description" notes={encounter.notes} map={encounter.map ?? null} locked setMap={null} />
        </div>
        <div class="log">
          <label for="log-note" class="label">{t.log}</label>
          <textarea id="log-note" class="text-field log-field" rows="2" maxlength={R.MAX_LOG_TEXT} placeholder={t.logPlaceholder} bind:value={note}></textarea>
          <button class="btn add-log" disabled={!note.trim()} onclick={addNote}>{t.addToLog}</button>
          <ol class="log-list">
            {#each run.log as entry, i (i)}
              <li>
                <span class="dice smaller muted">{t.stamp(fmt.format(entry.round), entry.actor)}</span>
                <span class="small">{entry.text}</span>
              </li>
            {/each}
          </ol>
        </div>
      </aside>
    </div>
  </div>
{/if}

<AddCombatant bind:this={adder} {monsters} characters={absent} onpick={addPicked} />

<dialog bind:this={mapDialog} class="dialog map-dialog" aria-labelledby="map-title" onclose={() => (mapOpen = false)}>
  <div class="dialog-head">
    <h2 id="map-title" class="notes-title">{t.map}</h2>
    <button class="icon-btn icon-btn-lg" aria-label={t.closeMap} onclick={() => mapDialog?.close()}>
      <i class="ti ti-x" aria-hidden="true"></i>
    </button>
  </div>
  {#if mapOpen && encounter}
    <div class="map-body">
      <div class="map-view">
        <NotesPanel topic="map" notes={encounter.notes} map={encounter.map ?? null} locked setMap={null} />
      </div>
      {#if encounter.notes.pins.length > 0}
        <div class="legend">
          <h3 class="label">{t.markers}</h3>
          {#each encounter.notes.pins as pin, i (i)}
            <div class="marker">
              <span class="marker-hex dice">{i + 1}</span>
              <strong>{pin.title || strings.encounters.notes.untitled}</strong>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</dialog>

<dialog bind:this={resumeDialog} class="dialog end" aria-labelledby="resume-title" aria-describedby="resume-question">
  <h2 id="resume-title">{t.resume.title}</h2>
  <p id="resume-question">{t.resume.question(encounter?.name ?? "", fmt.format(run?.round ?? 1))}</p>
  {#if changedSince && encounter?.status === "ready"}<p class="ember">{t.resume.changed}</p>{/if}
  <div class="end-actions">
    {#if encounter?.status === "ready"}
      <button class="btn" disabled={restarting} onclick={restart}>{t.resume.restart}</button>
    {/if}
    <!-- svelte-ignore a11y_autofocus: carrying on is the safe default (Escape does the same) -->
    <button class="btn btn-primary" autofocus disabled={restarting} onclick={() => resumeDialog?.close()}>{t.resume.continue}</button>
  </div>
</dialog>

<dialog bind:this={endDialog} class="dialog end" aria-labelledby="end-title" onclose={() => (endError = null)}>
  <h2 id="end-title">{t.ending.title}</h2>
  <p>{t.ending.question(encounter?.name ?? "")}</p>
  {#if fighting.length > 0}
    <label class="check"><input type="checkbox" bind:checked={writeBack} />{t.ending.writeBack}</label>
  {/if}
  {#if endError}<p class="error" role="alert">{endError}</p>{/if}
  <div class="end-actions">
    <!-- svelte-ignore a11y_autofocus: the safe answer is the default -->
    <button class="btn" autofocus onclick={() => endDialog?.close()}>{t.ending.cancel}</button>
    <button class="btn btn-primary" disabled={ending} onclick={end}>{t.ending.end}</button>
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

  /* Nothing goes below the fold: the screen is the window's height, and each part that can grow scrolls inside. */
  .run {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    height: 100%;
    padding: var(--space-4) var(--space-4) var(--space-5);
  }

  .bar {
    flex: none;
  }

  .panel {
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-4);
  }

  .turn-bar {
    gap: var(--space-6);
    padding: var(--space-4) var(--space-6);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .title {
    margin: 0;
    font: var(--text-h1);
    letter-spacing: var(--tracking-tight);
    overflow-wrap: anywhere;
  }

  .bar > .title {
    flex: 1;
    font: var(--text-h2);
  }

  .stat,
  .acting {
    display: flex;
    flex-direction: column;
  }

  .acting {
    flex: 1 1 280px;
    min-width: 0;
  }

  .round {
    font-size: var(--text-2xl);
    font-weight: var(--weight-bold);
    line-height: var(--leading-tight);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .btn-lg {
    min-height: var(--control-height-lg);
  }

  .btn-quiet {
    border-color: transparent;
    background: none;
    color: var(--color-text-muted);
  }

  .accent {
    color: var(--color-accent);
  }

  .danger {
    color: var(--color-danger);
  }

  .ember {
    color: var(--ember);
  }

  .small {
    font: var(--text-body-sm);
  }

  .smaller {
    font-size: var(--text-xs);
  }

  .strong {
    font-weight: var(--weight-bold);
  }

  .grow {
    flex: 1 1 auto;
  }

  /* ---- Rolling initiative ---- */
  .setup {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-height: 0;
    max-width: var(--wizard-content-width);
    padding: var(--space-6);
  }

  .setup-list {
    min-height: 0;
    overflow-y: auto;
  }

  .setup-head,
  .setup-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }

  .setup-foot {
    justify-content: flex-end;
  }

  .setup-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .setup-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .setup-row.sub {
    margin-left: var(--space-8);
  }

  .setup-name {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .init-field {
    width: var(--level-input-width);
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font: var(--text-body-sm);
  }

  /* ---- Hexagons ---- */
  .hex {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: var(--roster-badge-width);
    height: var(--roster-badge-height);
    clip-path: polygon(25% 3%, 75% 3%, 100% 50%, 75% 97%, 25% 97%, 0 50%);
    background: var(--color-border);
    color: var(--color-text);
    font-family: var(--font-display);
    font-weight: var(--weight-display);
  }

  img.hex {
    object-fit: cover;
  }

  .hex.monster {
    background: var(--ember);
    color: var(--on-fill);
  }

  .hex.now {
    background: var(--color-accent);
    color: var(--color-accent-text);
  }

  .big-hex {
    width: var(--control-height-xl);
    height: calc(var(--control-height-xl) * 0.875);
    font-size: var(--text-xl);
  }

  /* ---- The three columns ---- */
  .columns {
    display: grid;
    flex: 1;
    grid-template-areas: "order main notes";
    grid-template-columns: minmax(0, var(--run-order-width)) minmax(0, 1fr) minmax(0, var(--run-notes-width));
    grid-template-rows: minmax(0, 1fr);
    gap: var(--space-4);
    min-height: 0;
  }

  /* ---- 1. Initiative ---- */
  .order {
    display: flex;
    grid-area: order;
    flex-direction: column;
    gap: var(--space-2);
    min-height: 0;
    overflow-y: auto;
  }

  .row {
    flex: none;
  }

  .row {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .row.bloodied {
    background: var(--color-danger-soft);
  }

  .row.selected {
    border-color: var(--color-border-strong);
    background: var(--color-surface-sunken);
  }

  /* The active turn: a thicker potion border, the padding one pixel less so nothing moves. */
  .row.active {
    padding: calc(var(--space-2) - var(--border-width)) calc(var(--space-3) - var(--border-width));
    border: calc(2 * var(--border-width)) solid var(--color-accent);
  }

  .row.gone {
    color: var(--color-text-muted);
  }

  .pick {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    min-height: var(--control-height-lg);
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .init {
    flex: none;
    width: var(--space-7);
    font-size: var(--text-lg);
    font-weight: var(--weight-bold);
  }

  .who {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-width: 0;
  }

  .name-line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .ac {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-1);
  }

  /* The lines under a row's name start under it, past the initiative. */
  .indent {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    padding-left: calc(var(--space-7) + var(--space-3));
  }

  .saves {
    gap: var(--space-4);
  }

  .hp-text {
    min-width: var(--space-10);
  }

  .hp-line .hp-bar {
    flex: 1 1 auto;
  }

  .tag {
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .tag-turn {
    background: var(--color-accent);
    color: var(--color-accent-text);
  }

  .tags {
    gap: var(--space-1);
  }

  .small-tag {
    display: inline-flex;
    align-items: center;
    height: var(--space-6);
    padding: 0 var(--space-1);
    border: var(--border-width) solid;
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
  }

  .small-tag.conc {
    border-color: var(--color-accent);
    color: var(--color-accent);
  }

  .small-tag.cond {
    padding-right: 0;
    border-color: var(--ember);
    color: var(--ember);
  }

  .tag-x {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--space-6);
    height: var(--space-6);
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    cursor: pointer;
  }

  .pips {
    display: flex;
    align-items: center;
    gap: 0;
  }

  .pip {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--space-6);
    height: var(--space-6);
    padding: 0;
    border: none;
    background: none;
    cursor: pointer;
  }

  .dot {
    width: var(--space-3);
    height: var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-pill);
  }

  .dot.on {
    border-color: var(--color-danger);
    background: var(--color-danger);
  }

  .dot.success.on {
    border-color: var(--color-success);
    background: var(--color-success);
  }

  .hp-bar {
    height: var(--run-bar-height);
    overflow: hidden;
    border-radius: var(--radius-sm);
    background: var(--color-border);
  }

  .hp-bar.thick {
    height: var(--sheet-bar-height);
  }

  .hp-fill {
    height: 100%;
  }

  .hp-fill.high {
    background: var(--color-success);
  }

  .hp-fill.mid {
    background: var(--ember);
  }

  .hp-fill.low {
    background: var(--color-danger);
  }

  /* ---- 2. The selected combatant ---- */
  .main {
    display: flex;
    grid-area: main;
    flex-direction: column;
    gap: var(--space-4);
    min-height: 0;
    padding: var(--space-6);
    overflow-y: auto;
  }

  .main-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-4);
  }

  .main-name {
    margin: 0;
    font: var(--text-h2);
  }

  .tracker {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  .hp-row {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-6);
  }

  .hp-sum {
    display: flex;
    flex: 1 1 200px;
    flex-direction: column;
    gap: var(--space-2);
  }

  .hp-numbers {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-2);
  }

  .hp-big {
    font-size: var(--text-2xl);
    font-weight: var(--weight-bold);
    line-height: var(--leading-tight);
  }

  .hp-max {
    font-size: var(--text-xl);
  }

  /* Compact: the amount and its three buttons at the normal control height, close together. */
  .hp-controls {
    display: flex;
    flex: none;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-1);
  }

  .hp-btn {
    padding: 0 var(--space-3);
  }

  .amount {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .amount-field {
    width: var(--run-amount-width);
    font-size: var(--text-md);
  }

  .hp-damage {
    border-color: var(--color-danger);
    color: var(--color-danger);
  }

  .hp-heal {
    border-color: var(--color-success);
    color: var(--color-success);
  }

  .cond-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    padding-top: var(--space-3);
    border-top: var(--border-width) solid var(--color-border);
  }

  .cond-chip {
    display: inline-flex;
    align-items: center;
    height: var(--control-height-sm);
    padding-left: var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--ember);
    color: var(--on-fill);
    font: var(--text-body-sm);
    font-weight: var(--weight-bold);
  }

  .add-condition {
    min-height: var(--control-height-sm);
    border-style: dashed;
    font-weight: var(--weight-bold);
  }

  .conc-toggle {
    min-height: var(--control-height-sm);
  }

  .conc-toggle[aria-pressed="false"] {
    border-style: dashed;
    color: var(--color-text-muted);
  }

  .conc-toggle[aria-pressed="true"] {
    border-color: var(--color-accent);
    background: var(--color-accent);
    color: var(--color-accent-text);
  }

  /* .prose centres a reading page; here it fills the panel like Prepare's details card. */
  .stat-block {
    max-width: none;
    margin: 0;
  }

  /* The ability scores across the whole panel, six equal columns. */
  .stat-block :global(.abilities) {
    width: 100%;
    table-layout: fixed;
  }

  .quick {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .quick h3 {
    margin: 0;
  }

  .quick-tiles {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-2);
  }

  .tile {
    display: flex;
    flex-direction: column;
    padding: var(--space-2) var(--space-3);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .tile-value {
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
    line-height: var(--leading-normal);
  }

  /* ---- 3. DM notes and the log ---- */
  .notes {
    display: flex;
    grid-area: notes;
    flex-direction: column;
    gap: var(--space-4);
    min-height: 0;
    padding: var(--space-5);
    overflow-y: auto;
  }

  .notes-title {
    margin: 0;
    font: var(--text-h2);
  }

  .notes-text {
    font: var(--text-body-sm);
  }

  .log {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-top: var(--space-4);
    border-top: var(--border-width) solid var(--color-border);
  }

  .log-field {
    padding: var(--space-2) var(--space-3);
    font: var(--text-body-sm);
    resize: vertical;
  }

  .add-log {
    align-self: flex-start;
  }

  .log-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .log-list li {
    display: flex;
    flex-direction: column;
    padding-bottom: var(--space-2);
    border-bottom: var(--border-width) solid var(--color-border);
  }

  /* ---- Dialogs ---- */
  .map-dialog {
    width: min(100% - 2 * var(--space-4), var(--run-map-width));
    max-height: calc(100% - 2 * var(--space-4));
    overflow-y: auto;
    padding: var(--space-6);
  }

  .dialog-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }

  .map-body {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: var(--space-4);
  }

  .map-view {
    display: flex;
    flex: 999 1 560px;
    flex-direction: column;
    min-width: 0;
    aspect-ratio: 16 / 10;
  }

  .legend {
    display: flex;
    flex: 1 1 260px;
    flex-direction: column;
    gap: var(--space-3);
    min-width: 0;
  }

  .legend h3 {
    margin: 0;
  }

  .marker {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .marker-hex {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: var(--character-level-width-compact);
    height: var(--character-level-height-compact);
    clip-path: polygon(25% 3%, 75% 3%, 100% 50%, 75% 97%, 25% 97%, 0 50%);
    background: var(--color-accent);
    color: var(--color-accent-text);
    font-weight: var(--weight-bold);
  }

  /* Too narrow for three columns: the DM notes go under the initiative order, each half the height. */
  @media (max-width: 1200px) {
    .columns {
      grid-template-areas:
        "order main"
        "notes main";
      grid-template-columns: minmax(0, var(--run-order-width)) minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr) minmax(0, 1fr);
    }
  }

  /* A narrow window: everything in one column that scrolls as a whole. */
  @media (max-width: 900px) {
    .run {
      height: auto;
    }

    .columns {
      grid-template-areas: "order" "main" "notes";
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: none;
    }

    .order,
    .main,
    .notes {
      overflow: visible;
    }
  }

  .end-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>
