<script lang="ts">
  // The party icons (Tabler's users, users-minus and users-plus, docs/users*.svg) as one drawing whose strokes draw in
  // and out, so switching between them moves instead of fading. They share the head and the figure behind; users-minus
  // and users-plus cut the front figure's shoulder short and leave out the second figure's body, adding a − (and a |).
  // Each changing stroke has a path length of 1, so its dash offset is how much of it is hidden (from its end).
  let { shape }: { shape: "users" | "minus" | "plus" } = $props();

  const users = $derived(shape === "users");
  /** Hidden: a little past the whole stroke, so its round cap doesn't leave a dot at its start. */
  const HIDDEN = 1.1;
</script>

<svg class="party-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <path d="M5 7a4 4 0 1 0 8 0a4 4 0 1 0 -8 0" />
  <path d="M16 3.13a4 4 0 0 1 0 7.75" />
  <!-- The front figure's body; users-minus and users-plus end it 73% of the way, on its shoulder. -->
  <path pathLength="1" d="M3 21v-2a4 4 0 0 1 4 -4h4a4 4 0 0 1 4 4v2" style:stroke-dashoffset={users ? 0 : 0.27} />
  <path pathLength="1" d="M21 21v-2a4 4 0 0 0 -3 -3.85" style:stroke-dashoffset={users ? 0 : HIDDEN} />
  <path class="sign" pathLength="1" d="M16 19h6" style:stroke-dashoffset={users ? HIDDEN : 0} />
  <path class="sign" pathLength="1" d="M19 16v6" style:stroke-dashoffset={shape === "plus" ? 0 : HIDDEN} />
</svg>

<style>
  .party-icon {
    width: var(--icon-size);
    height: var(--icon-size);
    stroke-width: 1.5; /* like the icon font */
  }

  path[pathLength] {
    stroke-dasharray: 1 2; /* one dash as long as the stroke, then a gap long enough to hide it */
    transition: stroke-dashoffset 220ms ease;
  }

  /* The − and | start a moment later, so the figure makes room for them first. */
  .sign {
    transition-delay: 60ms;
  }

  :global([data-motion="reduce"]) path[pathLength] {
    transition: none;
  }
</style>
