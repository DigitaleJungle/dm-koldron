import { getCurrentWindow, type CloseRequestedEvent } from "@tauri-apps/api/window";

/** Runs `handler` when the user closes the window; it can hold the close with `event.preventDefault()`. Resolves to the unlisten function. */
export const onCloseRequested = (handler: (event: CloseRequestedEvent) => void | Promise<void>) => getCurrentWindow().onCloseRequested(handler);

/** Closes the window without asking again (after a held close was answered). */
export const closeWindow = () => getCurrentWindow().destroy();
