import { FormEvent, useEffect, useRef, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { formatError, importVaultBackup, listWallets } from "../lib/api";
import { coalesceDescriptorPaste } from "../lib/qrChunks";
import {
  formatNetwork,
  getActiveWalletId,
  setActiveWalletId,
} from "../lib/settings";
import type { WalletSummaryDto } from "../lib/types";

export function ImportVaultPage() {
  const navigate = useNavigate();
  const fileRef = useRef<HTMLInputElement>(null);
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
        payload: coalesceDescriptorPaste(payload),
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

  async function onPickFile(file: File | null) {
    if (!file) return;
    setError(null);
    try {
      const text = await file.text();
      setPayload(coalesceDescriptorPaste(text));
      if (!name.trim()) {
        const base = file.name.replace(/\.(json|txt|bsms|dat)$/i, "");
        if (base) setName(base);
      }
    } catch (err) {
      setError(formatError(err));
    }
  }

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Import vault</h2>
          <p>
            Watch-only import — paste or load a descriptor,{" "}
            <span className="mono">minisatoshi-vault-v1.json</span>, BSMS 1.0,
            or Liana/Nunchuk-ish JSON. Multi-QR paste uses{" "}
            <span className="mono">MSDESC1</span> framing.
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
        <div className="row-actions">
          <button
            type="button"
            className="secondary"
            onClick={() => fileRef.current?.click()}
          >
            Load file…
          </button>
          <input
            ref={fileRef}
            type="file"
            accept=".json,.txt,.bsms,.dat,text/plain,application/json"
            hidden
            onChange={(e) => void onPickFile(e.target.files?.[0] ?? null)}
          />
        </div>
        <label>
          Backup / BSMS / descriptor / multi-QR paste
          <textarea
            className="mono"
            rows={10}
            value={payload}
            onChange={(e) => setPayload(e.target.value)}
            placeholder='{"formatVersion":"minisatoshi-vault-v1", …} · tr(…)#… · BSMS 1.0 · MSDESC1/1/2/…'
            required
          />
        </label>
        <p className="muted">
          Network must match the wallet when the payload includes one. Checksum is
          verified or computed (BIP-380). Imported vaults stay watch-only — no
          seeds are imported.
        </p>
        {error ? <pre className="error">{error}</pre> : null}
        <button type="submit" disabled={busy || !payload.trim()}>
          {busy ? "Importing…" : "Import watch-only vault"}
        </button>
      </form>
    </section>
  );
}
