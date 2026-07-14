import type { NetworkName } from "./types";

const ACTIVE_WALLET_KEY = "minisatoshi.activeWalletId";
const ESPLORA_URL_KEY = "minisatoshi.esploraUrl";
const NETWORK_KEY = "minisatoshi.preferredNetwork";

export function getActiveWalletId(): string | null {
  return localStorage.getItem(ACTIVE_WALLET_KEY);
}

export function setActiveWalletId(id: string | null) {
  if (id) localStorage.setItem(ACTIVE_WALLET_KEY, id);
  else localStorage.removeItem(ACTIVE_WALLET_KEY);
}

export function getEsploraUrl(): string {
  return localStorage.getItem(ESPLORA_URL_KEY) ?? "";
}

export function setEsploraUrl(url: string) {
  if (url.trim()) localStorage.setItem(ESPLORA_URL_KEY, url.trim());
  else localStorage.removeItem(ESPLORA_URL_KEY);
}

export function getPreferredNetwork(): NetworkName {
  const value = localStorage.getItem(NETWORK_KEY);
  if (
    value === "mainnet" ||
    value === "testnet" ||
    value === "signet" ||
    value === "regtest"
  ) {
    return value;
  }
  return "testnet";
}

export function setPreferredNetwork(network: NetworkName) {
  localStorage.setItem(NETWORK_KEY, network);
}

export function formatSats(sats: number): string {
  return `${sats.toLocaleString()} sats`;
}

export async function copyText(value: string): Promise<void> {
  await navigator.clipboard.writeText(value);
}
