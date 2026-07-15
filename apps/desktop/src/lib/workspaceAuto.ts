import {
  createWorkspace,
  listWallets,
  listWorkspaces,
} from "./api";
import { formatNetwork, setActiveWorkspaceId } from "./settings";
import type { NetworkName, WalletSummaryDto } from "./types";

export type WalletListItem = WalletSummaryDto & {
  network: NetworkName;
};

/** Get-or-create the implicit workspace for a chain network (hidden from UX). */
export async function ensureWorkspaceForNetwork(
  network: NetworkName,
): Promise<string> {
  const list = await listWorkspaces();
  const existing = list.find((w) => w.network === network);
  if (existing) {
    setActiveWorkspaceId(existing.id);
    return existing.id;
  }
  const created = await createWorkspace({
    name: formatNetwork(network),
    network,
  });
  setActiveWorkspaceId(created.id);
  return created.id;
}

/** All spendable wallets across all networks (excludes calling filter). */
export async function listAllWallets(): Promise<WalletListItem[]> {
  const spaces = await listWorkspaces();
  const out: WalletListItem[] = [];
  for (const ws of spaces) {
    const wallets = await listWallets(ws.id);
    for (const w of wallets) {
      out.push({ ...w, network: ws.network });
    }
  }
  return out;
}
