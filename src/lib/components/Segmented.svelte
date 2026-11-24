<script lang="ts" generics="T extends string">
  // One of a few options, side by side (a radio group): click one, or move with the arrow keys (Tab reaches the chosen
  // one only). `disabled(value)` greys an option out.
  import { tick } from "svelte";

  let {
    label,
    options,
    value,
    onchange,
    disabled = () => false,
  }: { label: string; options: { value: T; label: string }[]; value: T; onchange: (value: T) => void; disabled?: (value: T) => boolean } = $props();
  let group: HTMLDivElement | undefined = $state();
  // The option Tab reaches: the chosen one, or the first one not greyed out when it is.
  const current = $derived(options.find((o) => o.value === value && !disabled(o.value))?.value ?? options.find((o) => !disabled(o.value))?.value);

  async function onkeydown(event: KeyboardEvent, i: number) {
    const step = ["ArrowRight", "ArrowDown"].includes(event.key) ? 1 : ["ArrowLeft", "ArrowUp"].includes(event.key) ? -1 : 0;
    if (!step) return;
    event.preventDefault();
    for (let k = 1; k < options.length; k++) {
      const at = (i + step * k + options.length) % options.length;
      if (disabled(options[at].value)) continue;
      onchange(options[at].value);
      await tick();
      group?.querySelectorAll<HTMLButtonElement>("button")[at]?.focus();
      return;
    }
  }
</script>

<div class="segmented" role="radiogroup" aria-label={label} bind:this={group}>
  {#each options as option, i (option.value)}
    <button
      type="button"
      role="radio"
      aria-checked={option.value === value}
      tabindex={option.value === current ? 0 : -1}
      disabled={disabled(option.value)}
      onclick={() => onchange(option.value)}
      onkeydown={(e) => onkeydown(e, i)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    border: var(--border-width) solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  button {
    flex: 1 1 0;
    min-height: var(--control-height-sm);
    padding: 0 var(--space-2);
    border: none;
    background: transparent;
    color: var(--color-text);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    white-space: nowrap;
    cursor: pointer;
  }

  button + button {
    border-left: var(--border-width) solid var(--color-border);
  }

  button:hover:not(:disabled, [aria-checked="true"]) {
    background: var(--color-surface-sunken);
  }

  button[aria-checked="true"] {
    background: var(--color-accent);
    color: var(--color-accent-text);
  }

  button:disabled {
    color: var(--color-text-muted);
    cursor: not-allowed;
  }

  /* Inside the group's edge, which clips anything outside it. */
  button:focus-visible {
    outline-offset: calc(-2 * var(--focus-ring));
  }
</style>
