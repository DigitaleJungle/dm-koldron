// The Characters screen: class line, total level, searching and sorting the cards, and the picture's square crop.
import type { Character } from "$lib/api/characters";

/** "Fighter 3 / Wizard 2". */
/** The largest stat (HP, AC, passive scores) a character form takes: far above any real one, and safe for Rust's numbers. */
export const MAX_STAT = 99999;

export const classLine = (c: Pick<Character, "classes">) => c.classes.map((k) => `${k.name} ${k.level}`).join(" / ");

export const totalLevel = (c: Pick<Character, "classes">) => c.classes.reduce((sum, k) => sum + (Number(k.level) || 0), 0);

export type CharacterSort = "name" | "level" | "class";
/** All characters, only those in the party, or only those who aren't. */
export type PartyFilter = "all" | "in" | "out";

/**
 * Every search word in the name, the class line or the player, in or out of the party as asked, and with a class of
 * that name (any case; empty: any class); by name, by level (highest first) or by class line, ties by name.
 */
export function selectCharacters(list: Character[], search: string, sort: CharacterSort, party: PartyFilter = "all", className = ""): Character[] {
  const words = search.toLowerCase().split(/\s+/).filter(Boolean);
  const matches = (c: Character) => words.every((w) => [c.name, classLine(c), c.player].some((text) => text.toLowerCase().includes(w)));
  const wanted = className.toLowerCase();
  const inFilter = (c: Character) =>
    (party === "all" || c.onHome === (party === "in")) && (!wanted || c.classes.some((k) => k.name.toLowerCase() === wanted));
  const byName = (a: Character, b: Character) => a.name.localeCompare(b.name);
  const first = (a: Character, b: Character) =>
    sort === "level" ? totalLevel(b) - totalLevel(a) : sort === "class" ? classLine(a).localeCompare(classLine(b)) : 0;
  return list.filter((c) => matches(c) && inFilter(c)).sort((a, b) => first(a, b) || byName(a, b));
}

/** The classes the characters have, each once (ignoring case, the first spelling kept), alphabetically. */
export function classNamesOf(list: Character[]): string[] {
  const byKey = new Map<string, string>();
  for (const c of list) for (const { name } of c.classes) if (!byKey.has(name.toLowerCase())) byKey.set(name.toLowerCase(), name);
  return [...byKey.values()].sort((a, b) => a.localeCompare(b));
}

/** The party's average level, to one decimal ("5", "4.5"); null without a party. */
export function averageLevel(party: Character[]): string | null {
  if (party.length === 0) return null;
  const average = party.reduce((sum, c) => sum + totalLevel(c), 0) / party.length;
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 }).format(average);
}

/** A new character's form: one class row, nothing else filled in. */
export const blankCharacter = (): Character => ({
  id: "",
  name: "",
  player: "",
  classes: [{ name: "", level: 1 }],
  maxHp: null,
  currentHp: null,
  tempHp: null,
  ac: null,
  passivePerception: null,
  passiveInsight: null,
  passiveInvestigation: null,
  onHome: false,
  createdAt: 0,
  updatedAt: 0,
});

const SIDE = 512;

/** Where the square sits on the picture's longer side: 0 = left / top, 1 = right / bottom. */
export type Crop = { x: number; y: number };
/** Centred across and a quarter of the way down, where faces usually are. */
export const defaultCrop = (): Crop => ({ x: 0.5, y: 0.25 });
export const clamp01 = (n: number) => Math.min(1, Math.max(0, n));

/** A picked picture: decoded (to crop it) and as a data URL (to show it; the CSP allows data:, not blob:). */
export async function loadPicture(file: File): Promise<{ image: ImageBitmap; url: string }> {
  const image = await createImageBitmap(file);
  const url = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
  return { image, url };
}

/** The square at `crop` as JPEG. Rust decodes and shrinks it again before saving. */
export function cropPicture(image: ImageBitmap, crop: Crop): Uint8Array {
  const side = Math.min(image.width, image.height);
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = Math.min(side, SIDE);
  const context = canvas.getContext("2d");
  if (!context) throw new Error("no canvas");
  context.drawImage(image, (image.width - side) * crop.x, (image.height - side) * crop.y, side, side, 0, 0, canvas.width, canvas.height);
  const binary = atob(canvas.toDataURL("image/jpeg", 0.9).split(",")[1]);
  return Uint8Array.from(binary, (ch) => ch.charCodeAt(0));
}
