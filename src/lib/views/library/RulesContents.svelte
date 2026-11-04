<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";
  import { strings } from "$lib/strings";
  import type { ContentsChapter } from "$lib/api/rules";

  let {
    contents,
    current,
    section,
    onopen,
  }: {
    contents: ContentsChapter[];
    current: ContentsChapter | undefined;
    section: string | null;
    onopen: (chapter: ContentsChapter, section?: string) => void;
  } = $props();

  const t = strings.library;
  const key = (c: ContentsChapter) => `${c.pack}/${c.index}`;
  const expanded = new SvelteSet<string>();
  // Pack names only help when there's more than one pack.
  const groups = $derived(
    [...new Set(contents.map((c) => c.pack))].map((pack) => ({ pack, chapters: contents.filter((c) => c.pack === pack) })),
  );

  // The open chapter shows its sections.
  $effect(() => {
    if (current) expanded.add(key(current));
  });

  function toggle(c: ContentsChapter) {
    if (expanded.has(key(c))) expanded.delete(key(c));
    else expanded.add(key(c));
  }
</script>

<nav aria-label={t.contents}>
  {#each groups as group (group.pack)}
    {#if groups.length > 1}<p class="label pack">{group.chapters[0].packName}</p>{/if}
    <ul>
      {#each group.chapters as chapter (key(chapter))}
        {@const open = expanded.has(key(chapter))}
        {@const isCurrent = current && key(current) === key(chapter)}
        <li>
          <div class="row">
            {#if chapter.sections.length > 0}
              <button class="toggle" aria-expanded={open} aria-label={t.showSections(chapter.name)} onclick={() => toggle(chapter)}>
                <i class="ti {open ? 'ti-chevron-down' : 'ti-chevron-right'}" aria-hidden="true"></i>
              </button>
            {:else}
              <span class="toggle" aria-hidden="true"></span>
            {/if}
            <button class="item chapter" aria-current={isCurrent && !section ? "location" : undefined} onclick={() => onopen(chapter)}>
              {chapter.name}
            </button>
          </div>
          {#if open}
            <ul class="sections">
              {#each chapter.sections as s (s.index)}
                <li>
                  <button class="item" aria-current={isCurrent && section === s.index ? "location" : undefined} onclick={() => onopen(chapter, s.index)}>
                    {s.name}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </li>
      {/each}
    </ul>
  {/each}
</nav>

<style>
  nav {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .pack {
    padding: var(--space-2) var(--space-2) 0;
  }

  .row {
    display: flex;
    align-items: center;
  }

  .toggle {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--control-height-sm);
    height: var(--control-height-sm);
    padding: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .item {
    flex: 1;
    min-width: 0;
    width: 100%;
    padding: var(--space-1) var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .chapter {
    font-weight: var(--weight-semibold);
  }

  .toggle:hover,
  .item:hover {
    background: var(--color-surface-sunken);
  }

  .item[aria-current="location"] {
    background: var(--color-accent-soft);
    color: var(--color-link);
  }

  .sections {
    padding-left: var(--control-height-sm);
  }
</style>
