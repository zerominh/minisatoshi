import { FormEvent, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { createWallet, formatError, listWallets } from "../lib/api";
import {
  getActiveWalletId,
  getPreferredNetwork,
  setActiveWalletId,
} from "../lib/settings";
import type { NetworkName, WalletSummaryDto } from "../lib/types";

export function WalletsPage() {
  const [wallets, setWallets] = useState<WalletSummaryDto[]>([]);
  const [name, setName] = useState("");
  const [network, setNetwork] = useState<NetworkName>(getPreferredNetwork());
  const [activeId, setActiveId] = useState<string | null>(getActiveWalletId());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    const items = await listWallets();
    setWallets(items);
    if (!activeId && items.length > 0) {
      setActiveWalletId(items[0].id);
      setActiveId(items[0].id);
    }
  }

  useEffect(() => {
    void refresh().catch((err) => setError(formatError(err)));
  }, []);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const wallet = await createWallet({ name, network });
      setActiveWalletId(wallet.id);
      setActiveId(wallet.id);
      setName("");
      await refresh();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  function selectWallet(id: string) {
    setActiveWalletId(id);
    setActiveId(id);
  }

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Wallets</h2>
          <p>Local SQLite wallets — offline-first, watch-only.</p>
        </div>
      </header>

      <form className="panel form-grid" onSubmit={(e) => void onSubmit(e)}>
        <h3>Create wallet</h3>
        <label>
          Name
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Family fund"
            required
          />
        </label>
        <label>
          Network
          <select
            value={network}
            onChange={(e) => setNetwork(e.target.value as NetworkName)}
          >
            <option value="testnet">Testnet</option>
            <option value="signet">Signet</option>
            <option value="regtest">Regtest</option>
            <option value="mainnet">Mainnet</option>
          </select>
        </label>
        <button type="submit" disabled={busy || !name.trim()}>
          {busy ? "Creating…" : "Create wallet"}
        </button>
      </form>

      {error ? <pre className="error">{error}</pre> : null}

      <div className="panel">
        <h3>Your wallets</h3>
        {wallets.length === 0 ? (
          <p className="muted">No wallets yet.</p>
        ) : (
          <ul className="list">
            {wallets.map((wallet) => (
              <li key={wallet.id} className="list-item">
                <div>
                  <strong>{wallet.name}</strong>
                  <div className="muted">
                    {wallet.network} · {wallet.vaultCount} vault
                    {wallet.vaultCount === 1 ? "" : "s"}
                  </div>
                </div>
                <div className="row-actions">
                  {activeId === wallet.id ? (
                    <span className="badge">Active</span>
                  ) : (
                    <button
                      type="button"
                      className="secondary"
                      onClick={() => selectWallet(wallet.id)}
                    >
                      Use
                    </button>
                  )}
                  <Link className="button-link" to="/vaults">
                    Vaults
                  </Link>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}
