<script lang="ts">
  // A homebrew entry of one type: its name, then the type's fields (entryTypes.ts). Choices and suggestions come from
  // the switched-on packs. Fields the form doesn't show are kept. Save writes the whole book; Delete asks first.
  import { untrack } from "svelte";
  import { homebrewChoices, saveHomebrew, type Book, type Choice, type PackEntry } from "$lib/api/homebrew";
  import { errorMessage } from "$lib/api/storage";
  import OrderList from "$lib/components/OrderList.svelte";
  import RichText from "$lib/components/RichText.svelte";
  import PictureField from "./PictureField.svelte";
  import { moveTo } from "$lib/components/sidebarOrder";
  import { strings } from "$lib/strings";
  import { deleteEntry, openEntry, saveEntry, type Item, type Level } from "./linked";
  import { STAT_BLOCK_PROFICIENCIES } from "$lib/views/library/entries/format";
  import { allFields, getField, reference, setField, type EntryType, type Field, type Source } from "./entryTypes";

  let {
    book,
    type,
    entry,
    changed = $bindable(false),
    onsaved,
    ondeleted,
  }: {
    book: Book;
    type: EntryType;
    entry: PackEntry;
    changed?: boolean;
    onsaved: (book: Book, index: string) => void;
    ondeleted: (book: Book) => void;
  } = $props();

  const t = strings.homebrew;
  const c = strings.catalog.columns;
  const id = $props.id();
  let draft = $state<PackEntry>(untrack(() => openEntry($state.snapshot(book) as Book, type, $state.snapshot(entry) as PackEntry)));
  let saved = $state(untrack(() => JSON.stringify(draft)));
  /** Bumped by Discard, so the text fields (which own their content once filled) start again from the saved text. */
  let resets = $state(0);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let deleting = $state(false);

  $effect(() => {
    changed = JSON.stringify(draft) !== saved;
  });

  // ---- Choices, from the switched-on packs ---------------------------------------------------------
  const sourceKey = (s: Source) => `${s.kind}|${s.field ?? ""}`;
  // A linked field's entries are edited in place, not chosen.
  const sourceOf = (f: Field): Source | null => (f.input === "linked" ? null : "of" in f ? { kind: f.of } : "suggest" in f && f.suggest ? f.suggest : null);
  let choices = $state<Record<string, Choice[]>>({});
  $effect(() => {
    const sources = allFields(type.fields).map(sourceOf).filter((s): s is Source => s !== null);
    Promise.all(sources.map((s) => homebrewChoices(s.kind, s.field).then((list) => [sourceKey(s), list] as const)))
      .then((pairs) => (choices = Object.fromEntries(pairs)))
      .catch((e) => (error = t.loadError(errorMessage(e))));
  });
  const choicesOf = (f: Field) => {
    const source = sourceOf(f);
    return source ? (choices[sourceKey(source)] ?? []) : [];
  };

  // ---- Field values --------------------------------------------------------------------------------
  type Ref = { index: string; name: string; url?: string };
  const get = (f: Field) => getField(draft, f.key);
  const set = (f: Field, value: unknown) => setField(draft, f.key, value);
  const textOf = (f: Field) => {
    const value = get(f);
    return Array.isArray(value) ? value.filter((v) => typeof v === "string").join("\n\n") : typeof value === "string" ? value : typeof value === "number" ? String(value) : "";
  };
  const refsOf = (f: Field): Ref[] => {
    const value = get(f);
    return (Array.isArray(value) ? value : value ? [value] : []).filter((r): r is Ref => typeof r?.index === "string");
  };
  const tagsOf = (f: Field): string[] => {
    const value = get(f);
    return Array.isArray(value) ? value.filter((v): v is string => typeof v === "string") : [];
  };
  const refTo = (f: Field & { of: string }, index: string) => {
    const name = choicesOf(f).find((c) => c.index === index)?.name ?? index;
    return reference(f.of, index, name);
  };
  type Bonus = { value: number; proficiency: Ref };
  const bonusesOf = (f: Field): Bonus[] => {
    const value = get(f);
    return Array.isArray(value) ? value.filter((b): b is Bonus => typeof b?.proficiency?.index === "string") : [];
  };
  const actionsOf = (f: Field): Item[] => {
    const value = get(f);
    return Array.isArray(value) ? value.filter((a): a is Item => typeof a?.name === "string") : [];
  };
  const levelsOf = (f: Field): Level[] => {
    const value = get(f);
    return Array.isArray(value) ? value : [];
  };
  /** Tells action rows apart (they have no index of their own) for as long as the form is open. */
  const rowIds = new WeakMap<object, string>();
  let nextRow = 0;
  const rowId = (row: object) => rowIds.get(row) ?? (rowIds.set(row, `r${nextRow++}`), rowIds.get(row)!);

  function toggleTag(f: Field, tag: string, on: boolean) {
    const tags = tagsOf(f).filter((v) => v !== tag);
    set(f, on ? [...tags, tag] : tags.length ? tags : undefined);
  }

  // ---- Save and delete -----------------------------------------------------------------------------
  async function write(nextBook: Book): Promise<Book | null> {
    saving = true;
    try {
      const result = await saveHomebrew(nextBook);
      error = null;
      return result;
    } catch (e) {
      error = t.saveError(errorMessage(e));
      return null;
    } finally {
      saving = false;
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    const { book: nextBook, index } = saveEntry($state.snapshot(book) as Book, type, $state.snapshot(draft) as PackEntry);
    const result = await write(nextBook);
    if (!result) return;
    draft.index = index;
    draft.name = draft.name.trim();
    saved = JSON.stringify(draft);
    changed = false;
    onsaved(result, index);
  }

  async function remove() {
    deleting = false;
    const result = await write(deleteEntry($state.snapshot(book) as Book, type, entry.index));
    if (!result) return;
    changed = false;
    ondeleted(result);
  }
</script>

<form class="form" onsubmit={save}>
  <p class="label">{type.label}</p>
  <div class="head">
    <label class="field">
      <span>{t.entryName} <span class="muted">({t.required})</span></span>
      <!-- svelte-ignore a11y_autofocus: a new entry starts at its name -->
      <input bind:value={draft.name} required maxlength="200" autocomplete="off" autofocus={!entry.index} />
    </label>
    <PictureField book={book.id} what={t.picture.it} bind:value={() => (typeof draft.image === "string" ? draft.image : undefined), (v) => setField(draft, "image", v)} />
  </div>

  {#key resets}
    {#each type.fields as f (f.key)}{@render field(f)}{/each}
  {/key}

  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="actions">
    <button class="btn btn-primary" type="submit" disabled={saving || (!changed && !!entry.index)}>{t.save}</button>
    <button class="btn" type="button" disabled={saving || !changed} onclick={() => { draft = JSON.parse(saved); resets++; }}>{t.discard}</button>
    {#if entry.index}
      {#if deleting}
        <span class="confirm end">
          {t.deleteEntryQuestion(entry.name)}
          <button class="btn small" type="button" onclick={() => (deleting = false)}>{t.cancel}</button>
          <button class="btn small btn-danger" type="button" disabled={saving} onclick={remove}>{t.delete}</button>
        </span>
      {:else}
        <button class="btn btn-danger end" type="button" disabled={saving} onclick={() => (deleting = true)}>{t.delete}</button>
      {/if}
    {/if}
  </div>
</form>

<!-- One field of any kind; a group renders its fields the same way. -->
{#snippet field(f: Field)}
  {#if f.input === "markdown"}
    <div class="field">
      <span>{f.label}</span>
      <RichText bind:value={() => textOf(f), (v) => set(f, f.list ? (v ? [v] : undefined) : v)} label={f.label} extended markdown />
    </div>
  {:else if f.input === "bool"}
    <label class="check">
      <!-- Off is left out, as in the packs' data. -->
      <input type="checkbox" checked={!!get(f)} onchange={(e) => set(f, e.currentTarget.checked || undefined)} />
      {f.label}
    </label>
  {:else if f.input === "number"}
    <label class="field short">
      <span>{f.label}{#if f.required}{" "}<span class="muted">({t.required})</span>{/if}</span>
      <input type="number" min={f.min} step={f.step ?? 1} required={f.required} value={textOf(f)} oninput={(e) => set(f, Number.isNaN(e.currentTarget.valueAsNumber) ? undefined : e.currentTarget.valueAsNumber)} />
    </label>
  {:else if f.input === "text"}
    <label class="field">
      <span>{f.label}</span>
      <input value={textOf(f)} maxlength="200" autocomplete="off" list={f.suggest ? `${id}-${f.key}` : undefined} oninput={(e) => set(f, e.currentTarget.value)} />
      {#if f.suggest}
        <datalist id="{id}-{f.key}">
          {#each choicesOf(f) as choice (choice.index)}<option value={choice.name}></option>{/each}
        </datalist>
      {/if}
    </label>
  {:else if f.input === "ref"}
    {@const current = refsOf(f)[0]}
    <label class="field">
      <span>{f.label}</span>
      <select value={current?.index ?? ""} required={f.required} onchange={(e) => set(f, e.currentTarget.value ? refTo(f, e.currentTarget.value) : undefined)}>
        <option value="">{t.none}</option>
        {#if current && !choicesOf(f).some((c) => c.index === current.index)}<option value={current.index}>{current.name}</option>{/if}
        {#each choicesOf(f) as choice (choice.index)}<option value={choice.index}>{choice.name}</option>{/each}
      </select>
    </label>
  {:else if f.input === "refs"}
    {@const chosen = refsOf(f)}
    <div class="field">
      <span id="{id}-{f.key}">{f.label}</span>
      <div class="chips" role="group" aria-labelledby="{id}-{f.key}">
        {#each chosen as ref (ref.index)}
          <span class="chip">
            {ref.name}
            <button class="icon-btn" type="button" aria-label={t.removeRef(ref.name)} onclick={() => set(f, chosen.length > 1 ? chosen.filter((r) => r.index !== ref.index) : undefined)}>
              <i class="ti ti-x" aria-hidden="true"></i>
            </button>
          </span>
        {/each}
        <select
          aria-label={t.add(f.label)}
          value=""
          onchange={(e) => {
            const index = e.currentTarget.value;
            e.currentTarget.value = "";
            if (index) set(f, [...chosen, refTo(f, index)]);
          }}
        >
          <option value="">{t.add(f.label)}</option>
          {#each choicesOf(f).filter((c) => !chosen.some((r) => r.index === c.index)) as choice (choice.index)}<option value={choice.index}>{choice.name}</option>{/each}
        </select>
      </div>
    </div>
  {:else if f.input === "tags"}
    {@const tags = tagsOf(f)}
    <fieldset class="field">
      <legend>{f.label}</legend>
      <div class="chips">
        {#each [...new Set([...choicesOf(f).map((c) => c.name), ...tags])] as tag (tag)}
          <label class="check"><input type="checkbox" checked={tags.includes(tag)} onchange={(e) => toggleTag(f, tag, e.currentTarget.checked)} />{tag}</label>
        {/each}
      </div>
    </fieldset>
  {:else if f.input === "group"}
    <fieldset class="field">
      <legend>{f.label}</legend>
      <div class="group">{#each f.fields as sub (sub.key)}{@render field(sub)}{/each}</div>
    </fieldset>
  {:else if f.input === "bonuses"}
    {@const rows = bonusesOf(f)}
    <div class="field">
      <span id="{id}-{f.key}">{f.label}</span>
      <div class="rows" role="group" aria-labelledby="{id}-{f.key}">
        {#each rows as row, i (row.proficiency.index)}
          <div class="bonus-row">
            <span class="bonus-name">{row.proficiency.name}</span>
            <input
              type="number"
              class="bonus"
              aria-label={t.monster.bonus(row.proficiency.name)}
              value={row.value}
              oninput={(e) => !Number.isNaN(e.currentTarget.valueAsNumber) && set(f, rows.map((r, j) => (j === i ? { ...r, value: e.currentTarget.valueAsNumber } : r)))}
            />
            <button class="icon-btn" type="button" aria-label={t.removeRef(row.proficiency.name)} onclick={() => set(f, rows.length > 1 ? rows.filter((_, j) => j !== i) : undefined)}>
              <i class="ti ti-x" aria-hidden="true"></i>
            </button>
          </div>
        {/each}
        <select
          aria-label={t.monster.addBonus}
          value=""
          onchange={(e) => {
            const index = e.currentTarget.value;
            e.currentTarget.value = "";
            if (index && "of" in f) set(f, [...rows, { value: 0, proficiency: refTo(f, index) }]);
          }}
        >
          <option value="">{t.monster.addBonus}</option>
          <!-- Only the kinds the stat block shows. -->
          {#each choicesOf(f).filter((c) => STAT_BLOCK_PROFICIENCIES.some((k) => c.name.startsWith(`${k}:`)) && !rows.some((r) => r.proficiency.index === c.index)) as choice (choice.index)}<option value={choice.index}>{choice.name}</option>{/each}
        </select>
      </div>
    </div>
  {:else if f.input === "actions" || f.input === "linked"}
    {@render items(actionsOf(f), (list) => set(f, list.length ? list : undefined), f.label, f.input === "actions" ? f.text : undefined, f.input === "actions" && !!f.leveled)}
  {:else if f.input === "levels"}
    {@const rows = levelsOf(f)}
    <div class="field">
      <span>{f.label}</span>
      {#each rows as row, i (row.level)}
        <fieldset class="level">
          <legend>{t.fields.level(row.level)}</legend>
          <div class="level-head">
            <label class="check">
              {t.monster.pb}
              <input
                type="number"
                class="bonus"
                aria-label={t.fields.profBonus(row.level)}
                value={row.prof_bonus ?? ""}
                oninput={(e) => (row.prof_bonus = Number.isNaN(e.currentTarget.valueAsNumber) ? undefined : e.currentTarget.valueAsNumber)}
              />
            </label>
            <button class="icon-btn" type="button" aria-label={t.fields.removeLevel(row.level)} onclick={() => set(f, rows.length > 1 ? rows.filter((_, j) => j !== i) : undefined)}>
              <i class="ti ti-trash" aria-hidden="true"></i>
            </button>
          </div>
          {@render items(row.features, (list) => (row.features = list), t.fields.features(row.level))}
        </fieldset>
      {/each}
      <button class="btn add" type="button" onclick={() => set(f, [...rows, { level: Math.max(0, ...rows.map((r) => r.level)) + 1, features: [] }])}>
        <i class="ti ti-plus" aria-hidden="true"></i>{t.fields.addLevel}
      </button>
    </div>
  {/if}
{/snippet}

<!-- Named texts in order (`list`), moved by their grip; `change` stores the new list. -->
{#snippet items(list: Item[], change: (list: Item[]) => void, label: string, textKey = "desc", leveled = false)}
  <div class="field">
    <span>{label}</span>
    {#if list.length}
      <OrderList class="action-list" items={list.map((a) => ({ id: rowId(a), label: a.name || t.action.untitled, a }))} onmove={(from, to) => change(moveTo(list, from, to))}>
        {#snippet row(item)}
          <div class="action">
            <div class="action-head">
              <input aria-label={t.action.name} placeholder={t.action.name} value={item.a.name} required maxlength="200" autocomplete="off" oninput={(e) => (item.a.name = e.currentTarget.value)} />
              {#if leveled}
                <input
                  type="number"
                  class="bonus"
                  min="1"
                  required
                  aria-label={t.fields.featureLevel(item.label)}
                  placeholder={c.level}
                  value={typeof item.a.level === "number" ? item.a.level : ""}
                  oninput={(e) => (item.a.level = Number.isNaN(e.currentTarget.valueAsNumber) ? undefined : e.currentTarget.valueAsNumber)}
                />
              {/if}
              <button class="icon-btn" type="button" aria-label={t.action.remove(item.label)} onclick={() => change(list.filter((x) => x !== item.a))}>
                <i class="ti ti-trash" aria-hidden="true"></i>
              </button>
            </div>
            <RichText bind:value={() => (typeof item.a[textKey] === "string" ? item.a[textKey] : ""), (v) => (item.a[textKey] = v)} label={t.action.text(item.label)} extended markdown />
          </div>
        {/snippet}
      </OrderList>
    {/if}
    <button class="btn add" type="button" onclick={() => change([...list, { name: "", [textKey]: "" }])}><i class="ti ti-plus" aria-hidden="true"></i>{t.action.add(label)}</button>
  </div>
{/snippet}

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: var(--homebrew-form-width);
    margin-inline: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    border: none;
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  /* The name, with the picture's tile beside it. */
  .head {
    display: flex;
    align-items: flex-end;
    gap: var(--space-4);
  }

  .head > .field {
    flex: 1;
    min-width: 0;
  }

  .field.short {
    max-width: var(--homebrew-number-width);
  }

  .group {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-3) var(--space-4);
  }

  .group > .field {
    flex: 1 1 var(--homebrew-number-width);
  }

  .rows {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }

  .bonus-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: var(--weight-regular);
  }

  .bonus-name {
    min-width: var(--homebrew-number-width);
  }

  .bonus {
    width: var(--homebrew-bonus-width);
  }

  .form :global(.action-list) {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .form :global(.action-list > li) {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
  }

  .action {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .action-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .action-head input {
    flex: 1;
    min-width: 0;
  }

  .add {
    align-self: flex-start;
  }

  .level {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: var(--space-3);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .level-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  legend {
    padding: 0;
    margin-bottom: var(--space-1);
  }

  input:not([type="checkbox"]),
  select {
    min-height: var(--control-height);
    padding: 0 var(--space-3);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    font-weight: var(--weight-regular);
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-4);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding-left: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
    font-weight: var(--weight-regular);
  }

  .confirm {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .end {
    margin-left: auto;
  }

  .small {
    min-height: var(--control-height-sm);
    padding: 0 var(--space-3);
  }

  .muted {
    color: var(--color-text-muted);
    font-weight: var(--weight-regular);
  }

  .error {
    color: var(--color-danger);
  }
</style>
