/** Format unix seconds for transaction list (local timezone). */
export function formatTxTime(
  blockTime: number | null | undefined,
  locale?: string,
): string | null {
  if (blockTime == null || !Number.isFinite(blockTime) || blockTime <= 0) {
    return null;
  }
  const date = new Date(blockTime * 1000);
  if (Number.isNaN(date.getTime())) return null;
  return date.toLocaleString(locale, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
