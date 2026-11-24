<script lang="ts">
  // The monster form's own widgets for some of its fields (entryTypes.ts): challenge rating with the usual XP and
  // proficiency bonus, armor class and hit points with the average of the hit dice, the ability tiles with their saving
  // throws, the skills, the damage defenses and the languages. They write the pack schema's fields; the choices (skills,
  // damage types, languages) and the usual values come from the switched-on packs.
  import type { Choice, MonsterRules, PackEntry } from "$lib/api/homebrew";
  import Segmented from "$lib/components/Segmented.svelte";
  import { strings } from "$lib/strings";
  import { title } from "$lib/views/library/catalog";
  import { ABILITIES, modifier, signed } from "$lib/views/library/entries/format";
  import { getField, reference, setField, type Field } from "./entryTypes";
  import { averageHp, bonusFor, hpFormula, joinList, parseHitDice, splitList, usualFor, type Level } from "./monster";

  let {
    field,
    draft = $bindable(),
    rules,
    choices,
    levels = $bindable(),
    pb,
  }: {
    /** The field: `identity`, `challenge`, `defense`, `speed`, `abilities`, `proficiencies`, `damage`, `senses` or
     *  `languages`. */
    field: Field;
    draft: PackEntry;
    rules: MonsterRules | null;
    choices: (kind: string, field?: string) => Choice[];
    /** Each saving throw's and skill's level, by its proficiency's index (kept by the form). */
    levels: Record<string, Level>;
    /** The proficiency bonus the bonuses are worked out with, if known. */
    pb: number | undefined;
  } = $props();

  const t = strings.homebrew;
  const m = t.monster;
  const e = strings.entry;
  const id = $props.id();
  const key = $derived(field.key);
  /** A row's fields (size, type and alignment; the speeds; the senses), all side by side; Hover sits by Fly's label. */
  const ROWS = new Set(["identity", "speed", "senses"]);
  const rowFields = $derived(field.input === "group" ? field.fields.filter((f) => f.input !== "bool") : []);
  const number = new Intl.NumberFormat();

  const numberOf = (k: string) => {
    const v = getField(draft, k);
    return typeof v === "number" ? v : undefined;
  };
  const setNumber = (k: string, input: HTMLInputElement) => setField(draft, k, Number.isNaN(input.valueAsNumber) ? undefined : input.valueAsNumber);
  const mod = (ability: string) => modifier(numberOf(ability) ?? 10);

  // ---- Saving throws and skills (`proficiencies`) ---------------------------------------------------
  type Ref = { index: string; name: string; url?: string };
  type Bonus = { value: number; proficiency: Ref };
  const rows = $derived.by((): Bonus[] => {
    const value = draft.proficiencies;
    return Array.isArray(value) ? value.filter((b): b is Bonus => typeof b?.proficiency?.index === "string") : [];
  });
  const abilityOf = (index: string) => rules?.proficiencyAbilities[index];
  const proficiencyChoices = $derived(choices("proficiencies"));
  const saveFor = (ability: string) => proficiencyChoices.find((c) => c.name.startsWith("Saving Throw:") && abilityOf(c.index) === ability);
  const skillName = (name: string) => name.replace(/^Skill:\s*/, "");

  function add(choice: Choice) {
    const ability = abilityOf(choice.index);
    const level: Level = ability && pb !== undefined ? "prof" : "custom";
    const value = ability && pb !== undefined ? bonusFor("prof", mod(ability), pb) : ability ? mod(ability) : 0;
    levels[choice.index] = level;
    setField(draft, "proficiencies", [...rows, { value, proficiency: reference("proficiencies", choice.index, choice.name) }]);
  }

  function remove(index: string) {
    delete levels[index];
    const left = (draft.proficiencies as Bonus[]).filter((r) => r?.proficiency?.index !== index);
    setField(draft, "proficiencies", left.length ? left : undefined);
  }

  function setLevel(row: Bonus, level: Level) {
    levels[row.proficiency.index] = level;
    const ability = abilityOf(row.proficiency.index);
    if (level !== "custom" && ability && pb !== undefined) row.value = bonusFor(level, mod(ability), pb);
  }

  // ---- Damage defenses (three lists of damage types) ------------------------------------------------
  const DEFENSES = [
    { key: "damage_vulnerabilities", value: "vulnerable", label: m.vulnerable },
    { key: "damage_resistances", value: "resistant", label: m.resistant },
    { key: "damage_immunities", value: "immune", label: m.immune },
  ] as const;
  type Defense = (typeof DEFENSES)[number]["value"];
  const listOf = (k: string): string[] => {
    const v = getField(draft, k);
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : [];
  };
  const defenses = $derived.by(() => {
    const seen = new Map<string, Defense>();
    for (const d of DEFENSES) for (const v of listOf(d.key)) if (!seen.has(v)) seen.set(v, d.value);
    return [...seen];
  });
  function setDefense(type: string, to: Defense | null) {
    for (const d of DEFENSES) {
      const list = listOf(d.key).filter((v) => v !== type);
      if (d.value === to) list.push(type);
      setField(draft, d.key, list.length ? list : undefined);
    }
  }

  // ---- Languages (one text, a list of names) --------------------------------------------------------
  const languages = $derived(splitList(draft.languages));
  let language = $state("");
  function addLanguage() {
    const name = language.trim();
    language = "";
    if (name && !languages.includes(name)) setField(draft, "languages", joinList([...languages, name]));
  }

  // ---- Hit points and challenge ---------------------------------------------------------------------
  const DICE = [4, 6, 8, 10, 12, 20];
  const hitDice = $derived(parseHitDice(draft.hit_dice));
  const conMod = $derived(mod("constitution"));
  const average = $derived(hitDice ? averageHp(hitDice.count, hitDice.die, conMod) : undefined);
  function setHitDice(count: number, die: number) {
    setField(draft, "hit_dice", count > 0 ? `${count}d${die}${hitDice?.rest ?? ""}` : undefined);
  }
  const usual = $derived(usualFor(rules, draft.challenge_rating));
