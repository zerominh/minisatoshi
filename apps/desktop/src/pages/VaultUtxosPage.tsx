import { formatSats } from "../lib/settings";
import { useVault } from "../vault/VaultContext";

export function VaultUtxosPage() {
  const { sync, busy, runSync } = useVault();

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>UTXOs</h2>
          <p>Spendable outputs — also used for coin control on Send.</p>
        </div>
        <button type="button" disabled={busy} onClick={() => void runSync()}>
          {busy ? "Syncing…" : "Sync chain"}
        </button>
      </header>

      <div className="panel">
        {!sync ? (
          <p className="muted">Sync to load UTXOs.</p>
        ) : sync.utxos.length === 0 ? (
          <p className="muted">No UTXOs.</p>
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
                    {utxo.isChange ? " · change" : " · receive"}
                    {utxo.confirmed ? "" : " · unconfirmed"}
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
