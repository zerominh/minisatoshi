import type { NetworkName } from "./types";

/** BIP-86 account path for Taproot xpub export via HWI. */
export function defaultBip86AccountPath(network: NetworkName): string {
  return network === "mainnet" ? "m/86'/0'/0'" : "m/86'/1'/0'";
}

/** Store origin without `m/` prefix (policy compiler accepts both). */
export function originPathFromDerivation(path: string): string {
  return path.replace(/^m\//i, "").trim();
}
