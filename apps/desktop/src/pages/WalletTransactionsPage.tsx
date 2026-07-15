import { useCallback, useState } from "react";
import { useT } from "../i18n/LocaleContext";
import { formatSyncAge } from "../lib/formatSyncAge";
import { formatSats } from "../lib/settings";
import { useWallet } from "../wallet/WalletContext";

export function WalletTransactionsPage() {
  const t = useT();
  const { sync, lastSyncedAt, busy, runSync } = useWallet();
  const [syncTitle, setSyncTitle] = useState(() =>
    formatSyncAge(lastSyncedAt, t),
  );
  const refreshSyncTitle = useCallback(() => {
    setSyncTitle(formatSyncAge(lastSyncedAt, t));
  }, [lastSyncedAt, t]);

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
        ) : sync.history.length === 0 ? (
          <p className="muted">{t("tx.empty")}</p>
        ) : (
          <ul className="list">
            {sync.history.map((tx) => (
              <li key={tx.txid} className="list-item">
                <div>
                  <span className="mono">{tx.txid}</span>
                  <div className="muted">
                    {tx.confirmed
                      ? tx.blockHeight != null
                        ? `Block ${tx.blockHeight}`
                        : t("tx.confirmed")
                      : t("tx.unconfirmed")}
                  </div>
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
