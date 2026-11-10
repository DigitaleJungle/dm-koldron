import { downloadImages, type ImageProgress } from "$lib/api/images";
import { errorMessage } from "$lib/api/storage";
import { forgetPictures } from "$lib/stores/monsterPictures";
import { notify } from "$lib/stores/notifications.svelte";
import { strings } from "$lib/strings";

/** The image download running per pack id (Settings → Rules). Lives outside Settings, so closing it doesn't stop it. */
export const imageDownloads = $state<Record<string, ImageProgress>>({});

/** Starts downloading a pack's images; a notification says how it ended. */
export function startImageDownload(pack: string, name: string) {
  if (imageDownloads[pack]) return;
  const t = strings.settings.rules.images;
  imageDownloads[pack] = { done: 0, failed: 0, total: 0, cancelled: false };
  downloadImages(pack, (p) => imageDownloads[pack] && (imageDownloads[pack] = p))
    .then((p) => notify(p.cancelled ? t.cancelled(name) : p.failed ? t.doneWithFailures(name, p.done, p.failed) : t.done(name, p.done)))
    .catch((e) => notify(t.downloadError(name, errorMessage(e))))
    .finally(() => {
      delete imageDownloads[pack];
      forgetPictures(); // pictures that weren't kept before may be now
    });
}
