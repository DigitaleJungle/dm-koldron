import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { strings } from "$lib/strings";

/** A picture file in the campaign's folder; `version` changes when it's replaced. */
export type Picture = { path: string; version: number };

/** Mirrors `Campaign` in src-tauri/src/commands/campaigns.rs. Times are ms since the Unix epoch. */
export type Campaign = { id: string; name: string; createdAt: number; lastOpenedAt: number; picture?: Picture };
export type CampaignList = { campaigns: Campaign[]; archived: Campaign[]; unreadable: string[] };

/** A URL the window can show the picture from (the version makes a replaced picture load fresh). */
export const pictureSrc = (p: Picture) => `${convertFileSrc(p.path)}?v=${p.version}`;

// All of these reject with a StorageError.
export function listCampaigns(): Promise<CampaignList> {
  return invoke("list_campaigns");
}

export function createCampaign(name: string): Promise<Campaign> {
  return invoke("create_campaign", { name });
}

/** Loads an active campaign and marks it as opened now. */
export function openCampaign(id: string): Promise<Campaign> {
  return invoke("open_campaign", { id });
}

export function renameCampaign(id: string, name: string): Promise<Campaign> {
  return invoke("rename_campaign", { id, name });
}

/** Asks for a picture file; null when the dialog is cancelled. Rust then shrinks and stores it. */
export async function chooseCampaignPicture(id: string): Promise<Campaign | null> {
  const path = await open({ filters: [{ name: strings.campaignSettings.pictureFilter, extensions: ["png", "jpg", "jpeg", "webp", "gif"] }] });
  return path ? invoke("set_campaign_picture", { id, path }) : null;
}

export function removeCampaignPicture(id: string): Promise<Campaign> {
  return invoke("remove_campaign_picture", { id });
}

/** Moves the campaign to the archive, where it can be restored. */
export function archiveCampaign(id: string): Promise<void> {
  return invoke("archive_campaign", { id });
}

export function restoreCampaign(id: string): Promise<void> {
  return invoke("restore_campaign", { id });
}

/** Permanently deletes a campaign (active or archived) and all its data. */
export function deleteCampaign(id: string): Promise<void> {
  return invoke("delete_campaign", { id });
}
