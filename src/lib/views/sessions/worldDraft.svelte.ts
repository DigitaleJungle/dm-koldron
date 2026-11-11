// The campaign's locations and NPCs being edited: loaded once, saved on request (the page decides when: a second after
// the last change, and before leaving). Shared by the Sessions overview and the session page.
import { errorMessage } from "$lib/api/storage";
import { getWorld, saveWorld, type World } from "$lib/api/world";
import { strings } from "$lib/strings";

export class WorldDraft {
  /** Null until it has loaded. */
  world = $state<World | null>(null);
  loadError = $state<string | null>(null);
  saveError = $state<string | null>(null);
  saving = $state(false);
  /** What was last saved (or loaded). */
  #baseline = $state<string | null>(null);
  /** What last failed to save: not tried again until something changes. */
  #failed = $state<string | null>(null);
  #inflight: Promise<boolean> | null = null;

  private campaign: string;

  constructor(campaign: string) {
    this.campaign = campaign;
    getWorld(campaign).then(
      (w) => {
        this.world = w;
        this.#baseline = JSON.stringify(w);
      },
      (e) => (this.loadError = strings.sessions.world.loadError(errorMessage(e))),
    );
  }

  get changed(): boolean {
    return this.world !== null && JSON.stringify(this.world) !== this.#baseline;
  }

  /** Changed, and not what last failed: worth saving on its own. */
  get due(): boolean {
    return this.changed && !this.saving && JSON.stringify(this.world) !== this.#failed;
  }

  /** Saves the changes (after a save under way); false when it couldn't (`saveError` says why). */
  async save(): Promise<boolean> {
    while (this.#inflight) await this.#inflight;
    if (!this.changed) return true;
    const sent = JSON.stringify(this.world);
    this.saving = true;
    this.#inflight = saveWorld(this.campaign, JSON.parse(sent)).then(
      () => {
        this.#baseline = sent; // changed again while it saved: still unsaved, and saved next
        [this.saveError, this.#failed] = [null, null];
        return true;
      },
      (e) => {
        [this.saveError, this.#failed] = [strings.sessions.world.saveError(errorMessage(e)), sent];
        return false;
      },
    );
    try {
      return await this.#inflight;
    } finally {
      this.#inflight = null;
      this.saving = false;
    }
  }
}
