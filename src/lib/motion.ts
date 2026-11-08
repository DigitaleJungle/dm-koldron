// How a filtered list of cards moves (Characters and Encounters): when search, filters or sort change, the cards
// glide to their new places and returning ones fade in. Leaving ones go at once: fading out, they'd keep their place
// until gone, and the others would jump instead of glide. Instant with Settings → UI → Animations off.
import { cubicInOut } from "svelte/easing";
import { reduceMotion } from "$lib/stores/theme.svelte";

/** For `animate:flip` on each card. */
export const moveMotion = () => ({ duration: reduceMotion() ? 0 : 200, easing: cubicInOut });

/** For `in:scale` on each card. */
export const enterMotion = () => ({ start: 0.95, duration: reduceMotion() ? 0 : 200, easing: cubicInOut });
