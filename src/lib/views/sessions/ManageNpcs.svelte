<script lang="ts">
  // Every NPC of the session: name, what they want and their voice edited in place, where they appear, remove
  // (unlinking them from every scene); a new one by name.
  import type { Session } from "$lib/api/sessions";
  import { strings } from "$lib/strings";
  import { newNpc, removeNpc, scenesWith } from "./sessions";

  let { session = $bindable() }: { session: Session } = $props();

  const t = strings.sessions.manage;
  let draft = $state("");

  function add(ev: SubmitEvent) {
    ev.preventDefault();
    const name = draft.trim();
    if (!name) return;
    session.npcs.push(newNpc(name));
    draft = "";
  }

  const appears = (id: string) => scenesWith(session, { kind: "npc", id }).map((sc) => sc.title || strings.sessions.scenes.untitled);
</script>

<h2>{strings.sessions.sections.npcs}</h2>
{#each session.npcs as n (n.id)}
  {@const where = appears(n.id)}
  <div class="card">
    <div class="row">
      <div class="field grow">
        <label for="npc-name-{n.id}" class="small-label">{t.name}</label>
        <input id="npc-name-{n.id}" class="text-field strong" bind:value={n.name} maxlength="100" autocomplete="off" />
      </div>
      <button class="btn icon" aria-label={t.remove(n.name || t.unnamedNpc)} onclick={() => removeNpc(session, n.id)}>
        <i class="ti ti-trash" aria-hidden="true"></i>
      </button>
    </div>
    <div class="pair">
      <div class="field">
        <label for="npc-wants-{n.id}" class="small-label">{t.wants}</label>
        <input id="npc-wants-{n.id}" class="text-field" bind:value={n.wants} maxlength="200" autocomplete="off" />
      </div>
      <div class="field">
        <label for="npc-voice-{n.id}" class="small-label">{t.voice}</label>
        <input id="npc-voice-{n.id}" class="text-field" bind:value={n.voice} maxlength="200" autocomplete="off" />
      </div>
    </div>
    <p class="note">{where.length ? t.appearsIn(where.join(", ")) : t.notLinked}</p>
  </div>
{:else}
  <p class="note">{t.noNpcs}</p>
{/each}
<form class="add" onsubmit={add}>
  <input class="text-field" bind:value={draft} maxlength="100" autocomplete="off" placeholder={t.newNpc} aria-label={t.newNpc} />
  <button class="btn" type="submit" disabled={!draft.trim()}>{t.addNpc}</button>
</form>