</script>

{#if ROWS.has(key)}
  <fieldset class="field">
    <legend class:sr-only={key === "identity"}>{field.label}</legend>
    <div class="row" style:--columns={rowFields.length}>
      {#each rowFields as sub (sub.key)}
        <div class="field">
          <div class="label-row">
            <label for="{id}-{sub.key}">{sub.label}</label>
            {#if sub.key === "speed.fly"}
              <label class="check">
                <input type="checkbox" checked={!!getField(draft, "speed.hover")} onchange={(ev) => setField(draft, "speed.hover", ev.currentTarget.checked || undefined)} />
                {m.hover}
              </label>
            {/if}
          </div>
          {#if sub.input === "number"}
            <input id="{id}-{sub.key}" type="number" class="dice" min={sub.min} value={numberOf(sub.key) ?? ""} oninput={(ev) => setNumber(sub.key, ev.currentTarget)} />
          {:else if sub.input === "text"}
            {@const value = getField(draft, sub.key)}
            <input
              id="{id}-{sub.key}"
              value={typeof value === "string" ? value : ""}
              maxlength="200"
              autocomplete="off"
              list={sub.suggest ? `${id}-${sub.key}-list` : undefined}
              oninput={(ev) => setField(draft, sub.key, ev.currentTarget.value)}
            />
            {#if sub.suggest}
              <datalist id="{id}-{sub.key}-list">
                {#each choices(sub.suggest.kind, sub.suggest.field) as choice (choice.index)}<option value={choice.name}></option>{/each}
              </datalist>
            {/if}
          {/if}
        </div>
      {/each}
    </div>
  </fieldset>
{:else if key === "challenge"}
  <div class="grid">
    <label class="field">
      <span>{e.cr}</span>
      <input type="number" class="dice" min="0" step="any" value={numberOf("challenge_rating") ?? ""} oninput={(ev) => setNumber("challenge_rating", ev.currentTarget)} />
    </label>
    <div class="field">
      <label for="{id}-xp">{m.xp}</label>
      <div class="with">
        <input id="{id}-xp" type="number" class="dice" min="0" value={numberOf("xp") ?? ""} oninput={(ev) => setNumber("xp", ev.currentTarget)} />
        {#if typeof usual?.xp === "number" && usual.xp !== draft.xp}
          {@const xp = usual.xp}
          <button class="btn small suggest" type="button" aria-label={m.useFor(number.format(xp), m.xp)} onclick={() => setField(draft, "xp", xp)}>{m.use(number.format(xp))}</button>
        {/if}
      </div>
    </div>
    <div class="field">
      <label for="{id}-pb">{m.pb}</label>
      <div class="with">
        <input id="{id}-pb" type="number" class="dice" value={numberOf("proficiency_bonus") ?? ""} oninput={(ev) => setNumber("proficiency_bonus", ev.currentTarget)} />
        {#if typeof usual?.pb === "number" && usual.pb !== draft.proficiency_bonus}
          {@const usualPb = usual.pb}
          <button class="btn small suggest" type="button" aria-label={m.useFor(signed(usualPb), m.pb)} onclick={() => setField(draft, "proficiency_bonus", usualPb)}>{m.use(signed(usualPb))}</button>
        {/if}
      </div>
    </div>
  </div>
{:else if key === "defense"}
  <div class="grid">
    <label class="field">
      <span>{e.ac}</span>
      <input type="number" class="dice" min="0" value={numberOf("armor_class.0.value") ?? ""} oninput={(ev) => setNumber("armor_class.0.value", ev.currentTarget)} />
    </label>
    <div class="field">
      <label for="{id}-hp">{e.hp} <span class="muted">({t.required})</span></label>
      <div class="with">
        <input id="{id}-hp" type="number" class="dice" min="1" required value={numberOf("hit_points") ?? ""} oninput={(ev) => setNumber("hit_points", ev.currentTarget)} />
        {#if average !== undefined && average !== draft.hit_points}
          {@const avg = average}
          <button class="btn small suggest" type="button" aria-label={m.useFor(String(avg), e.hp)} onclick={() => setField(draft, "hit_points", avg)}>{m.use(String(avg))}</button>
        {/if}
      </div>
    </div>
    {#if typeof draft.hit_dice === "string" && draft.hit_dice.trim() && !hitDice}
      <!-- Hit dice it can't read stay a text to edit. -->
      <label class="field">
        <span>{m.hitDice}</span>
        <input value={draft.hit_dice} maxlength="200" autocomplete="off" oninput={(ev) => setField(draft, "hit_dice", ev.currentTarget.value)} />
      </label>
    {:else}
      <div class="field">
        <span id="{id}-hd">{m.hitDice}</span>
        <div class="with" role="group" aria-labelledby="{id}-hd">
          <input
            type="number"
            class="dice count"
            min="1"
            aria-label={m.hitDiceCount}
            value={hitDice?.count ?? ""}
            oninput={(ev) => setHitDice(Number.isNaN(ev.currentTarget.valueAsNumber) ? 0 : ev.currentTarget.valueAsNumber, hitDice?.die ?? 8)}
          />
          <select class="dice" aria-label={m.hitDie} value={hitDice?.die ?? 8} disabled={!hitDice} onchange={(ev) => setHitDice(hitDice?.count ?? 1, Number(ev.currentTarget.value))}>
            {#each hitDice && !DICE.includes(hitDice.die) ? [...DICE, hitDice.die] : DICE as die (die)}<option value={die}>d{die}</option>{/each}
          </select>
        </div>
      </div>
    {/if}
  </div>
  {#if hitDice}
    <p class="hint">{m.average} <span class="dice">{hpFormula(hitDice.count, hitDice.die, conMod)}</span></p>
  {/if}
{:else if key === "abilities"}
  <div class="tiles-room">
  <div class="tiles">
    {#each ABILITIES as ability (ability)}
      {@const save = saveFor(ability)}
      {@const row = save ? rows.find((r) => r.proficiency.index === save.index) : undefined}
      <div class="tile">
        <label class="label" for="{id}-{ability}">{e.abilities[ability]}</label>
        <input id="{id}-{ability}" type="number" class="dice score" min="1" max="30" value={numberOf(ability) ?? ""} oninput={(ev) => setNumber(ability, ev.currentTarget)} />
        <span class="dice mod">{signed(mod(ability))}</span>
        <button class="save" type="button" aria-pressed={!!row} disabled={!save} onclick={() => (row ? remove(row.proficiency.index) : save && add(save))}>
          <span class="sr-only">{e.abilities[ability]}</span>
          {m.saveToggle}{#if row && levels[row.proficiency.index] !== "custom"}{" "}{signed(row.value)}{/if}
        </button>
        {#if row && levels[row.proficiency.index] === "custom"}
          <input type="number" class="dice bonus" aria-label={m.bonus(m.saveOf(e.abilities[ability]))} value={row.value} oninput={(ev) => !Number.isNaN(ev.currentTarget.valueAsNumber) && (row.value = ev.currentTarget.valueAsNumber)} />
        {/if}
      </div>
    {/each}
  </div>
  </div>
{:else if key === "proficiencies"}
  {@const skills = rows.filter((r) => r.proficiency.name.startsWith("Skill:"))}
  <div class="field">
    <span id="{id}-skills">{m.skills}</span>
    <div class="rows" role="group" aria-labelledby="{id}-skills">
      {#each skills as row (row.proficiency.index)}
        {@const name = skillName(row.proficiency.name)}
        {@const ability = abilityOf(row.proficiency.index)}
        {@const level = levels[row.proficiency.index] ?? "custom"}
        <div class="skill">
          <span class="skill-name"><strong>{name}</strong>{#if ability}<span class="muted">{e.abilities[ability as (typeof ABILITIES)[number]]}</span>{/if}</span>
          <Segmented
            label={m.levelOf(name)}
            options={[
              { value: "prof", label: m.levels.prof },
              { value: "expert", label: m.levels.expert },
              { value: "custom", label: m.levels.custom },
            ]}
            value={level}
            disabled={(v) => v !== "custom" && (pb === undefined || !ability)}
            onchange={(v) => setLevel(row, v)}
          />
          {#if level === "custom"}
            <input type="number" class="dice bonus" aria-label={m.bonus(name)} value={row.value} oninput={(ev) => !Number.isNaN(ev.currentTarget.valueAsNumber) && (row.value = ev.currentTarget.valueAsNumber)} />
          {:else}
            <strong class="dice bonus-value">{signed(row.value)}</strong>
          {/if}
          <button class="icon-btn" type="button" aria-label={t.removeRef(name)} onclick={() => remove(row.proficiency.index)}><i class="ti ti-x" aria-hidden="true"></i></button>
        </div>
      {/each}
      <select
        class="chip-pick"
        aria-label={m.addSkill}
        value=""
        onchange={(ev) => {
          const index = ev.currentTarget.value;
          ev.currentTarget.value = "";
          const choice = proficiencyChoices.find((c) => c.index === index);
          if (choice) add(choice);
        }}
      >
        <option value="">{m.addSkill}</option>
        {#each proficiencyChoices.filter((c) => c.name.startsWith("Skill:") && !rows.some((r) => r.proficiency.index === c.index)) as choice (choice.index)}
          <option value={choice.index}>{skillName(choice.name)}</option>
        {/each}
      </select>
    </div>
  </div>
{:else if key === "damage"}
  <div class="field">
    <span id="{id}-damage">{m.damage}</span>
    <div class="rows" role="group" aria-labelledby="{id}-damage">
      {#each defenses as [type, defense] (type)}
        <div class="defense">
          <strong>{title(type)}</strong>
          <Segmented label={m.damageOf(title(type))} options={DEFENSES.map((d) => ({ value: d.value, label: d.label }))} value={defense} onchange={(v) => setDefense(type, v)} />
          <button class="icon-btn" type="button" aria-label={t.removeRef(title(type))} onclick={() => setDefense(type, null)}><i class="ti ti-x" aria-hidden="true"></i></button>
        </div>
      {:else}
        <p class="muted">{t.none}</p>
      {/each}
      <select
        class="chip-pick"
        aria-label={m.addDefense}
        value=""
        onchange={(ev) => {
          const type = ev.currentTarget.value;
          ev.currentTarget.value = "";
          if (type) setDefense(type, "resistant");
        }}
      >
        <option value="">{m.addDefense}</option>
        {#each choices("damage-types", "index").filter((c) => !defenses.some(([type]) => type === c.index)) as choice (choice.index)}
          <option value={choice.index}>{title(choice.name)}</option>
        {/each}
      </select>
    </div>
  </div>
{:else if key === "languages"}
  <div class="field">
    <span id="{id}-languages">{e.languages}</span>
    <div class="chips" role="group" aria-labelledby="{id}-languages">
      {#each languages as name (name)}
        <span class="chip-link">
          {name}
          <button class="icon-btn" type="button" aria-label={t.removeRef(name)} onclick={() => setField(draft, "languages", joinList(languages.filter((l) => l !== name)) || undefined)}>
            <i class="ti ti-x" aria-hidden="true"></i>
          </button>
        </span>
      {/each}
      <span class="with">
        <input
          class="language"
          aria-label={m.languageName}
          list="{id}-language-list"
          maxlength="200"
          autocomplete="off"
          bind:value={language}
          onkeydown={(ev) => {
            if (ev.key !== "Enter") return;
            ev.preventDefault();
            addLanguage();
          }}
        />
        <datalist id="{id}-language-list">
          {#each choices("languages").filter((c) => !languages.includes(c.name)) as choice (choice.index)}<option value={choice.name}></option>{/each}
        </datalist>
        <button class="btn small" type="button" disabled={!language.trim()} onclick={addLanguage}><i class="ti ti-plus" aria-hidden="true"></i>{m.addLanguage}</button>
      </span>
    </div>
  </div>
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--homebrew-number-width), 1fr));
    gap: var(--space-3) var(--space-4);
    align-items: end;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .with {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .with input {
    flex: 1;
    min-width: 0;
  }

  .with .count {
    flex: 0 0 var(--homebrew-bonus-width);
  }

  input,
  select {
    min-height: var(--control-height);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    font-weight: var(--weight-regular);
  }

  input {
    padding: 0 var(--space-3);
  }

  .small {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
  }

  .suggest {
    flex: none;
    border-color: var(--color-accent);
    color: var(--color-link);
  }

  .hint {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--text-sm);
  }

  .hint .dice {
    color: var(--color-text);
  }

  /* The six abilities: one row of six, or (narrow) two rows of three, never 4 + 2. */
  .tiles-room {
    container-type: inline-size;
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-2);
  }

  @container (min-width: 560px) {
    .tiles {
      grid-template-columns: repeat(6, minmax(0, 1fr));
    }
  }

  /* A row of fields side by side (size, type and alignment; the speeds; the senses). */
  fieldset.field {
    margin: 0;
    padding: 0;
    border: none;
  }

  legend {
    padding: 0;
    margin-bottom: var(--space-1);
  }

  .row {
    display: grid;
    grid-template-columns: repeat(var(--columns), minmax(0, 1fr));
    gap: var(--space-3);
    align-items: end;
  }

  .row input {
    width: 100%;
  }

  .label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-text-muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-regular);
  }

  .check input {
    min-height: 0;
    width: auto;
  }

  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  .score,
  .bonus {
    width: var(--homebrew-bonus-width);
    padding: 0 var(--space-1);
    text-align: center;
  }

  .mod {
    color: var(--color-success);
    font-weight: var(--weight-bold);
  }

  .save {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-2);
    border: var(--border-width) dashed var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    cursor: pointer;
  }

  .save[aria-pressed="true"] {
    border-style: solid;
    border-color: var(--color-accent);
    background: var(--color-accent);
    color: var(--color-accent-text);
  }

  .save:disabled {
    cursor: not-allowed;
  }

  .rows {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-2);
    font-weight: var(--weight-regular);
  }

  .rows > select {
    align-self: flex-start;
  }

  .skill,
  .defense {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
  }

  .skill-name,
  .defense > strong {
    flex: 1 1 var(--homebrew-number-width);
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    min-width: 0;
  }

  .bonus-value {
    width: var(--homebrew-bonus-width);
    color: var(--color-success);
    text-align: center;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    font-weight: var(--weight-regular);
  }

  .language {
    width: var(--homebrew-number-width);
  }

  .muted {
    color: var(--color-text-muted);
    font-weight: var(--weight-regular);
  }

  p.muted {
    margin: 0;
  }
</style>
