<script lang="ts">
  // An NPC linked to a scene: portrait (or its initial), name and details; the pencil opens it in the NPC editor, the
  // cross unlinks it.
  import { pictureSrc } from "$lib/api/campaigns";
  import type { Npc } from "$lib/api/world";
  import { strings } from "$lib/strings";
  import { npcSpecs } from "./sessions";

  let { npc, onedit, onremove }: { npc: Npc; onedit: () => void; onremove: () => void } = $props();

  const name = $derived(npc.name || strings.sessions.manage.unnamedNpc);
</script>

<article class="link-card npc">
  <div class="portrait" aria-hidden="true">
    {#if npc.portrait}<img src={pictureSrc(npc.portrait)} alt="" />{:else}{name.charAt(0)}{/if}
  </div>
  <div class="text">
    <div class="title">
      <h4>{name}</h4>
      <button class="icon-btn" data-edit={npc.id} aria-label={strings.sessions.world.edit(name)} onclick={onedit}>
        <i class="ti ti-pencil" aria-hidden="true"></i>
      </button>
      <button class="icon-btn" aria-label={strings.sessions.scenes.unlink(name)} onclick={onremove}>
        <i class="ti ti-x" aria-hidden="true"></i>
      </button>
    </div>
    {#if npcSpecs(npc)}<p class="muted">{npcSpecs(npc)}</p>{/if}
  </div>
</article>

<style>
  .npc {
    display: flex;
    gap: var(--space-3);
    padding: var(--space-3);
  }

  .portrait {
    flex: none;
    display: grid;
    place-items: center;
    width: var(--link-card-portrait-size);
    height: var(--link-card-portrait-size);
    overflow: hidden;
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
    color: var(--color-text-muted);
    font: var(--text-body);
    font-weight: var(--weight-bold);
  }

  .portrait img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
</style>
