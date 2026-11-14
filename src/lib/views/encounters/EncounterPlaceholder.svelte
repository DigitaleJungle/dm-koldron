<script lang="ts">
  // No encounters yet: a dashed card the size of an encounter card, opening a new encounter. On the Encounters
  // screen and Home's Random encounter.
  import { strings } from "$lib/strings";

  let { base }: { base: string } = $props();
</script>

<a class="placeholder" href="{base}/new">
  <i class="ti ti-plus" aria-hidden="true"></i>
  {strings.encounters.create}
</a>

<style>
  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    min-height: var(--encounter-placeholder-height);
    padding: 0 var(--space-6);
    text-align: center;
    border: var(--border-width) dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    color: var(--color-text-muted);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    text-decoration: none;
  }

  .placeholder:hover {
    border-color: var(--color-text);
    color: var(--color-text);
  }

  .placeholder i {
    font-size: var(--text-xl);
  }
</style>
