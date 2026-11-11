<script lang="ts">
  // Manage locations / Manage NPCs: the campaign's, in tree order, each edited in place with where this session uses it;
  // removing one unlinks it from this session's scenes (what was under it moves up); a new one by name, at the top.
  import type { Session } from "$lib/api/sessions";
  import type { World } from "$lib/api/world";
  import { strings } from "$lib/strings";
  import WorldEntry from "./WorldEntry.svelte";
  import { newLocation, newNpc, removeLocation, removeNpc, scenesWith, treeOrder } from "./sessions";

  let { kind, session = $bindable(), world = $bindable() }: { kind: "location" | "npc"; session: Session; world: World } = $props();

  const t = strings.sessions.manage;
  const rows = $derived(kind === "location" ? treeOrder(world.locations) : treeOrder(world.npcs));
  let draft = $state("");

  function add(ev: SubmitEvent) {
    ev.preventDefault();
    const name = draft.trim();
    if (!name) return;
    if (kind === "location") world.locations.push(newLocation(name));
    else world.npcs.push(newNpc(name));
    draft = "";
  }

  function note(id: string) {
    const where = scenesWith(session, { kind, id }).map((sc) => sc.title || strings.sessions.scenes.untitled);
    if (kind === "location") return where.length ? t.usedIn(where.join(", ")) : t.notUsed;
    return where.length ? t.appearsIn(where.join(", ")) : t.notLinked;
  }

  const remove = (id: string) => (kind === "location" ? removeLocation : removeNpc)(world, session, id);
</script>

<h2>{kind === "location" ? strings.sessions.sections.locations : strings.sessions.sections.npcs}</h2>
{#each rows as r (r.entry.id)}
  <div class="card" style:margin-left="calc({r.depth} * var(--space-5))">
    <WorldEntry {kind} bind:world id={r.entry.id} note={note(r.entry.id)} onremove={remove} />
  </div>
{:else}
  <p class="note">{kind === "location" ? t.noLocations : t.noNpcs}</p>
{/each}
<form class="add" onsubmit={add}>
  <input class="text-field" bind:value={draft} maxlength="100" autocomplete="off" placeholder={kind === "location" ? t.newLocation : t.newNpc} aria-label={kind === "location" ? t.newLocation : t.newNpc} />
  <button class="btn" type="submit" disabled={!draft.trim()}>{kind === "location" ? t.addLocation : t.addNpc}</button>
</form>
