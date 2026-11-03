<script lang="ts">
  import "$lib/styles/tokens.css";
  import "../../packs/tabler-icons/tabler-icons.min.css";
  import "../../packs/tabler-icons/tabler-icons-filled.min.css";
  let { children } = $props();
</script>

{@render children()}

<style>
  :global(body) {
    margin: 0;
    background: var(--color-bg);
    color: var(--color-text);
    font-family: var(--font-sans);
  }
</style>
