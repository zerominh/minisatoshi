import type { MessageKey } from "../i18n/en";

type Translate = (key: MessageKey, vars?: Record<string, string | number>) => string;

/** Human-readable age for “last synced” tooltips. */
export function formatSyncAge(
  syncedAtMs: number | null | undefined,
  t: Translate,
  now = Date.now(),
): string {
  if (syncedAtMs == null || !Number.isFinite(syncedAtMs)) {
    return t("sync.notYet");
  }
  const sec = Math.max(0, Math.floor((now - syncedAtMs) / 1000));
  if (sec < 5) return t("sync.justNow");
  if (sec < 60) return t("sync.secondsAgo", { n: sec });
  const min = Math.floor(sec / 60);
  if (min < 60) return t("sync.minutesAgo", { n: min });
  const hr = Math.floor(min / 60);
  if (hr < 48) return t("sync.hoursAgo", { n: hr });
  const days = Math.floor(hr / 24);
  return t("sync.daysAgo", { n: days });
}
