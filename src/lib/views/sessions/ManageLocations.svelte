<script lang="ts">
  // Every location of the session: name and description edited in place, where it's used, remove (clearing it from
  // its scenes); a new one by name.
  import type { Session } from "$lib/api/sessions";
  import { strings } from "$lib/strings";
  import { newLocation, removeLocation, scenesWith } from "./sessions";

  let { session = $bindable() }: { session: Session } = $props();

  const t = strings.sessions.manage;
  const s = strings.sessions.scenes;
  let draft = $state("");

  function add(ev: SubmitEvent) {
    ev.preventDefault();
    const name = draft.trim();
    if (!name) return;
    session.locations.push(newLocation(name));
    draft = "";
  }

  const used = (id: string) => scenesWith(session, { kind: "location", id }).map((sc) => sc.title || s.untitled);
</script>

<h2>{strings.sessions.sections.locations}</h2>
{#each session.locations as l (l.id)}
  {@const where = used(l.id)}
  <div class="card">
    <div class="row">
      <div class="field grow">
        <label for="loc-name-{l.id}" class="small-label">{t.name}</label>
        <input id="loc-name-{l.id}" class="text-field strong" bind:value={l.name} maxlength="100" autocomplete="off" />
      </div>
      <button class="btn icon" aria-label={t.remove(l.name || t.unnamedLocation)} onclick={() => removeLocation(session, l.id)}>
        <i class="ti ti-trash" aria-hidden="true"></i>
      </button>
    </div>
    <div class="field">
      <label for="loc-desc-{l.id}" class="small-label">{s.description}</label>
      <textarea id="loc-desc-{l.id}" class="text-field" rows="2" maxlength="20000" bind:value={l.description}></textarea>
    </div>
    <p class="note">{where.length ? t.usedIn(where.join(", ")) : t.notUsed}</p>
  </div>
{:else}
  <p class="note">{t.noLocations}</p>
{/each}
<form class="add" onsubmit={add}>
  <input class="text-field" bind:value={draft} maxlength="100" autocomplete="off" placeholder={t.newLocation} aria-label={t.newLocation} />
  <button class="btn" type="submit" disabled={!draft.trim()}>{t.addLocation}</button>
</form>
