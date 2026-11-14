<script lang="ts">
  // Home's Party: the party as a hexagon collage (as on the Characters screen), each hexagon with a small cross that
  // takes that character out of the party. Hovering the spot the next hexagon would take shows a dashed hexagon with a
  // plus: it lists the campaign's other characters to put in. The party is the same one the Characters screen's party
  // button sets. The characters are bound to Home, which loads them and hands them to the Random encounter widget too.
  import { pictureSrc } from "$lib/api/campaigns";
  import { saveCharacter, type Character } from "$lib/api/characters";
  import { errorMessage } from "$lib/api/storage";
  import Menu from "$lib/components/Menu.svelte";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";
  import HexCollage from "$lib/components/HexCollage.svelte";

  const t = strings.home.party;
  const campaign = campaigns.current!.id;

  // Loaded by Home (so Random encounter has them with Party taken off Home too); null while loading.
  let { characters = $bindable(), loadError }: { characters: Character[] | null; loadError: string | null } = $props();
  let error = $state<string | null>(null);

  const byName = (a: Character, b: Character) => a.name.localeCompare(b.name);
  const party = $derived((characters ?? []).filter((c) => c.onHome).toSorted(byName));
  const others = $derived((characters ?? []).filter((c) => !c.onHome).toSorted(byName));

  let busy = $state(false);
  async function setParty(character: Character, onHome: boolean) {
    [busy, error] = [true, null];
    try {
      const saved = await saveCharacter(campaign, { ...character, onHome });
      characters = (characters ?? []).map((c) => (c.id === saved.id ? saved : c));
    } catch (e) {
      error = t.error(character.name, errorMessage(e));
    } finally {
      busy = false;
    }
  }
</script>

{#if loadError}
  <p class="error" role="alert">{loadError}</p>
{:else if characters === null}
  <p class="muted" role="status">{strings.home.loading}</p>
{:else}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if party.length === 0 && others.length === 0}
    <p class="muted">{t.empty}</p>
  {:else}
    <div class="collage" role="group" aria-label={strings.home.titles.party}>
      <HexCollage
        tiles={party.map((c) => ({ key: c.id, picture: c.picture ? pictureSrc(c.picture) : null, letter: c.name.charAt(0) }))}
        seed={campaign}
        hexWidth="var(--home-party-hex-width)"
        maxRows={2.5}
        disabled={busy}
        removeLabel={(tile) => t.remove(party.find((c) => c.id === tile.key)?.name ?? tile.letter)}
        onremove={(cid) => {
          const c = party.find((p) => p.id === cid);
          if (c) void setParty(c, false);
        }}
        add={others.length > 0 ? addSpot : undefined}
      />
    </div>
  {/if}
{/if}

<!-- The spot for the next character: a dashed hexagon with a plus, opening the list of the others. -->
{#snippet addSpot()}
  <Menu id="party-add" label={t.addTo} align="start" triggerClass="add-hex" items={others.map((c) => ({ label: c.name, onselect: () => setParty(c, true) }))}>
    {#snippet trigger()}
      <svg viewBox="-0.02 -0.02 1.04 0.906" aria-hidden="true"><polygon points="0.25,0 0.75,0 1,0.433 0.75,0.866 0.25,0.866 0,0.433" /></svg>
      <i class="ti ti-plus" aria-hidden="true"></i>
    {/snippet}
  </Menu>
{/snippet}

<style>
  p {
    margin: 0;
  }

  /* Two and a half hexagons high at most (as the Characters screen's band), bigger hexagons; it grows sideways. */
  .collage {
    --home-party-hex-width: calc(1.4 * var(--party-hex-width));
    display: flex;
    justify-content: center;
    height: calc(2.44 * var(--home-party-hex-width));
  }

  /* The add spot fills its hexagon's box (Menu's root and its button are Menu's own elements). */
  .collage :global(.menu-root) {
    flex: 1;
    display: flex;
  }

  .collage :global(.add-hex) {
    position: relative;
    flex: 1;
    display: grid;
    place-items: center;
    padding: 0;
    border: 0;
    background: none;
    color: var(--color-text-muted);
    font-size: var(--icon-size);
    cursor: pointer;
  }

  .collage :global(.add-hex svg) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .collage :global(.add-hex polygon) {
    fill: none;
    stroke: currentColor;
    stroke-width: 0.02;
    stroke-dasharray: 0.06 0.04;
  }

  .collage :global(.add-hex:hover),
  .collage :global(.add-hex[aria-expanded="true"]) {
    color: var(--color-accent);
  }
</style>
