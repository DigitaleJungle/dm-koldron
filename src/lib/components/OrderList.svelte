<script lang="ts" generics="T extends { id: string; label: string }">
  // A list whose rows move by a grip: drag it onto another row, or press the up and down arrow keys on it (the focus
  // stays on the moved row's grip). `row` draws a row's content; the parent styles rows and the grip with :global.
  import type { Snippet } from "svelte";
  import { tick } from "svelte";
  import type { HTMLAttributes } from "svelte/elements";
  import { strings } from "$lib/strings";

  let {
    items,
    row,
    onmove,
    ...rest
  }: { items: T[]; row: Snippet<[T]>; onmove: (from: number, to: number) => void } & HTMLAttributes<HTMLUListElement> = $props();
  let list: HTMLUListElement | undefined = $state();
  let dragging = $state<number | null>(null);
  let over = $state<number | null>(null);

  async function onGripKey(event: KeyboardEvent, i: number, id: string) {
    const to = event.key === "ArrowUp" ? i - 1 : event.key === "ArrowDown" ? i + 1 : null;
    if (to === null) return;
    event.preventDefault();
    onmove(i, to);
    await tick();
    list?.querySelector<HTMLElement>(`[data-grip="${CSS.escape(id)}"]`)?.focus();
  }
</script>

<ul bind:this={list} {...rest}>
  {#each items as item, i (item.id)}
    <li
      class:dragging={dragging === i}
      class:over={over === i && dragging !== null && dragging !== i}
      ondragover={(ev) => {
        if (dragging === null) return;
        ev.preventDefault();
        over = i;
      }}
      ondrop={(ev) => {
        ev.preventDefault();
        if (dragging !== null) onmove(dragging, i);
        dragging = over = null;
      }}
    >
      {@render row(item)}
      <button
        class="icon-btn grip"
        data-grip={item.id}
        draggable="true"
        aria-label={strings.campaign.move(item.label)}
        onkeydown={(ev) => onGripKey(ev, i, item.id)}
        ondragstart={(ev) => {
          dragging = i;
          ev.dataTransfer?.setData("text/plain", item.id);
          if (ev.dataTransfer) ev.dataTransfer.effectAllowed = "move";
        }}
        ondragend={() => (dragging = over = null)}
      >
        <i class="ti ti-grip-vertical" aria-hidden="true"></i>
      </button>
    </li>
  {/each}
</ul>

<style>
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li.dragging {
    opacity: 0.5;
  }

  /* Where a dragged row will land. */
  li.over {
    outline: var(--border-width) solid var(--color-accent);
  }

  .grip {
    cursor: grab;
  }
</style>
