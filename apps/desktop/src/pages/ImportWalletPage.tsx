import { FormEvent, useEffect, useRef, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { formatError, importWalletBackup, listWorkspaces } from "../lib/api";
import { coalesceDescriptorPaste } from "../lib/qrChunks";
import {
  formatNetwork,
  getActiveWorkspaceId,
  setActiveWorkspaceId,
} from "../lib/settings";
import type { WorkspaceSummaryDto } from "../lib/types";

export function ImportWalletPage() {
  const navigate = useNavigate();
  const fileRef = useRef<HTMLInputElement>(null);
  const [workspaces, setWorkspaces] = useState<WorkspaceSummaryDto[]>([]);
  const [workspaceId, setWorkspaceId] = useState(getActiveWorkspaceId() ?? "");
  const [name, setName] = useState("");
  const [payload, setPayload] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void listWorkspaces()
      .then((items) => {
        setWorkspaces(items);
        if (!workspaceId && items[0]) {
          setWorkspaceId(items[0].id);
          setActiveWorkspaceId(items[0].id);
        }
      })
      .catch((err) => setError(formatError(err)));
  }, []);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    if (!workspaceId) {
      setError("Select a workspace first.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const wallet = await importWalletBackup({
        workspaceId,
        payload: coalesceDescriptorPaste(payload),
        name: name.trim() || null,
      });
      setActiveWorkspaceId(workspaceId);
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
          Target workspace
          <select
            value={workspaceId}
            onChange={(e) => {
              setWorkspaceId(e.target.value);
              setActiveWorkspaceId(e.target.value);
            }}
            required
          >
            {workspaces.map((workspace) => (
              <option key={workspace.id} value={workspace.id}>
                {workspace.name} ({formatNetwork(workspace.network)})
              </option>
            ))}
          </select>
        </label>
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
          Network must match the workspace when the payload includes one.
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
