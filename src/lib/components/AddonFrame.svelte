<script lang="ts">
  import { addonUrl, type AddonSummary } from "$lib/api/addons";
  import { errorMessage } from "$lib/api/storage";
  import { answer, isCall } from "./addonBridge";

  // One page of an addon, in a sandboxed frame: scripts run, but with an opaque origin, so it can't reach this window,
  // Tauri or storage. It talks to the app only through postMessage calls, answered by `answer`.
  let { addon, entry, onpackschange }: { addon: AddonSummary; entry: string; onpackschange?: () => void } = $props();
  let frame: HTMLIFrameElement | undefined = $state();
  let height = $state<number>();

  function onmessage(event: MessageEvent) {
    // Only this frame's own window: the sender decides which addon's permissions apply, never the message.
    if (!frame || event.source !== frame.contentWindow || !isCall(event.data)) return;
    const { id, call, args } = event.data;
    const reply = (message: object, transfer: Transferable[] = []) =>
      frame?.contentWindow?.postMessage({ koldron: 1, id, ...message }, "*", transfer);
    const host = {
      setHeight: (px: number) => (height = px),
      packsChanged: () => onpackschange?.(),
      theme: () => document.documentElement.dataset.theme ?? "light",
    };
    answer(addon.id, call, args ?? [], host)
      .then((result) => reply({ ok: true, result }, result instanceof ArrayBuffer ? [result] : []))
      .catch((e) => reply({ ok: false, error: errorMessage(e) }));
  }
</script>

<svelte:window {onmessage} />

<iframe
  bind:this={frame}
  src={addonUrl(addon.id, entry)}
  title={addon.name}
  sandbox="allow-scripts"
  style:height={height === undefined ? undefined : `${height}px`}
></iframe>

<style>
  iframe {
    width: 100%;
    border: none;
    background: transparent;
  }
</style>
