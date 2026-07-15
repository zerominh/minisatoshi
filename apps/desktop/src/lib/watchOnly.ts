import type { WalletDto } from "./types";

/** True when Settings has a HW fingerprint that matches a wallet policy key. */
export function hasRememberedSigningDevice(
  wallet: WalletDto,
  fingerprint: string,
): boolean {
  const fp = fingerprint.trim().toLowerCase();
  if (!fp) return false;
  return wallet.policy.keys.some(
    (key) => key.fingerprint.trim().toLowerCase() === fp,
  );
}
