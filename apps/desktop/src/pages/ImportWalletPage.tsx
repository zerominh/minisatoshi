import { FormEvent, useRef, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { formatError, importWalletBackup } from "../lib/api";
import { coalesceDescriptorPaste } from "../lib/qrChunks";
import {
  formatNetwork,
  getPreferredNetwork,
} from "../lib/settings";
import { ensureWorkspaceForNetwork } from "../lib/workspaceAuto";
import type { NetworkName } from "../lib/types";

export function ImportWalletPage() {
  const navigate = useNavigate();
  const fileRef = useRef<HTMLInputElement>(null);
  const [network, setNetwork] = useState<NetworkName>(getPreferredNetwork());
  const [name, setName] = useState("");
  const [payload, setPayload] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const workspaceId = await ensureWorkspaceForNetwork(network);
      const wallet = await importWalletBackup({
        workspaceId,
        payload: coalesceDescriptorPaste(payload),
        name: name.trim() || null,
      });
      navigate(`/wallets/${wallet.id}`);
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
          <h2>Import wallet</h2>
          <p>
            Watch-only import — paste or load a descriptor,{" "}
            <span className="mono">minisatoshi-wallet-v1.json</span>, BSMS 1.0,
            or Liana/Nunchuk-ish JSON. Multi-QR paste uses{" "}
            <span className="mono">MSDESC1</span> framing.
          </p>
        </div>
        <Link className="button-link" to="/wallets">
          Back to wallets
        </Link>
      </header>

      <form className="panel form-grid" onSubmit={(e) => void onSubmit(e)}>
        <label>
          Network
          <select
            value={network}
            onChange={(e) => setNetwork(e.target.value as NetworkName)}
          >
            <option value="testnet">Testnet3</option>
            <option value="testnet4">Testnet4</option>
            <option value="signet">Signet</option>
            <option value="regtest">Regtest</option>
            <option value="mainnet">Mainnet</option>
          </select>
        </label>
        <p className="muted">
          Import into {formatNetwork(network)} (created automatically if needed).
        </p>
        <label>
          Name override (optional)
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Uses backup name / “Imported wallet”"
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
            placeholder='{"formatVersion":"minisatoshi-wallet-v1", …} · tr(…)#… · BSMS 1.0 · MSDESC1/1/2/…'
            required
          />
        </label>
        <p className="muted">
          When the payload includes a network, it must match the selection.
          Checksum is verified or computed (BIP-380). Imported wallets stay
          watch-only — no seeds are imported.
        </p>
        {error ? <pre className="error">{error}</pre> : null}
        <button type="submit" disabled={busy || !payload.trim()}>
          {busy ? "Importing…" : "Import watch-only wallet"}
        </button>
      </form>
    </section>
  );
}
