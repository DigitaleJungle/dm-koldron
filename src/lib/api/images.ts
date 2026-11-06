import { Channel, convertFileSrc, invoke } from "@tauri-apps/api/core";

/** Mirrors `PackImages` in src-tauri/src/commands/images.rs: a pack's downloadable images, their total size, and how many are kept. */
export type PackImages = { pack: string; count: number; bytes: number; kept: number };
/** Mirrors `Progress`: `done` includes images already kept. */
export type ImageProgress = { done: number; failed: number; total: number; cancelled: boolean };

/** The packs that list images. Rejects with a StorageError. */
export function listPackImages(): Promise<PackImages[]> {
  return invoke("list_pack_images");
}

/** A URL for an entry's `image`, downloaded and kept the first time; null when its pack's images are off, offline, or none. */
export async function entryImage(pack: string, image: string): Promise<string | null> {
  const path: string | null = await invoke("entry_image", { pack, image });
  return path && convertFileSrc(path);
}

/** Downloads every image of the pack not kept yet. Resolves when finished or cancelled; rejects with a StorageError. */
export function downloadImages(pack: string, onProgress: (p: ImageProgress) => void): Promise<ImageProgress> {
  const channel = new Channel<ImageProgress>();
  channel.onmessage = onProgress;
  return invoke("download_images", { pack, onProgress: channel });
}

export function cancelImageDownload(pack: string): Promise<void> {
  return invoke("cancel_image_download", { pack });
}

/** Cancels a running download and removes the pack's kept images. Rejects with a StorageError. */
export function deleteImages(pack: string): Promise<void> {
  return invoke("delete_images", { pack });
}
