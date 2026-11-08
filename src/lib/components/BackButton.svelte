<script lang="ts">
  import { canGoBack } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";

  // A true back: the same as the mouse's back button (the screen the user came from, as it was: filters, scroll).
  // `fallback` when there's nothing to go back to (the app was opened right here). Still a link, so it works from the
  // keyboard and shows where it goes without history.
  let { fallback = "/" }: { fallback?: string } = $props();

  function back(event: MouseEvent) {
    if (!canGoBack()) return;
    event.preventDefault();
    history.back();
  }
</script>

<a class="btn back" href={fallback} onclick={back}>
  <i class="ti ti-chevron-left" aria-hidden="true"></i>
  {strings.back}
</a>

<style>
  .back {
    gap: var(--space-1);
    padding-left: var(--space-2);
    color: var(--color-text);
    text-decoration: none;
  }
</style>
