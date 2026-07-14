import { FormEvent, useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { formatError, importVaultBackup, listWallets } from "../lib/api";
import {
  formatNetwork,
  getActiveWalletId,
  setActiveWalletId,
} from "../lib/settings";
import type { WalletSummaryDto } from "../lib/types";

export function ImportVaultPage() {
  const navigate = useNavigate();
  const [wallets, setWallets] = useState<WalletSummaryDto[]>([]);
  const [walletId, setWalletId] = useState(getActiveWalletId() ?? "");
  const [name, setName] = useState("");
  const [payload, setPayload] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void listWallets()
      .then((items) => {
        setWallets(items);
        if (!walletId && items[0]) {
          setWalletId(items[0].id);
          setActiveWalletId(items[0].id);
        }
      })
      .catch((err) => setError(formatError(err)));
  }, []);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    if (!walletId) {
      setError("Select a wallet first.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const vault = await importVaultBackup({
        walletId,
        payload,
        name: name.trim() || null,
      });
      setActiveWalletId(walletId);
      navigate(`/vaults/${vault.id}`);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Import vault</h2>
          <p>
            Paste a <span className="mono">minisatoshi-vault-v1.json</span>{" "}
            backup or a checksummed descriptor (`tr(…)#…`).
          </p>
        </div>
        <Link className="button-link" to="/vaults">
          Back to vaults
        </Link>
      </header>

      <form className="panel form-grid" onSubmit={(e) => void onSubmit(e)}>
        <label>
          Target wallet
          <select
            value={walletId}
            onChange={(e) => {
              setWalletId(e.target.value);
              setActiveWalletId(e.target.value);
            }}
            required
          >
            {wallets.map((wallet) => (
              <option key={wallet.id} value={wallet.id}>
                {wallet.name} ({formatNetwork(wallet.network)})
              </option>
            ))}
          </select>
        </label>
        <label>
          Name override (optional)
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Uses backup name / “Imported vault”"
          />
        </label>
        <label>
          Backup JSON or descriptor
          <textarea
            className="mono"
            rows={10}
            value={payload}
            onChange={(e) => setPayload(e.target.value)}
            placeholder='{"formatVersion":"minisatoshi-vault-v1", …} or tr(…)#checksum'
            required
          />
        </label>
        <p className="muted">
          Network must match the wallet. Checksum is verified (BIP-380). Policy
          JSON is restored when present in the backup.
        </p>
        {error ? <pre className="error">{error}</pre> : null}
        <button type="submit" disabled={busy || !payload.trim()}>
          {busy ? "Importing…" : "Import vault"}
        </button>
      </form>
    </section>
  );
}
