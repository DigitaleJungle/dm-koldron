<script lang="ts">
  import BackButton from "$lib/components/BackButton.svelte";
  import { onDestroy, type Snippet } from "svelte";
  import { page } from "$app/state";
  import { openCampaign } from "$lib/api/campaigns";
  import { errorMessage } from "$lib/api/storage";
  import { campaigns } from "$lib/stores/campaigns.svelte";
  import { strings } from "$lib/strings";

  let { children }: { children: Snippet } = $props();

  let error = $state<string | null>(null);
  const campaignId = $derived(page.params.id ?? "");

  // Opening marks the campaign as opened now (Start lists the most recent first).
  $effect(() => {
    const id = campaignId;
    error = null;
    let current = true; // ignore a slow answer for a campaign we've already left
    openCampaign(id).then(
      (c) => current && (campaigns.current = c),
      (e) => {
        if (!current) return;
        campaigns.current = null;
        error = strings.campaign.openError(errorMessage(e));
      },
    );
    return () => {
      current = false;
    };
  });

  // Leaving the campaign screens altogether.
  onDestroy(() => {
    campaigns.current = null;
  });
</script>

<!-- Pages under this layout only render once the campaign is loaded. -->
{#if campaigns.current}
  {@render children()}
{:else}
  <main class="status">
    {#if error}
      <p class="error" role="alert">{error}</p>
      <BackButton />
    {:else}
      <p role="status">{strings.campaign.loading}</p>
    {/if}
  </main>
{/if}

<style>
  .status {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-7);
  }

  .error {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-size: var(--text-sm);
  }
</style>
