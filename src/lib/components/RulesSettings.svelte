<script lang="ts">
  import { strings } from "$lib/strings";
  import { listPacks, type PackSummary } from "$lib/api/packs";
  import { getSettings, setSettings } from "$lib/api/settings";
  import { errorMessage } from "$lib/api/storage";
  import Switch from "$lib/components/Switch.svelte";

  const t = strings.settings.rules;
  let packs = $state<PackSummary[]>();
  let loadError = $state<string>();
  let saveError = $state<string>();
  let saving = $state(false);

  $effect(() => {
    listPacks()
      .then((list) => (packs = list))
      .catch((e) => (loadError = errorMessage(e)));
  });

  async function toggle(pack: PackSummary, on: boolean) {
    saving = true;
    saveError = undefined;
    try {
      // From the saved list, so packs not loaded right now keep their setting.
      const saved = (await getSettings()).disabledPacks;
      await setSettings({ disabledPacks: on ? saved.filter((id) => id !== pack.id) : [...new Set([...saved, pack.id])] });
      pack.enabled = on;
    } catch (e) {
      saveError = t.saveError(pack.name, errorMessage(e));
    } finally {
      saving = false;
    }
  }
</script>

{#if loadError}
  <p class="error" role="alert">{t.loadError(loadError)}</p>
{:else if !packs}
  <p class="muted" role="status">{t.loading}</p>
{:else if packs.length === 0}
  <p class="muted">{t.empty}</p>
{:else}
  {#if saveError}<p class="error" role="alert">{saveError}</p>{/if}
  <ul>
    {#each packs as pack (pack.id)}
      <li>
        {#if pack.errors.length === 0}
          <Switch id="pack-{pack.id}" label={pack.name} checked={pack.enabled} disabled={saving} onchange={(on) => toggle(pack, on)} />
          <p class="muted">{t.details(pack.version, pack.license, pack.entries)}</p>
        {:else}
          <p class="name">{pack.name}</p>
          <p class="invalid">{t.invalid}</p>
          <ul class="errors">
            {#each pack.errors as error, i (i)}
              <li>{error}</li>
            {/each}
          </ul>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    list-style: none;
  }

  li {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  p {
    font-size: var(--text-sm);
  }

  .name {
    font-size: var(--text-md);
    font-weight: var(--weight-medium);
  }

  .muted {
    color: var(--color-text-muted);
  }

  .invalid {
    color: var(--color-danger);
    font-weight: var(--weight-medium);
  }

  .errors {
    gap: var(--space-1);
    padding-left: var(--space-4);
    list-style: disc;
    font-size: var(--text-sm);
    color: var(--color-text-muted);
  }

  .error {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
  }
</style>
