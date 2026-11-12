<script lang="ts">
  // A scene's location: its first image (or its map) as a banner, name, where it sits, and its atmosphere; the pencil
  // opens it in the location editor, the cross unlinks it.
  import { pictureSrc } from "$lib/api/campaigns";
  import type { Location } from "$lib/api/world";
  import { cleanHtml, isEmpty } from "$lib/components/richtext";
  import "$lib/styles/prose.css";
  import { strings } from "$lib/strings";

  let { location, path, onedit, onremove }: { location: Location; /** Its parents, top first. */ path: string[]; onedit: () => void; onremove: () => void } =
    $props();

  const name = $derived(location.name || strings.sessions.manage.unnamedLocation);
  const banner = $derived(location.pictures?.[0] ?? location.map ?? null);
</script>

<article class="link-card location">
  {#if banner}<img class="banner" src={pictureSrc(banner)} alt="" />{/if}
  <div class="text">
    <div class="title">
      <h4>{name}</h4>
      <button class="icon-btn" data-edit={location.id} aria-label={strings.sessions.world.edit(name)} onclick={onedit}>
        <i class="ti ti-pencil" aria-hidden="true"></i>
      </button>
      <button class="icon-btn" aria-label={strings.sessions.scenes.unlink(name)} onclick={onremove}>
        <i class="ti ti-x" aria-hidden="true"></i>
      </button>
    </div>
    {#if path.length}<p class="muted">{path.join(" › ")}</p>{/if}
    {#if !isEmpty(location.atmosphere)}<div class="prose atmosphere">{@html cleanHtml(location.atmosphere)}</div>{/if}
  </div>
</article>

<style>
  .location {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .banner {
    width: 100%;
    height: var(--link-card-banner-height);
    object-fit: cover;
    border-bottom: var(--border-width) solid var(--color-border);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3);
  }

  /* The start of it: the editor has the rest. */
  .atmosphere {
    max-width: none;
    margin: 0;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    font: var(--text-body-sm);
  }

  .atmosphere > :global(*) {
    margin-block: 0;
  }
</style>
