// Monster pictures for the encounter screens (list, Prepare, Run): one way to ask for them and to report a failure.
import { entryImage } from "$lib/api/images";
import { errorMessage } from "$lib/api/storage";
import { notify } from "$lib/stores/notifications.svelte";

const asked = new Map<string, Promise<string | null>>();

/**
 * A monster's picture, or null (its pack's images are off, offline, or it has none). Every answer is kept for the
 * session, so screens may ask as often as they redraw: nothing is fetched or reported twice. A picture that can't be
 * kept is reported once and shows as none. `forgetPictures` starts over (when the image settings change).
 */
export function monsterPicture(pack: string, image: string): Promise<string | null> {
  const key = `${pack}/${image}`;
  if (!asked.has(key)) {
    asked.set(
      key,
      entryImage(pack, image).catch((e) => {
        notify(errorMessage(e));
        return null;
      }),
    );
  }
  return asked.get(key)!;
}

/** Asks for every picture again next time: call after a pack's images are switched on or off. */
export const forgetPictures = () => asked.clear();
