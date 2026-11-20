import { invoke } from "@tauri-apps/api/core";

/** The latest published release's version when it's newer than the running app, else null. Rejects with a StorageError. */
export function checkUpdate(): Promise<string | null> {
  return invoke("check_update");
}

/** Downloads, checks and installs the latest release, then restarts the app. Rejects with a StorageError. */
export function installUpdate(): Promise<void> {
  return invoke("install_update");
}
