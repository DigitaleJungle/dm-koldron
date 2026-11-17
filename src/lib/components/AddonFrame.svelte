<script lang="ts">
  import { addonUrl, type AddonSummary } from "$lib/api/addons";
  import { errorMessage } from "$lib/api/storage";
  import { charactersChanged, setLibraryVisible } from "$lib/stores/addons.svelte";
  import { notify } from "$lib/stores/notifications.svelte";
  import { registerPanel, unsavedPanels } from "$lib/stores/settingsPanels.svelte";
  import { strings } from "$lib/strings";
  import { answer, isCall } from "./addonBridge";

  // One page of an addon, in a sandboxed frame: scripts run, but with an opaque origin, so it can't reach this window,
  // Tauri or storage. Forms are allowed only so their submit event fires (without allow-forms the browser drops a
  // submit before the event); the page's own CSP has form-action 'none', so a form can't send anything anywhere. It talks to the app only through postMessage calls, answered by `answer`. `context` is sent to
  // the page once it has loaded and again whenever it changes: `{ koldron: 1, event: "context", context }`.
  let {
    addon,
    entry,
    context,
    hidden = false,
    onpackschange,
    onclose,
    onnavigate,
    onsidebarbutton,
    panel,
    onopenlibrary,
    onheight,
  }: {
    addon: AddonSummary;
    entry: string;
    context?: Record<string, unknown>;
    /** A background page: kept out of sight (display: none also keeps it out of the Tab order). */
    hidden?: boolean;
    onpackschange?: () => void;
    onclose?: () => void;
    /** Only the addon's Library tab page: `ui.navigate` moves the app to this state of the tab. */
    onnavigate?: (state: string | null, replace: boolean) => Promise<void>;
    /** Only a Library tab page with `sidebar`: `ui.setSidebarButton` shows or hides the header's sidebar button. */
    onsidebarbutton?: (shown: boolean) => void;
    /** A Settings panel (its section): its changes wait for the dialog's Save (`ui.setUnsaved`, `ui.settingsDone`). */
    panel?: string;
    /** Only the Start screen page: `ui.openLibrary` opens the addon's Library tab at a state. */
    onopenlibrary?: (state: string | null) => Promise<void>;
    /** Hears the height the page asks for (`ui.setHeight`). */
    onheight?: (px: number) => void;
  } = $props();
  let frame: HTMLIFrameElement | undefined = $state();
  let height = $state<number>();
  let loaded = $state(false);

  // A Settings panel saves or discards when the dialog says so, and answers with `ui.settingsDone`.
  // Saving may check something online; discarding only goes back, so closing never waits long on a stuck page.
  const PANEL_ANSWER_MS = { save: 120_000, discard: 10_000 };
  const panelKey = $derived(panel ? `${addon.id}:${panel}` : null);
  let nextRequest = 0;
  const waiting = new Map<number, { done: () => void; failed: (e: Error) => void }>();
  $effect(() => {
    if (!panelKey) return;
    return registerPanel(panelKey, (action) => {
      const id = ++nextRequest;
      return new Promise<void>((done, failed) => {
        const timer = setTimeout(() => {
          waiting.delete(id);
          failed(new Error(strings.addons.noAnswer(addon.name)));
        }, PANEL_ANSWER_MS[action]);
        const stop = () => clearTimeout(timer);
        waiting.set(id, { done: () => (stop(), done()), failed: (e) => (stop(), failed(e)) });
        frame?.contentWindow?.postMessage({ koldron: 1, event: "settings", action, id }, "*");
      });
    });
  });
  // Also in the page's address (?theme=), so the page can show it before its first paint.
  const theme = () => document.documentElement.dataset.theme ?? "light";

  $effect(() => {
    const sent = $state.snapshot(context);
    if (loaded && sent) frame?.contentWindow?.postMessage({ koldron: 1, event: "context", context: sent }, "*");
  });

  function onmessage(event: MessageEvent) {
    // Only this frame's own window: the sender decides which addon's permissions apply, never the message.
    if (!frame || event.source !== frame.contentWindow || !isCall(event.data)) return;
    const { id, call, args } = event.data;
    const reply = (message: object, transfer: Transferable[] = []) =>
      frame?.contentWindow?.postMessage({ koldron: 1, id, ...message }, "*", transfer);
    const host = {
      setHeight: (px: number) => {
        height = px;
        onheight?.(px);
      },
      packsChanged: () => onpackschange?.(),
      charactersChanged,
      theme,
      close: () => onclose?.(),
      // Always says which addon speaks, so its words can't pass for the app's.
      notify: (message: string) => notify(strings.addons.notice(addon.name, message)),
      navigate: async (state: string | null, replace: boolean) => {
        if (!onnavigate) throw new Error("ui.navigate works only from the addon's Library tab.");
        await onnavigate(state, replace);
      },
      libraryVisible: (visible: boolean) => setLibraryVisible(addon.id, visible),
      sidebarButton: (shown: boolean) => {
        if (!onsidebarbutton) throw new Error("ui.setSidebarButton works only from a Library tab with a sidebar.");
        onsidebarbutton(shown);
      },
      openLibrary: async (state: string | null) => {
        if (!onopenlibrary) throw new Error("ui.openLibrary works only from the Start screen page of an addon with a Library tab.");
        await onopenlibrary(state);
      },
      unsaved: (has: boolean) => {
        if (!panelKey) throw new Error("ui.setUnsaved works only from a Settings panel.");
        unsavedPanels[panelKey] = has;
      },
      settingsDone: (request: number, error: string | null) => {
        const pending = waiting.get(request);
        waiting.delete(request);
        if (error === null) pending?.done();
        else pending?.failed(new Error(strings.addons.notice(addon.name, error)));
      },
    };
    answer(addon.id, call, args ?? [], host)
      .then((result) => reply({ ok: true, result }, result instanceof ArrayBuffer ? [result] : []))
      .catch((e) => reply({ ok: false, error: errorMessage(e) }));
  }
</script>

<svelte:window {onmessage} />

<iframe
  bind:this={frame}
  src="{addonUrl(addon.id, entry)}?theme={theme()}"
  title={addon.name}
  sandbox="allow-scripts allow-forms"
  class:hidden
  aria-hidden={hidden || undefined}
  onload={() => (loaded = true)}
  style:height={height === undefined ? undefined : `${height}px`}
></iframe>

<style>
  iframe {
    width: 100%;
    border: none;
    background: transparent;
  }

  .hidden {
    display: none;
  }
</style>
