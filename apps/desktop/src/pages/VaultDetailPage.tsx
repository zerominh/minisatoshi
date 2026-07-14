import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import {
  deleteVault,
  exportVaultBackup,
  formatError,
  getVault,
  hwRegisterVault,
  prepareHwRegistration,
  syncVault,
} from "../lib/api";
import { saveTextFileWithDialog, sanitizedFilename } from "../lib/download";
import { formatTimelockLabel } from "../lib/duration";
import {
  formatNetwork,
  formatSats,
  getEsploraUrl,
  getHwFingerprint,
  getHwiPath,
  setHwFingerprint,
} from "../lib/settings";
import type {
  RegistrationPackageDto,
  SyncResultDto,
  VaultDto,
} from "../lib/types";

export function VaultDetailPage() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const [vault, setVault] = useState<VaultDto | null>(null);
  const [sync, setSync] = useState<SyncResultDto | null>(null);
  const [registration, setRegistration] =
    useState<RegistrationPackageDto | null>(null);
  const [regFingerprint, setRegFingerprint] = useState(getHwFingerprint());
  const [cosignerHints, setCosignerHints] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
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

  async function onSaveDescriptorFile() {
    if (!vault) return;
    setError(null);
    try {
      const filename = `${sanitizedFilename(vault.name)}-descriptor.txt`;
      const path = await saveTextFileWithDialog(
        filename,
        `${vault.descriptor}\n`,
      );
      if (path) {
        setMessage(
          `Saved to ${path} — for Core / Nunchuk / Minisatoshi import (Sparrow: fund address only)`,
        );
      }
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function onExportBackup() {
    setBusy(true);
    setError(null);
    try {
      const backup = await exportVaultBackup(id);
      const filename = `${sanitizedFilename(backup.name)}-minisatoshi-vault-v1.json`;
      const path = await saveTextFileWithDialog(filename, `${backup.json}\n`);
      if (path) {
        setMessage(
          `Backup saved to ${path} — restore via Vaults → Import vault (no SQLite needed).`,
        );
      }
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onDeleteVault() {
    if (!vault) return;
    const ok = window.confirm(
      `Delete vault “${vault.name}”? This removes local data only (not funds on-chain). Export a backup first if you need it.`,
    );
    if (!ok) return;
    setBusy(true);
    setError(null);
    try {
      await deleteVault(id);
      navigate("/vaults");
    } catch (err) {
      setError(formatError(err));
      setBusy(false);
    }
  }

  async function onPrepareRegistration() {
    setBusy(true);
    setError(null);
    try {
      const pkg = await prepareHwRegistration(id);
      setRegistration(pkg);
      setMessage(
        "Registration package ready — export for Coldcard/Ledger or try Register on device.",
      );
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onRegisterOnDevice() {
    if (!regFingerprint.trim()) {
      setError("Enter a device fingerprint (from Settings → Signing devices).");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      setHwFingerprint(regFingerprint.trim());
      const result = await hwRegisterVault({
        vaultId: id,
        fingerprint: regFingerprint.trim(),
        hwiPath: getHwiPath() || null,
      });
      setRegistration(result.package);
      setCosignerHints(result.cosignerHints);
      setMessage(result.message);
      if (!result.ok) {
        setError(result.message);
      }
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onSaveColdcardFile() {
    if (!registration) return;
    try {
      const filename = `${sanitizedFilename(registration.vaultName)}-coldcard.txt`;
      const path = await saveTextFileWithDialog(
        filename,
        registration.coldcardSdText,
      );
      if (path) setMessage(`Coldcard MicroSD file saved to ${path}`);
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function onSaveBip388File() {
    if (!registration) return;
    try {
      const filename = `${sanitizedFilename(registration.vaultName)}-bip388.json`;
      const path = await saveTextFileWithDialog(
        filename,
        `${JSON.stringify(registration.bip388, null, 2)}\n`,
      );
      if (path) setMessage(`BIP-388 policy saved to ${path}`);
    } catch (err) {
      setError(formatError(err));
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
            {vault.scriptType} · {formatNetwork(vault.policy.network)}{" "}
            <span className="badge watch-only">Watch-only</span>
          </p>
        </div>
        <div className="row-actions">
          <Link className="button-link" to={`/vaults/${vault.id}/receive`}>
            Receive
          </Link>
          <Link className="button-link" to={`/vaults/${vault.id}/share`}>
            Share
          </Link>
          <Link className="button-link" to={`/vaults/${vault.id}/send`}>
            Send
          </Link>
          <button type="button" disabled={busy} onClick={() => void onSync()}>
            {busy ? "Syncing…" : "Sync chain"}
          </button>
          <button
            type="button"
            className="secondary"
            disabled={busy}
            onClick={() => void onDeleteVault()}
          >
            Delete vault
          </button>
        </div>
      </header>

      {error ? <pre className="error">{error}</pre> : null}
      {message ? <p className="status">{message}</p> : null}

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
        <div className="row-actions">
          <button type="button" onClick={() => void onSaveDescriptorFile()}>
            Save descriptor file
          </button>
          <button
            type="button"
            className="secondary"
            disabled={busy}
            onClick={() => void onExportBackup()}
          >
            Export vault backup
          </button>
          <Link className="button-link" to={`/vaults/${vault.id}/share`}>
            Share (QR / BSMS)
          </Link>
        </div>
      </div>

      <div className="panel form-grid">
        <h3>Register on hardware</h3>
        <p className="muted">
          Map this vault to a BIP-388 wallet policy (Ledger) and Coldcard
          MicroSD text before the first hardware co-sign. See{" "}
          <span className="mono">docs/hardware-signing.md</span>.
        </p>
        <div className="row-actions">
          <button
            type="button"
            disabled={busy}
            onClick={() => void onPrepareRegistration()}
          >
            Prepare registration package
          </button>
        </div>
        {registration ? (
          <>
            <label>
              BIP-388 policy template
              <textarea
                className="mono"
                rows={4}
                readOnly
                value={registration.bip388.policy}
              />
            </label>
            <label>
              Device fingerprint
              <input
                className="mono"
                value={regFingerprint}
                onChange={(e) => setRegFingerprint(e.target.value)}
                placeholder="from Settings → Signing devices"
              />
            </label>
            <div className="row-actions">
              <button
                type="button"
                disabled={busy || !regFingerprint.trim()}
                onClick={() => void onRegisterOnDevice()}
              >
                Register on device
              </button>
              <button
                type="button"
                className="secondary"
                onClick={() => void onSaveColdcardFile()}
              >
                Save Coldcard MicroSD file
              </button>
              <button
                type="button"
                className="secondary"
                onClick={() => void onSaveBip388File()}
              >
                Save BIP-388 JSON
              </button>
            </div>
            {cosignerHints.length > 0 ? (
              <div>
                <h4>Primary path cosigners</h4>
                <ul className="list compact">
                  {cosignerHints.map((hint) => (
                    <li key={hint}>{hint}</li>
                  ))}
                </ul>
              </div>
            ) : null}
            <div>
              <h4>Vendor notes</h4>
              {registration.vendors.map((vendor) => (
                <div key={vendor.deviceType} className="muted">
                  <strong>{vendor.title}</strong>
                  <ul className="list compact">
                    {vendor.instructions.map((line) => (
                      <li key={line}>{line}</li>
                    ))}
                  </ul>
                </div>
              ))}
            </div>
          </>
        ) : null}
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
