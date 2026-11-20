import { checkUpdate, installUpdate } from "$lib/api/updates";
import { errorMessage } from "$lib/api/storage";

/** The startup update check: the newer version, and why the check or the install failed (shown in Settings → About). */
export const update = $state({
  latest: null as string | null,
  checkError: null as string | null,
  installError: null as string | null,
  installing: false,
});

/** Checks once at startup; resolves to the newer version, if any. Offline or unreachable stays silent here. */
export async function checkForUpdate(): Promise<string | null> {
  try {
    update.latest = await checkUpdate();
    update.checkError = null;
  } catch (e) {
    update.checkError = errorMessage(e);
  }
  return update.latest;
}

/** Installs the newer version; the app restarts when it's done. */
export async function installNewVersion() {
  update.installing = true;
  update.installError = null;
  try {
    await installUpdate();
  } catch (e) {
    update.installError = errorMessage(e);
  } finally {
    update.installing = false;
  }
}
