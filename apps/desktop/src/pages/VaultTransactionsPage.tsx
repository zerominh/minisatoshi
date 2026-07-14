import { formatSats } from "../lib/settings";
import { useVault } from "../vault/VaultContext";

export function VaultTransactionsPage() {
  const { sync, busy, runSync } = useVault();

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Transactions</h2>
          <p>History for this vault after chain sync.</p>
        </div>
        <button type="button" disabled={busy} onClick={() => void runSync()}>
          {busy ? "Syncing…" : "Sync chain"}
        </button>
      </header>

      <div className="grid-2">
        <div className="panel">
          <h3>Balance</h3>
          {sync ? (
            <>
              <p>
                <strong>{formatSats(sync.balance.confirmedSats)}</strong>{" "}
                confirmed
              </p>
              <p className="muted">
                {formatSats(sync.balance.unconfirmedSats)} unconfirmed
              </p>
            </>
          ) : (
            <p className="muted">Sync to load balance.</p>
          )}
        </div>
        <div className="panel">
          <h3>UTXO count</h3>
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
        <h3>History</h3>
        {!sync ? (
          <p className="muted">Sync the chain to load transactions.</p>
        ) : sync.history.length === 0 ? (
          <p className="muted">No transactions yet.</p>
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
                        : "Confirmed"
                      : "Unconfirmed / mempool"}
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
