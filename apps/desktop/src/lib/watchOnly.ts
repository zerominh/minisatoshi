import type { VaultDto } from "./types";

/** True when Settings has a HW fingerprint that matches a vault policy key. */
export function hasRememberedSigningDevice(
  vault: VaultDto,
  fingerprint: string,
): boolean {
  const fp = fingerprint.trim().toLowerCase();
  if (!fp) return false;
  return vault.policy.keys.some(
    (key) => key.fingerprint.trim().toLowerCase() === fp,
  );
}
