import { gotoReplacing } from "$lib/stores/history.svelte";
import * as api from "$lib/api/campaigns";
import type { Campaign, CampaignList } from "$lib/api/campaigns";
import { errorMessage } from "$lib/api/storage";
import { strings } from "$lib/strings";

/** Shared by the Start screen, the campaign screens and Settings, so a rename or archive shows everywhere at once. */
export const campaigns = $state({
  list: null as CampaignList | null, // null until loaded
  loadError: null as string | null,
  /** The campaign whose screens are open, if any. */
  current: null as Campaign | null,
  /** The campaign the DM is working in: set when its screens open, kept in the Library, cleared on the Start screen. Search stays inside it. */
  scope: null as string | null,
});

/** The campaign whose settings dialog is open (from the switcher, its Start tile or Settings → Campaigns), if any. */
export const campaignDialog = $state({ id: null as string | null });

/** The Library to open pack content in: the campaign's own while working in one (inside its screens), else the one
 * outside any campaign. */
export const libraryHome = () => (campaigns.scope ? `/campaign/${campaigns.scope}/library` : "/library");

export async function refreshCampaigns() {
  try {
    campaigns.list = await api.listCampaigns();
    campaigns.loadError = null;
  } catch (e) {
    campaigns.loadError = strings.start.loadError(errorMessage(e));
  }
}

// The mutations below reject with a StorageError; callers show it next to the control that failed.

export async function createCampaign(name: string): Promise<Campaign> {
  const campaign = await api.createCampaign(name);
  await refreshCampaigns();
  return campaign;
}

export async function renameCampaign(id: string, name: string) {
  const renamed = await api.renameCampaign(id, name);
  if (campaigns.current?.id === id) campaigns.current = renamed;
  await refreshCampaigns();
}

export async function setPartyName(id: string, name: string) {
  const updated = await api.setPartyName(id, name);
  if (campaigns.current?.id === id) campaigns.current = updated;
  await refreshCampaigns();
}

export async function setCampaignPacks(id: string, packsOff: string[]) {
  const updated = await api.setCampaignPacks(id, packsOff);
  if (campaigns.current?.id === id) campaigns.current = updated;
  await refreshCampaigns();
}

/** Asks for a picture and sets it, or removes it (`choose` false). Does nothing if the dialog is cancelled. */
export async function setCampaignPicture(id: string, choose: boolean) {
  const updated = choose ? await api.chooseCampaignPicture(id) : await api.removeCampaignPicture(id);
  if (!updated) return;
  if (campaigns.current?.id === id) campaigns.current = updated;
  await refreshCampaigns();
}

export async function archiveCampaign(id: string) {
  await api.archiveCampaign(id);
  await refreshCampaigns();
  if (campaigns.scope === id) campaigns.scope = null; // also when only the Library from it is open
  // The open campaign is gone from the active list, so leave its screens, in their place in the history: Back (or the
  // mouse's back button) mustn't return to a campaign that's no longer there.
  if (campaigns.current?.id === id) await gotoReplacing("/");
}

export async function restoreCampaign(id: string) {
  await api.restoreCampaign(id);
  await refreshCampaigns();
}

/** Deletes an active or archived campaign; leaves its screens if it was open. */
export async function deleteCampaign(id: string) {
  await api.deleteCampaign(id);
  await refreshCampaigns();
  if (campaigns.scope === id) campaigns.scope = null;
  if (campaigns.current?.id === id) await gotoReplacing("/"); // as when archiving
}
