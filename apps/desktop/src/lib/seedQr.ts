import { wordlist as english } from "@scure/bip39/wordlists/english.js";
import { entropyToMnemonic, validateMnemonic } from "@scure/bip39";

/** English BIP-39 wordlist (SeedQR assumes English). */
export const BIP39_ENGLISH = english;

export type SeedQrResult = {
  mnemonic: string;
  format: "standard-seedqr" | "compact-seedqr" | "plain-text" | "json";
};

function wordsFromIndices(indices: number[]): string {
  return indices
    .map((i) => {
      if (i < 0 || i >= BIP39_ENGLISH.length) {
        throw new Error(`SeedQR index out of range: ${i}`);
      }
      return BIP39_ENGLISH[i];
    })
    .join(" ");
}

/** Standard SeedQR: consecutive zero-padded 4-digit BIP-39 indices. */
export function decodeStandardSeedQr(digits: string): string {
  const clean = digits.replace(/\s+/g, "");
  if (!/^\d+$/.test(clean)) {
    throw new Error("Standard SeedQR must be numeric digits");
  }
  if (clean.length !== 48 && clean.length !== 96) {
    throw new Error(
      `Standard SeedQR needs 48 (12-word) or 96 (24-word) digits, got ${clean.length}`,
    );
  }
  const indices: number[] = [];
  for (let i = 0; i < clean.length; i += 4) {
    indices.push(Number.parseInt(clean.slice(i, i + 4), 10));
  }
  const mnemonic = wordsFromIndices(indices);
  if (!validateMnemonic(mnemonic, BIP39_ENGLISH)) {
    throw new Error("SeedQR decoded to an invalid BIP-39 checksum");
  }
  return mnemonic;
}

/**
 * CompactSeedQR: 16 or 32 entropy bytes (checksum bits omitted).
 * BIP-39 reconstructs the checksum word via entropyToMnemonic.
 */
export function decodeCompactSeedQr(bytes: Uint8Array): string {
  let entropy = bytes;
  // Some decoders prepend QR mode/length; try common trims to 16/32.
  if (entropy.length !== 16 && entropy.length !== 32) {
    if (entropy.length > 32) {
      // Prefer trailing/leading 32 then 16.
      const candidates = [entropy.slice(0, 32), entropy.slice(-32), entropy.slice(0, 16), entropy.slice(-16)];
      for (const c of candidates) {
        try {
          return decodeCompactSeedQr(c);
        } catch {
          /* try next */
        }
      }
    }
    throw new Error(
      `CompactSeedQR needs 16 or 32 entropy bytes, got ${entropy.length}`,
    );
  }
  const mnemonic = entropyToMnemonic(entropy, BIP39_ENGLISH);
  if (!validateMnemonic(mnemonic, BIP39_ENGLISH)) {
    throw new Error("CompactSeedQR decoded to an invalid BIP-39 checksum");
  }
  return mnemonic;
}

function tryExtractJsonMnemonic(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed.startsWith("{")) return null;
  try {
    const obj = JSON.parse(trimmed) as Record<string, unknown>;
    for (const key of ["mnemonic", "seed", "words", "bip39"]) {
      const v = obj[key];
      if (typeof v === "string" && v.trim()) return v.trim();
    }
  } catch {
    return null;
  }
  return null;
}

function normalizePlainMnemonic(raw: string): string | null {
  const words = raw
    .trim()
    .toLowerCase()
    .split(/[\s,]+/)
    .filter(Boolean);
  if (words.length !== 12 && words.length !== 15 && words.length !== 18 && words.length !== 21 && words.length !== 24) {
    return null;
  }
  if (!words.every((w) => BIP39_ENGLISH.includes(w))) return null;
  const mnemonic = words.join(" ");
  return validateMnemonic(mnemonic, BIP39_ENGLISH) ? mnemonic : null;
}

/**
 * Parse a decoded QR payload (text and/or raw bytes from jsQR).
 * Supports Standard SeedQR, CompactSeedQR, plain mnemonic text, and seed JSON.
 */
export function parseSeedQrPayload(
  text: string,
  binaryData?: number[] | Uint8Array | null,
): SeedQrResult {
  const trimmed = text?.trim() ?? "";

  const fromJson = tryExtractJsonMnemonic(trimmed);
  if (fromJson) {
    if (!validateMnemonic(fromJson, BIP39_ENGLISH)) {
      throw new Error("JSON mnemonic failed BIP-39 checksum");
    }
    return { mnemonic: fromJson, format: "json" };
  }

  const plain = normalizePlainMnemonic(trimmed);
  if (plain) {
    return { mnemonic: plain, format: "plain-text" };
  }

  const digitsOnly = trimmed.replace(/\s+/g, "");
  if (/^\d{48}$|^\d{96}$/.test(digitsOnly)) {
    return {
      mnemonic: decodeStandardSeedQr(digitsOnly),
      format: "standard-seedqr",
    };
  }

  if (binaryData && binaryData.length > 0) {
    const bytes =
      binaryData instanceof Uint8Array
        ? binaryData
        : Uint8Array.from(binaryData);
    try {
      return {
        mnemonic: decodeCompactSeedQr(bytes),
        format: "compact-seedqr",
      };
    } catch {
      // fall through
    }
  }

  // Compact may arrive as latin1/binary string in `text`
  if (trimmed.length === 16 || trimmed.length === 32) {
    const bytes = new Uint8Array(trimmed.length);
    for (let i = 0; i < trimmed.length; i++) {
      bytes[i] = trimmed.charCodeAt(i) & 0xff;
    }
    return {
      mnemonic: decodeCompactSeedQr(bytes),
      format: "compact-seedqr",
    };
  }

  throw new Error(
    "Unrecognized QR — expect Sparrow/SeedSigner SeedQR, CompactSeedQR, or BIP-39 words",
  );
}
