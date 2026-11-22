<script lang="ts">
  // Asks before leaving a Homebrew form with changes not saved: going elsewhere in the app, closing the window, or a
  // close of the form's own (`ask`). Escape or Cancel stays; Discard leaves.
  import { beforeNavigate, goto } from "$app/navigation";
  import type { BeforeNavigate } from "@sveltejs/kit";
  import { closeWindow, onCloseRequested } from "$lib/api/window";
  import { gotoReplacing, isReplacing } from "$lib/stores/history.svelte";
  import { strings } from "$lib/strings";

  // `keeps(navigation)`: a navigation this guard leaves alone (one that keeps the form open, or that its owner handles).
  let { changed = $bindable(false), keeps }: { changed?: boolean; keeps?: (navigation: BeforeNavigate) => boolean } = $props();
  const t = strings.homebrew.leave;
  const id = $props.id();
  let dialog: HTMLDialogElement | undefined = $state();
  let resolveLeave: ((leave: boolean) => void) | null = null;

  /** Whether to leave: at once without changes, else the user's answer. */
  export function ask(): Promise<boolean> {
    if (!changed) return Promise.resolve(true);
    dialog?.showModal();
    return new Promise((resolve) => (resolveLeave = resolve));
  }

  // Escape closes the dialog too, which counts as Cancel.
  function answer(leave: boolean) {
    const resolve = resolveLeave;
    resolveLeave = null;
    if (dialog?.open) dialog.close();
    resolve?.(leave);
  }

  beforeNavigate((navigation) => {
    if (!changed || !navigation.to || navigation.type === "leave" || keeps?.(navigation)) return;
    const replace = isReplacing();
    navigation.cancel();
    void ask().then((leave) => {
      if (!leave) return;
      changed = false;
      if (navigation.type === "popstate") history.go(navigation.delta ?? -1);
      else void (replace ? gotoReplacing : goto)(navigation.to!.url.href);
    });
  });

  // Closing the window is leaving too (SvelteKit's "leave" can't be held).
  $effect(() => {
    const unlisten = onCloseRequested(async (event) => {
      if (!changed) return;
      event.preventDefault();
      if (!(await ask())) return;
      changed = false;
      await closeWindow();
    });
    return () => void unlisten.then((stop) => stop());
  });
</script>

<dialog bind:this={dialog} class="dialog" aria-labelledby="{id}-title" aria-describedby="{id}-question" onclose={() => answer(false)}>
  <h2 id="{id}-title">{t.title}</h2>
  <p id="{id}-question">{t.question}</p>
  <div class="actions">
    <!-- svelte-ignore a11y_autofocus: the safe answer is the default -->
    <button class="btn" autofocus onclick={() => answer(false)}>{t.cancel}</button>
    <button class="btn btn-danger" onclick={() => answer(true)}>{t.discard}</button>
  </div>
</dialog>

<style>
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>
