import { useCallback, useState } from "react";
import { useT } from "../i18n/LocaleContext";
import { formatSyncAge } from "../lib/formatSyncAge";
import { formatSats } from "../lib/settings";
import { useVault } from "../vault/VaultContext";

export function VaultUtxosPage() {
  const t = useT();
  const { sync, lastSyncedAt, busy, runSync } = useVault();
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
          <h2>{t("utxos.title")}</h2>
          <p>{t("utxos.subtitle")}</p>
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

      <div className="panel">
        {!sync ? (
          <p className="muted">{t("utxos.syncToLoad")}</p>
        ) : sync.utxos.length === 0 ? (
          <p className="muted">{t("utxos.empty")}</p>
        ) : (
          <ul className="list">
            {sync.utxos.map((utxo) => (
              <li key={`${utxo.txid}:${utxo.vout}`} className="list-item">
                <div>
                  <strong>{formatSats(utxo.valueSats)}</strong>
                  <div className="mono wrap muted">
                    {utxo.txid}:{utxo.vout}
                  </div>
                  <div className="muted">
                    idx {utxo.derivationIndex}
                    {utxo.isChange
                      ? ` · ${t("utxos.change")}`
                      : ` · ${t("utxos.receive")}`}
                    {utxo.confirmed ? "" : ` · ${t("utxos.unconfirmed")}`}
                  </div>
                  <div className="mono wrap muted">{utxo.address}</div>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}
