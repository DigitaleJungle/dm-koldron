<script lang="ts">
  // A short list of one-line texts: each edited in place (emptied, it goes), removed with a button, one added below.
  // `onchange` gets the whole new list; the caller keeps it (and saves it, when that is the point).
  import { strings } from "$lib/strings";

  let { label, lines, onchange }: { label: string; lines: string[]; onchange: (lines: string[]) => void } = $props();

  const t = strings.sessions.aside;
  let draft = $state("");

  function add(e: SubmitEvent) {
    e.preventDefault();
    const text = draft.trim();
    if (!text) return;
    onchange([...lines, text]);
    draft = "";
  }

  function edit(i: number, value: string) {
    const text = value.trim();
    onchange(text ? lines.map((l, j) => (j === i ? text : l)) : lines.filter((_, j) => j !== i));
  }
</script>

<ul>
  {#each lines as line, i (i)}
    <li>
      <input class="text-field" value={line} maxlength="200" autocomplete="off" aria-label={t.lineEdit(label)} onchange={(e) => edit(i, e.currentTarget.value)} />
      <button class="icon-btn" aria-label={t.lineRemove(line)} onclick={() => onchange(lines.filter((_, j) => j !== i))}>
        <i class="ti ti-trash" aria-hidden="true"></i>
      </button>
    </li>
  {/each}
</ul>
<form onsubmit={add}>
  <input class="text-field" bind:value={draft} maxlength="200" autocomplete="off" aria-label={label} placeholder={label} />
  <button class="btn" type="submit" disabled={!draft.trim()}>{t.lineAdd}</button>
</form>

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  li,
  form {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  form {
    gap: var(--space-2);
  }

  input {
    flex: 1;
    min-width: 0;
  }

  li input {
    border-color: transparent;
    background: none;
  }

  li input:hover,
  li input:focus {
    border-color: var(--color-border-strong);
    background: var(--color-surface);
  }
</style>
