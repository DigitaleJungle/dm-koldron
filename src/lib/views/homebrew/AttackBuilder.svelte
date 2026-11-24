<script lang="ts">
  // A monster action written as an attack: its kind, the ability it uses, reach or range, damage dice and type, and an
  // extra sentence. The form writes the attack's text (shown under it) into the action.
  import type { Choice } from "$lib/api/homebrew";
  import { strings } from "$lib/strings";
  import { title } from "$lib/views/library/catalog";
  import { ABILITIES } from "$lib/views/library/entries/format";
  import type { Attack } from "./monster";

  let { attack = $bindable(), text, damageTypes }: { attack: Attack; text: string; damageTypes: Choice[] } = $props();
  const m = strings.homebrew.monster;
  const e = strings.entry;
</script>

<div class="attack">
  <label class="field">
    <span>{m.kind}</span>
    <select bind:value={attack.kind}>
      {#each ["melee", "ranged", "both"] as const as kind (kind)}<option value={kind}>{m.kinds[kind]}</option>{/each}
    </select>
  </label>
  <label class="field">
    <span>{m.uses}</span>
    <select bind:value={attack.ability}>
      {#each ABILITIES as ability (ability)}<option value={ability}>{e.abilities[ability]}</option>{/each}
    </select>
  </label>
  <label class="field">
    <span>{m.reach}</span>
    <input bind:value={attack.reach} maxlength="100" autocomplete="off" />
  </label>
  <label class="field">
    <span>{m.dice}</span>
    <input class="dice" bind:value={attack.dice} maxlength="50" autocomplete="off" />
  </label>
  <label class="field">
    <span>{m.damageType}</span>
    <select bind:value={attack.type}>
      {#if attack.type && !damageTypes.some((c) => c.index === attack.type)}<option value={attack.type}>{title(attack.type)}</option>{/if}
      {#each damageTypes as choice (choice.index)}<option value={choice.index}>{title(choice.name)}</option>{/each}
    </select>
  </label>
  <label class="field full">
    <span>{m.extra}</span>
    <input bind:value={attack.extra} maxlength="1000" autocomplete="off" />
  </label>
  <p class="line full">{text}</p>
</div>

<style>
  .attack {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(var(--homebrew-number-width) * 0.75), 1fr));
    gap: var(--space-2) var(--space-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
    font-size: var(--text-xs);
    color: var(--color-text-muted);
  }

  .full {
    grid-column: 1 / -1;
  }

  input,
  select {
    min-height: var(--control-height);
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: var(--text-sm);
  }

  input {
    padding: 0 var(--space-2);
  }

  .line {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border-left: var(--border-width-thick) solid var(--color-accent);
    background: var(--color-surface);
    font-size: var(--text-sm);
  }
</style>
