<script lang="ts">
  import { strings } from "$lib/strings";
  import { licenseTexts, type LicenseText } from "$lib/api/about";
  import { listAddons, type AddonSummary } from "$lib/api/addons";
  import { errorMessage } from "$lib/api/storage";
  import { installNewVersion, update } from "$lib/stores/updates.svelte";
  import { version } from "../../../package.json";

  // `unsaved`: Settings has changes not saved; Update restarts the app, which would lose them.
  let { unsaved }: { unsaved: boolean } = $props();

  const t = strings.settings.about;
  let texts = $state<LicenseText[]>();
  let error = $state<string>();
  let addons = $state<AddonSummary[]>([]);

  $effect(() => {
    licenseTexts()
      .then((list) => (texts = list))
      .catch((e) => (error = errorMessage(e)));
    listAddons()
      .then((list) => (addons = list))
      .catch(() => {}); // Settings → Addons shows why; the app's own license texts still show
  });
</script>

<section>
  <h3>{strings.appName}</h3>
  <p class="muted">{t.version(version)}</p>
  {#if update.latest}
    <div class="update">
      <p>{strings.updateAvailable(update.latest)}</p>
      <button
        class="btn btn-primary"
        disabled={update.installing || unsaved}
        aria-describedby={unsaved ? "about-update-unsaved" : undefined}
        onclick={installNewVersion}
      >
        {update.installing ? t.updating : t.update}
      </button>
    </div>
    {#if unsaved}<p id="about-update-unsaved" class="muted">{strings.settings.addons.restartUnsaved}</p>{/if}
  {/if}
  {#if update.installError}
    <p class="error" role="alert">{t.installError(update.installError)}</p>
  {:else if update.checkError}
    <p class="muted">{t.checkError(update.checkError)}</p>
  {/if}
</section>

<section>
  <h3>{t.licenses}</h3>
  <dl>
    {#each t.licenseList as item (item.part)}
      <dt>{item.part}</dt>
      <dd>{item.license}</dd>
    {/each}
  </dl>
</section>

<section>
  <h3>{t.attributions}</h3>
  <p>{t.srdAttribution}</p>
  <p>{t.srdModified}</p>
  <p>{t.srdApi}</p>
  <p>{t.tabler}</p>
</section>

<section>
  <h3>{t.texts}</h3>
  {#if error}
    <p class="error" role="alert">{t.loadError(error)}</p>
  {:else if !texts}
    <p class="muted" role="status">{t.loading}</p>
  {:else}
    {#each texts as license (license.id)}
      <details>
        <summary>{t.textNames[license.id] ?? license.id}</summary>
        <pre>{license.text}</pre>
      </details>
    {/each}
    {#each addons.filter((a) => a.licenseText) as addon (addon.id)}
      <details>
        <summary>{addon.name}</summary>
        <pre>{addon.licenseText}</pre>
      </details>
    {/each}
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  h3 {
    font-size: var(--text-md);
    font-weight: var(--weight-semibold);
  }

  p {
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
    overflow-wrap: anywhere;
  }

  .muted {
    color: var(--color-text-muted);
  }

  .update {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-1) var(--space-4);
    font-size: var(--text-sm);
  }

  dt {
    font-weight: var(--weight-medium);
  }

  summary {
    padding: var(--space-1) 0;
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  pre {
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .error {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
  }
</style>
