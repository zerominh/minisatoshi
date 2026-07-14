/**
 * Simple multi-QR framing for long descriptors (not UR).
 * Format: `MSDESC1/<i>/<n>/<payload>`
 * where i is 1-based chunk index and n is total chunks.
 */

export const QR_CHUNK_PREFIX = "MSDESC1";
/** Keep under common phone-camera capacity with modest ECC. */
export const QR_CHUNK_SIZE = 400;

export function splitDescriptorQrChunks(
  descriptor: string,
  chunkSize = QR_CHUNK_SIZE,
): string[] {
  const payload = descriptor.trim();
  if (!payload) return [];
  if (payload.length <= chunkSize) {
    return [`${QR_CHUNK_PREFIX}/1/1/${payload}`];
  }
  const parts: string[] = [];
  for (let offset = 0; offset < payload.length; offset += chunkSize) {
    parts.push(payload.slice(offset, offset + chunkSize));
  }
  const n = parts.length;
  return parts.map((part, i) => `${QR_CHUNK_PREFIX}/${i + 1}/${n}/${part}`);
}

export type QrChunkParse =
  | { kind: "framed"; index: number; total: number; payload: string }
  | { kind: "raw"; payload: string };

export function parseQrChunk(text: string): QrChunkParse {
  const trimmed = text.trim();
  const match = trimmed.match(/^MSDESC1\/(\d+)\/(\d+)\/([\s\S]*)$/);
  if (match) {
    return {
      kind: "framed",
      index: Number(match[1]),
      total: Number(match[2]),
      payload: match[3],
    };
  }
  return { kind: "raw", payload: trimmed };
}

/** Merge scanned / pasted framed chunks; returns null until complete. */
export function mergeQrChunks(
  frames: Array<{ index: number; total: number; payload: string }>,
): string | null {
  if (frames.length === 0) return null;
  const total = frames[0].total;
  if (total < 1 || frames.some((f) => f.total !== total)) return null;
  const byIndex = new Map<number, string>();
  for (const frame of frames) {
    if (frame.index < 1 || frame.index > total) return null;
    byIndex.set(frame.index, frame.payload);
  }
  if (byIndex.size !== total) return null;
  const parts: string[] = [];
  for (let i = 1; i <= total; i++) {
    const part = byIndex.get(i);
    if (part === undefined) return null;
    parts.push(part);
  }
  return parts.join("");
}

/** Accept a paste that may contain one or more MSDESC1 lines (or a raw descriptor). */
export function coalesceDescriptorPaste(text: string): string {
  const trimmed = text.trim();
  if (!trimmed) return "";
  const frames: Array<{ index: number; total: number; payload: string }> = [];
  for (const line of trimmed.split(/\r?\n/)) {
    const parsed = parseQrChunk(line);
    if (parsed.kind === "framed") {
      frames.push({
        index: parsed.index,
        total: parsed.total,
        payload: parsed.payload,
      });
    }
  }
  if (frames.length > 0) {
    return mergeQrChunks(frames) ?? trimmed;
  }
  const single = parseQrChunk(trimmed);
  if (single.kind === "framed") {
    return mergeQrChunks([
      { index: single.index, total: single.total, payload: single.payload },
    ]) ?? trimmed;
  }
  return trimmed;
}
