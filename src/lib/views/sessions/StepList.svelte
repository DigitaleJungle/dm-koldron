<script lang="ts">
  // The session's steps in order, each a numbered hexagon (filled once ready) and its title (the kind until it has one);
  // a trash icon per step (asking first, in its row, when the step has something in it); and Add step, a dropdown of
  // the kinds.
  import { tick } from "svelte";
  import Menu from "$lib/components/Menu.svelte";
  import type { Step, StepKind } from "$lib/api/sessions";
  import { strings } from "$lib/strings";
  import { hasItems, KINDS } from "./sessions";

  let {
    steps,
    selected,
    onselect,
    onadd,
    onremove,
  }: {
    steps: Step[];
    selected: string | null;
    onselect: (id: string) => void;
    onadd: (kind: StepKind) => void;
    onremove: (id: string) => void;
  } = $props();

  const t = strings.sessions.steps;
  const kindItems = KINDS.map((k) => ({ label: t.kinds[k].label, onselect: () => onadd(k) }));
  const nameOf = (step: Step) => step.title || t.kinds[step.kind].label;

  /** The step whose removal waits for a yes. */
  let confirming = $state<string | null>(null);
  let list: HTMLOListElement | undefined = $state();

  async function focusIn(id: string, selector: string) {
    await tick();
    list?.querySelector<HTMLElement>(`[data-step="${id}"] ${selector}`)?.focus();
  }

  function askRemove(step: Step) {
    if (!hasItems(step)) return onremove(step.id);
    confirming = step.id;
    focusIn(step.id, ".keep");
  }

  function keep(id: string) {
    confirming = null;
    focusIn(id, ".remove");
  }
</script>

<nav aria-label={t.title}>
  <ol bind:this={list}>
    {#each steps as step, i (step.id)}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <li data-step={step.id} onkeydown={(ev) => ev.key === "Escape" && confirming === step.id && keep(step.id)}>
        <button class="step" class:selected={step.id === selected} aria-current={step.id === selected} onclick={() => onselect(step.id)}>
          <span class="hex" class:ready={step.ready}>{i + 1}</span>
          <span class="title" class:untitled={!step.title}>{nameOf(step)}</span>
          {#if step.ready}<i class="ti ti-check ready-mark" role="img" aria-label={t.ready}></i>{/if}
        </button>
        {#if confirming === step.id}
          <button class="icon-btn danger" aria-label={t.confirmRemove(nameOf(step))} onclick={() => ((confirming = null), onremove(step.id))}>
            <i class="ti ti-trash" aria-hidden="true"></i>
          </button>
          <button class="icon-btn keep" aria-label={t.keep(nameOf(step))} onclick={() => keep(step.id)}>
            <i class="ti ti-x" aria-hidden="true"></i>
          </button>
        {:else}
          <button class="icon-btn remove" aria-label={t.remove(nameOf(step))} onclick={() => askRemove(step)}>
            <i class="ti ti-trash" aria-hidden="true"></i>
          </button>
        {/if}
      </li>
    {/each}
  </ol>

  <Menu id="add-step" label={t.add} items={kindItems} align="start" triggerClass="add-step">
    {#snippet trigger()}
      <i class="ti ti-plus" aria-hidden="true"></i>
      {t.add}
    {/snippet}
  </Menu>
</nav>

<style>
  nav {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  /* The trash shows on the row's hover or focus, or while it asks. */
  li .remove {
    opacity: 0;
  }

  li:hover .remove,
  li:focus-within .remove {
    opacity: 1;
  }

  .danger {
    color: var(--color-danger);
  }

  .step {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    min-height: var(--control-height-lg);
    padding: var(--space-1) var(--space-3);
    border: var(--border-width) solid transparent;
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text);
    text-align: left;
    cursor: pointer;
  }

  .step:hover {
    background: var(--color-surface-sunken);
  }

  .step.selected {
    border-color: var(--color-accent);
    background: var(--color-accent-soft);
  }

  .hex {
    display: grid;
    place-items: center;
    flex: none;
    width: var(--roster-badge-width);
    height: var(--roster-badge-height);
    background: var(--color-border-strong);
    color: var(--color-text);
    font: var(--text-dice);
    clip-path: polygon(25% 0, 75% 0, 100% 50%, 75% 100%, 25% 100%, 0 50%);
  }

  .hex.ready {
    background: var(--potion-glow);
    color: var(--charcoal);
  }

  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: var(--weight-bold);
  }


  .ready-mark {
    color: var(--color-success);
    font-size: var(--icon-size);
  }

  .untitled {
    color: var(--color-text-muted);
    font-weight: var(--weight-normal);
  }

  nav :global(.add-step) {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    min-height: var(--control-height-lg);
    margin-top: var(--space-1);
    padding: 0 var(--space-3);
    border: var(--border-width) dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text);
    font: var(--text-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    cursor: pointer;
  }

  nav :global(.add-step:hover) {
    background: var(--color-surface-sunken);
  }
</style>
