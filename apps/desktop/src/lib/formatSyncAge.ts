/** Human-readable age for “last synced” tooltips. */
export function formatSyncAge(syncedAtMs: number | null | undefined, now = Date.now()): string {
  if (syncedAtMs == null || !Number.isFinite(syncedAtMs)) {
    return "Not synced yet — click to sync";
  }
  const sec = Math.max(0, Math.floor((now - syncedAtMs) / 1000));
  if (sec < 5) return "Last sync: just now";
  if (sec < 60) return `Last sync: ${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `Last sync: ${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 48) return `Last sync: ${hr}h ago`;
  const days = Math.floor(hr / 24);
  return `Last sync: ${days}d ago`;
}
