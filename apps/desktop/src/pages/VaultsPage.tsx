import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { formatError, listVaults, listWallets } from "../lib/api";
import { formatNetwork, getActiveWalletId, setActiveWalletId } from "../lib/settings";
import type { VaultSummaryDto, WalletSummaryDto } from "../lib/types";

export function VaultsPage() {
  const [wallets, setWallets] = useState<WalletSummaryDto[]>([]);
  const [walletId, setWalletId] = useState<string | null>(getActiveWalletId());
  const [vaults, setVaults] = useState<VaultSummaryDto[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        const items = await listWallets();
        setWallets(items);
        const selected =
          walletId && items.some((w) => w.id === walletId)
            ? walletId
            : (items[0]?.id ?? null);
        if (selected !== walletId) {
          setWalletId(selected);
          if (selected) setActiveWalletId(selected);
        }
      } catch (err) {
        setError(formatError(err));
      }
    })();
  }, []);

  useEffect(() => {
    if (!walletId) {
      setVaults([]);
      return;
    }
    void listVaults(walletId)
      .then(setVaults)
      .catch((err) => setError(formatError(err)));
  }, [walletId]);

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Vaults</h2>
          <p>Policy-backed Taproot vaults for the active wallet.</p>
        </div>
        <div className="row-actions">
          <Link className="button-link" to="/vaults/import">
            Import vault
          </Link>
          <Link className="button-link primary" to="/vaults/new">
            New vault
          </Link>
        </div>
      </header>

      {wallets.length === 0 ? (
        <div className="panel">
          <p className="muted">Create a wallet first.</p>
          <Link className="button-link" to="/wallets">
            Go to wallets
          </Link>
        </div>
      ) : (
        <>
          <div className="panel form-grid">
            <label>
              Active wallet
              <select
                value={walletId ?? ""}
                onChange={(e) => {
                  setWalletId(e.target.value);
                  setActiveWalletId(e.target.value);
                }}
              >
                {wallets.map((wallet) => (
                  <option key={wallet.id} value={wallet.id}>
                    {wallet.name} ({formatNetwork(wallet.network)})
                  </option>
                ))}
              </select>
            </label>
          </div>

          {error ? <pre className="error">{error}</pre> : null}

          <div className="panel">
            {vaults.length === 0 ? (
              <p className="muted">No vaults in this wallet yet.</p>
            ) : (
              <ul className="list">
                {vaults.map((vault) => (
                  <li key={vault.id} className="list-item">
                    <div>
                      <strong>{vault.name}</strong>
                      <div className="muted">{vault.scriptType}</div>
                    </div>
                    <div className="row-actions">
                      <Link className="button-link" to={`/vaults/${vault.id}`}>
                        Open
                      </Link>
                      <Link
                        className="button-link"
                        to={`/vaults/${vault.id}/receive`}
                      >
                        Receive
                      </Link>
                      <Link
                        className="button-link"
                        to={`/vaults/${vault.id}/send`}
                      >
                        Send
                      </Link>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </>
      )}
    </section>
  );
}
