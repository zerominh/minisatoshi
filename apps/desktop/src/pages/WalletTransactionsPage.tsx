import { useCallback, useMemo, useState } from "react";
import type { MessageKey } from "../i18n/en";
import { useLocale, useT } from "../i18n/LocaleContext";
import { formatSyncAge } from "../lib/formatSyncAge";
import { formatTxTime } from "../lib/formatTxTime";
import { formatSats } from "../lib/settings";
import type { TxSummaryDto } from "../lib/types";
import { useWallet } from "../wallet/WalletContext";

function txMetaLine(
  tx: TxSummaryDto,
  t: (key: MessageKey) => string,
  locale: string,
): string {
  if (!tx.confirmed) return t("tx.unconfirmed");
  const parts: string[] = [];
  if (tx.blockHeight != null) {
    parts.push(`Block ${tx.blockHeight}`);
  } else {
    parts.push(t("tx.confirmed"));
  }
  const when = formatTxTime(tx.blockTime, locale);
  if (when) parts.push(when);
  return parts.join(" · ");
}

export function WalletTransactionsPage() {
  const t = useT();
  const { locale } = useLocale();
  const { sync, lastSyncedAt, busy, runSync } = useWallet();
  const [syncTitle, setSyncTitle] = useState(() =>
    formatSyncAge(lastSyncedAt, t),
  );
  const refreshSyncTitle = useCallback(() => {
    setSyncTitle(formatSyncAge(lastSyncedAt, t));
  }, [lastSyncedAt, t]);

  const history = useMemo(() => {
    if (!sync) return [];
    return [...sync.history].sort((a, b) => {
      const ta = a.blockTime ?? (a.confirmed ? 0 : Number.MAX_SAFE_INTEGER);
      const tb = b.blockTime ?? (b.confirmed ? 0 : Number.MAX_SAFE_INTEGER);
      if (ta !== tb) return tb - ta;
      return (b.blockHeight ?? 0) - (a.blockHeight ?? 0);
    });
  }, [sync]);

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>{t("tx.title")}</h2>
          <p>{t("tx.historyHint")}</p>
        </div>
        <button
          type="button"
          disabled={busy}
          onClick={() => void runSync()}
          onMouseEnter={refreshSyncTitle}
          onFocus={refreshSyncTitle}
          title={syncTitle}
        >
          {busy ? t("common.syncing") : t("sync.syncChain")}
        </button>
      </header>

      <div className="grid-2">
        <div className="panel">
          <h3>{t("tx.balance")}</h3>
          {sync ? (
            <>
              <p>
                <strong>{formatSats(sync.balance.confirmedSats)}</strong>{" "}
                {t("tx.confirmed")}
              </p>
              <p className="muted">
                {formatSats(sync.balance.unconfirmedSats)} {t("tx.unconfirmed")}
              </p>
            </>
          ) : (
            <p className="muted">{t("tx.syncToLoad")}</p>
          )}
        </div>
        <div className="panel">
          <h3>{t("tx.utxoCount")}</h3>
          <p>
            {sync ? (
              <strong>{sync.utxos.length}</strong>
            ) : (
              <span className="muted">—</span>
            )}
          </p>
        </div>
      </div>

      <div className="panel">
        <h3>{t("tx.title")}</h3>
        {!sync ? (
          <p className="muted">{t("tx.syncToLoad")}</p>
        ) : history.length === 0 ? (
          <p className="muted">{t("tx.empty")}</p>
        ) : (
          <ul className="list">
            {history.map((tx) => (
              <li key={tx.txid} className="list-item">
                <div>
                  <span className="mono">{tx.txid}</span>
                  <div className="muted">{txMetaLine(tx, t, locale)}</div>
                </div>
                <strong>
                  {tx.amountSats > 0 ? "+" : ""}
                  {formatSats(tx.amountSats)}
                </strong>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}
