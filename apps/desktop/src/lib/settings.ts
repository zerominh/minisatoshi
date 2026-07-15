import type { NetworkName } from "./types";

const ACTIVE_WORKSPACE_KEY = "minisatoshi.activeWorkspaceId";
/** @deprecated pre-rename key (container was called "wallet") — read as fallback only. */
const LEGACY_ACTIVE_WALLET_KEY = "minisatoshi.activeWalletId";
const ESPLORA_URL_KEY = "minisatoshi.esploraUrl";
const NETWORK_KEY = "minisatoshi.preferredNetwork";
const HWI_PATH_KEY = "minisatoshi.hwiPath";
const HW_FINGERPRINT_KEY = "minisatoshi.hwFingerprint";
const LOCALE_KEY = "minisatoshi.locale";

export type AppLocale = "en" | "vi";

export function getLocale(): AppLocale {
  const value = localStorage.getItem(LOCALE_KEY);
  if (value === "vi" || value === "en") return value;
  const nav =
    typeof navigator !== "undefined" ? navigator.language.toLowerCase() : "";
  return nav.startsWith("vi") ? "vi" : "en";
}

export function setLocale(locale: AppLocale) {
  localStorage.setItem(LOCALE_KEY, locale);
}

export function getActiveWorkspaceId(): string | null {
  return (
    localStorage.getItem(ACTIVE_WORKSPACE_KEY) ??
    localStorage.getItem(LEGACY_ACTIVE_WALLET_KEY)
  );
}

export function setActiveWorkspaceId(id: string | null) {
  if (id) localStorage.setItem(ACTIVE_WORKSPACE_KEY, id);
  else localStorage.removeItem(ACTIVE_WORKSPACE_KEY);
}

export function getEsploraUrl(): string {
  return localStorage.getItem(ESPLORA_URL_KEY) ?? "";
}

export function setEsploraUrl(url: string) {
  if (url.trim()) localStorage.setItem(ESPLORA_URL_KEY, url.trim());
  else localStorage.removeItem(ESPLORA_URL_KEY);
}

export function getHwiPath(): string {
  return localStorage.getItem(HWI_PATH_KEY) ?? "";
}

export function setHwiPath(path: string) {
  if (path.trim()) localStorage.setItem(HWI_PATH_KEY, path.trim());
  else localStorage.removeItem(HWI_PATH_KEY);
}

export function getHwFingerprint(): string {
  return localStorage.getItem(HW_FINGERPRINT_KEY) ?? "";
}

export function setHwFingerprint(fp: string) {
  if (fp.trim()) localStorage.setItem(HW_FINGERPRINT_KEY, fp.trim());
  else localStorage.removeItem(HW_FINGERPRINT_KEY);
}

export function getPreferredNetwork(): NetworkName {
  const value = localStorage.getItem(NETWORK_KEY);
  if (
    value === "mainnet" ||
    value === "testnet" ||
    value === "testnet4" ||
    value === "signet" ||
    value === "regtest"
  ) {
    return value;
  }
  return "testnet";
}

/** Human-readable network label (Testnet3 vs Testnet4). */
export function formatNetwork(network: NetworkName): string {
  switch (network) {
    case "testnet":
      return "Testnet3";
    case "testnet4":
      return "Testnet4";
    case "mainnet":
      return "Mainnet";
    case "signet":
      return "Signet";
    case "regtest":
      return "Regtest";
  }
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
