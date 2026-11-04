<script lang="ts">
  // An on/off light switch with its label. A button with the switch role: Space or Enter flips it, like a click.
  let {
    id,
    label,
    checked,
    disabled = false,
    onchange,
  }: { id: string; label: string; checked: boolean; disabled?: boolean; onchange: (checked: boolean) => void } = $props();
</script>

<div class="switch-row">
  <button {id} type="button" role="switch" class="switch" aria-checked={checked} aria-labelledby="{id}-label" {disabled} onclick={() => onchange(!checked)}>
    <span class="knob"></span>
  </button>
  <label id="{id}-label" for={id}>{label}</label>
</div>

<style>
  .switch-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  label {
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  /* The track is twice as wide as the knob; the knob slides to the right when on. */
  .switch {
    --knob: calc(var(--control-height-sm) - 2 * var(--space-1));
    flex: none;
    display: inline-flex;
    align-items: center;
    width: calc(2 * var(--knob) + 2 * var(--space-1));
    height: var(--control-height-sm);
    padding: 0 var(--space-1);
    border: none;
    border-radius: var(--radius-pill);
    background: var(--color-border-strong);
    cursor: pointer;
    transition: background var(--duration-drawer) ease;
  }

  .knob {
    width: var(--knob);
    height: var(--knob);
    border-radius: var(--radius-pill);
    background: var(--color-surface);
    box-shadow: var(--shadow-card);
    transition: transform var(--duration-drawer) ease;
  }

  .switch[aria-checked="true"] {
    background: var(--color-accent);
  }

  .switch[aria-checked="true"] .knob {
    transform: translateX(var(--knob));
  }

  .switch:disabled {
    cursor: not-allowed;
  }

  :global([data-motion="reduce"]) .switch,
  :global([data-motion="reduce"]) .knob {
    transition: none;
  }
</style>
