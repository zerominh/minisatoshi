import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { formatError, getVault, syncVault } from "../lib/api";
import { formatTimelockLabel } from "../lib/duration";
import { formatNetwork, formatSats, getEsploraUrl } from "../lib/settings";
import type { SyncResultDto, VaultDto } from "../lib/types";

export function VaultDetailPage() {
  const { id = "" } = useParams();
  const [vault, setVault] = useState<VaultDto | null>(null);
  const [sync, setSync] = useState<SyncResultDto | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void getVault(id)
      .then(setVault)
      .catch((err) => setError(formatError(err)));
  }, [id]);

  async function onSync() {
    setBusy(true);
    setError(null);
    try {
      const result = await syncVault(id, getEsploraUrl() || undefined);
      setSync(result);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  if (!vault && !error) return <p className="muted">Loading vault…</p>;
  if (!vault) return <pre className="error">{error}</pre>;

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>{vault.name}</h2>
          <p>
            {vault.scriptType} · {formatNetwork(vault.policy.network)}
          </p>
        </div>
        <div className="row-actions">
          <Link className="button-link" to={`/vaults/${vault.id}/receive`}>
            Receive
          </Link>
          <Link className="button-link" to={`/vaults/${vault.id}/send`}>
            Send
          </Link>
          <button type="button" disabled={busy} onClick={() => void onSync()}>
            {busy ? "Syncing…" : "Sync chain"}
          </button>
        </div>
      </header>

      {error ? <pre className="error">{error}</pre> : null}

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
            <p className="muted">Sync to load balance from Esplora.</p>
          )}
        </div>
        <div className="panel">
          <h3>Policy</h3>
          <p className="mono">{vault.policy.policy.primary}</p>
          {vault.policy.policy.fallback ? (
            <p className="muted">
              Fallback {vault.policy.policy.fallback.allow} after{" "}
              {formatTimelockLabel(vault.policy.policy.fallback.after)}
            </p>
          ) : null}
        </div>
      </div>

      <div className="panel">
        <h3>Descriptor</h3>
        <p className="mono wrap">{vault.descriptor}</p>
      </div>

      <div className="grid-2">
        <div className="panel">
          <h3>UTXOs</h3>
          {!sync || sync.utxos.length === 0 ? (
            <p className="muted">None yet.</p>
          ) : (
            <ul className="list compact">
              {sync.utxos.map((utxo) => (
                <li key={`${utxo.txid}:${utxo.vout}`}>
                  {formatSats(utxo.valueSats)} · {utxo.txid.slice(0, 10)}…:
                  {utxo.vout}
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="panel">
          <h3>Recent TX</h3>
          {!sync || sync.history.length === 0 ? (
            <p className="muted">None yet.</p>
          ) : (
            <ul className="list compact">
              {sync.history.map((tx) => (
                <li key={tx.txid}>
                  {tx.amountSats >= 0 ? "+" : ""}
                  {formatSats(tx.amountSats)} · {tx.txid.slice(0, 12)}…
                  {tx.confirmed ? "" : " (mempool)"}
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </section>
  );
}
