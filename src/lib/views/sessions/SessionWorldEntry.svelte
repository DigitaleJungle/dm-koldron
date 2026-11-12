<script lang="ts">
  // A location or NPC in the full editor (as on the Sessions overview), with where this session uses it and a cross back
  // to the scene. Removing one unlinks it from this session's scenes (what was under it moves up).
  import type { Session } from "$lib/api/sessions";
  import type { World } from "$lib/api/world";
  import { strings } from "$lib/strings";
  import WorldEntry from "./WorldEntry.svelte";
  import { removeLocation, removeNpc, scenesWith } from "./sessions";

  let {
    kind,
    session = $bindable(),
    world = $bindable(),
    id,
    onclose,
  }: {
    kind: "location" | "npc";
    session: Session;
    world: World;
    /** The location or NPC open. */
    id: string;
    onclose: () => void;
  } = $props();

  const t = strings.sessions.manage;
  const exists = $derived((kind === "location" ? world.locations : world.npcs).some((e) => e.id === id));

  function note(id: string) {
    const where = scenesWith(session, { kind, id }).map((sc) => sc.title || strings.sessions.scenes.untitled);
    if (kind === "location") return where.length ? t.usedIn(where.join(", ")) : t.notUsed;
    return where.length ? t.appearsIn(where.join(", ")) : t.notLinked;
  }

  function remove(id: string) {
    (kind === "location" ? removeLocation : removeNpc)(world, session, id);
    onclose();
  }
</script>

{#if exists}
  <WorldEntry {kind} bind:world {id} note={note(id)} onremove={remove}>
    {#snippet end()}
      <button class="icon-btn icon-btn-lg" aria-label={strings.sessions.world.close} onclick={onclose}>
        <i class="ti ti-x" aria-hidden="true"></i>
      </button>
    {/snippet}
  </WorldEntry>
{/if}
