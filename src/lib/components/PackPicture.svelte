<script lang="ts">
  // A pack's picture (an entry's, chapter's or section's `image`), floated right of the text; nothing while it loads or
  // when its pack has none to show. A failure is reported, but the text still shows.
  import { entryImage } from "$lib/api/images";
  import { errorMessage } from "$lib/api/storage";
  import { notify } from "$lib/stores/notifications.svelte";

  let { pack, image, fill = false }: { pack: string; image: string; fill?: boolean } = $props();
  let src = $state<string | null>(null);

  $effect(() => {
    src = null;
    let stale = false;
    entryImage(pack, image)
      .then((url) => !stale && (src = url))
      .catch((e) => notify(errorMessage(e)));
    return () => (stale = true);
  });
</script>

{#if src}<img class="picture" class:fill {src} alt="" />{/if}

<style>
  /* A fixed square, so the text doesn't move when it has loaded; at most 40% of the page in a narrow window. */
  .picture {
    float: right;
    width: min(var(--entry-picture-size), 40%);
    aspect-ratio: 1;
    margin: 0 0 var(--space-3) var(--space-4);
    object-fit: contain;
  }

  /* Filling a tile (a homebrew form's picture), cropped to it. */
  .fill {
    float: none;
    width: 100%;
    height: 100%;
    margin: 0;
    object-fit: cover;
  }
</style>
