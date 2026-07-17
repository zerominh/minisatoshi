import { FormEvent, useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import {
  deleteWallet,
  exportWalletBackup,
  formatError,
  getLedgerRegistrationStatus,
  hwRegisterWallet,
  prepareHwRegistration,
  renameHotWallet,
  renameWallet,
} from "../lib/api";
import { HwDevicePicker } from "../components/HwDevicePicker";
import { saveTextFileWithDialog, sanitizedFilename } from "../lib/download";
import { formatTimelockLabel } from "../lib/duration";
import {
  getHwFingerprint,
  getHwiPath,
  setHwFingerprint,
} from "../lib/settings";
import type { LedgerRegistrationStatusDto, RegistrationPackageDto } from "../lib/types";
import { useWallet } from "../wallet/WalletContext";

export function WalletSettingsPage() {
  const navigate = useNavigate();
  const {
    walletId,
    wallet,
    busy: walletBusy,
    setError,
    setMessage,
    kind,
    hotWalletId,
    refreshWallet,
  } = useWallet();
  const [localBusy, setLocalBusy] = useState(false);
  const [displayName, setDisplayName] = useState("");
  const [registration, setRegistration] =
    useState<RegistrationPackageDto | null>(null);
  const [regFingerprint, setRegFingerprint] = useState(getHwFingerprint());
  const [cosignerHints, setCosignerHints] = useState<string[]>([]);
  const [ledgerStatus, setLedgerStatus] =
    useState<LedgerRegistrationStatusDto | null>(null);

  const isAbcScriptPath =
    wallet?.descriptor.includes("and_v") ||
    wallet?.descriptor.includes("{{") ||
    false;

  const working = walletBusy || localBusy;

  useEffect(() => {
    setDisplayName(wallet?.name ?? "");
  }, [wallet?.name]);

  useEffect(() => {
    const fp = regFingerprint.trim();
    if (!walletId || !fp) {
      setLedgerStatus(null);
      return;
    }
    void getLedgerRegistrationStatus(walletId, fp)
      .then(setLedgerStatus)
      .catch(() => setLedgerStatus(null));
  }, [walletId, regFingerprint, registration?.ledgerHmac]);

  if (!wallet) return null;

  async function onRename(event: FormEvent) {
    event.preventDefault();
    const next = displayName.trim();
    if (!next || next === wallet!.name) return;
    setLocalBusy(true);
    setError(null);
    try {
      if (kind === "hot" && hotWalletId) {
        await renameHotWallet(hotWalletId, next);
      } else {
        await renameWallet(walletId, next);
      }
      await refreshWallet();
      setMessage(`Renamed to “${next}”`);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setLocalBusy(false);
    }
  }

  async function onSaveDescriptorFile() {
    setError(null);
    try {
      const filename = `${sanitizedFilename(wallet!.name)}-descriptor.txt`;
      const path = await saveTextFileWithDialog(
        filename,
        `${wallet!.descriptor}\n`,
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
    setLocalBusy(true);
    setError(null);
    try {
      const backup = await exportWalletBackup(walletId);
      const filename = `${sanitizedFilename(backup.name)}-minisatoshi-wallet-v1.json`;
      const path = await saveTextFileWithDialog(filename, `${backup.json}\n`);
      if (path) {
        setMessage(
          `Backup saved to ${path} — restore via Wallets → Import wallet.`,
        );
      }
    } catch (err) {
      setError(formatError(err));
    } finally {
      setLocalBusy(false);
    }
  }

  async function onDeleteWallet() {
    const name = wallet!.name;
    const ok = window.confirm(
      `Delete wallet “${name}”? This removes local data only (not funds on-chain). Export a backup first if you need it.`,
    );
    if (!ok) return;
    setLocalBusy(true);
    setError(null);
    try {
      await deleteWallet(walletId);
      navigate("/wallets");
    } catch (err) {
      setError(formatError(err));
      setLocalBusy(false);
    }
  }

  async function onPrepareRegistration() {
    setLocalBusy(true);
    setError(null);
    try {
      const pkg = await prepareHwRegistration(walletId);
      setRegistration(pkg);
      setMessage("Registration package ready.");
    } catch (err) {
      setError(formatError(err));
    } finally {
      setLocalBusy(false);
    }
  }

  async function onRegisterOnDevice() {
    if (!regFingerprint.trim()) {
      setError("Enter a device fingerprint (from Settings → Signing devices).");
      return;
    }
    setLocalBusy(true);
    setError(null);
    try {
      setHwFingerprint(regFingerprint.trim());
      const result = await hwRegisterWallet({
        walletId,
        fingerprint: regFingerprint.trim(),
        hwiPath: getHwiPath() || null,
      });
      setRegistration(result.package);
      setCosignerHints(result.cosignerHints);
      setMessage(result.message);
      if (regFingerprint.trim()) {
        const status = await getLedgerRegistrationStatus(
          walletId,
          regFingerprint.trim(),
        );
        setLedgerStatus(status);
      }
    } catch (err) {
      setError(formatError(err));
    } finally {
      setLocalBusy(false);
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

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Settings</h2>
          <p>Policy, descriptor, hardware registration, and danger zone.</p>
        </div>
      </header>

      <form className="panel form-grid" onSubmit={(e) => void onRename(e)}>
        <h3>Name</h3>
        <label>
          Display name
          <input
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            required
          />
        </label>
        <button
          type="submit"
          disabled={
            working || !displayName.trim() || displayName.trim() === wallet.name
          }
        >
          {working ? "…" : "Save name"}
        </button>
      </form>

      <div className="panel">
        <h3>Policy</h3>
        <p className="mono">{wallet.policy.policy.primary}</p>
        {wallet.policy.policy.fallback ? (
          <p className="muted">
            Fallback {wallet.policy.policy.fallback.allow} after{" "}
            {formatTimelockLabel(wallet.policy.policy.fallback.after)}
          </p>
        ) : null}
        {(wallet.policy.policy.fallbacks ?? []).map((fb) => (
          <p key={`${fb.allow}-${fb.after}`} className="muted">
            Fallback {fb.allow} after {formatTimelockLabel(fb.after)}
          </p>
        ))}
      </div>

      <div className="panel">
        <h3>Descriptor</h3>
        <p className="mono wrap">{wallet.descriptor}</p>
        <div className="row-actions">
          <button type="button" onClick={() => void onSaveDescriptorFile()}>
            Save descriptor file
          </button>
          <button
            type="button"
            className="secondary"
            disabled={working}
            onClick={() => void onExportBackup()}
          >
            Export wallet backup
          </button>
          <Link
            className="button-link"
            to={
              kind === "hot" && hotWalletId
                ? `/hot-wallets/${hotWalletId}/share`
                : `/wallets/${wallet.id}/share`
            }
          >
            Share (QR / BSMS)
          </Link>
        </div>
      </div>

      <div className="panel form-grid">
        <h3>Register on hardware</h3>
        <p className="muted">
          {isAbcScriptPath ? (
            <>
              For Miniscript Taproot (ABC) with <strong>Ledger</strong>: install the{" "}
              <strong>Ledger signer</strong> in Settings → Signing devices, then use{" "}
              <strong>Register Ledger policy</strong> below. Then sign PSBTs in Send.
            </>
          ) : (
            <>
              Register or verify the wallet policy on your hardware device before
              the first co-sign.
            </>
          )}
        </p>
        <div className="row-actions">
          <button
            type="button"
            disabled={working}
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
              <HwDevicePicker
                network={wallet.policy.network}
                fingerprint={regFingerprint}
                onFingerprintChange={setRegFingerprint}
                walletFingerprints={wallet.policy.keys.map((k) => k.fingerprint)}
                onError={setError}
                disabled={working}
              />
            </label>
            {ledgerStatus ? (
              <p className="muted">
                Ledger policy:{" "}
                {ledgerStatus.stale ? (
                  <>
                    <strong>stale</strong>
                    {ledgerStatus.staleReason
                      ? ` — ${ledgerStatus.staleReason}`
                      : " — re-register required"}
                  </>
                ) : ledgerStatus.registered ? (
                  <strong>registered</strong>
                ) : (
                  <strong>not registered</strong>
                )}
                {!ledgerStatus.registered && !ledgerStatus.stale
                  ? !ledgerStatus.pythonAvailable
                    ? " — install Ledger signer in Settings"
                    : null
                  : null}
              </p>
            ) : null}
            <div className="row-actions">
              <button
                type="button"
                disabled={working || !regFingerprint.trim()}
                onClick={() => void onRegisterOnDevice()}
              >
                {isAbcScriptPath
                  ? "Register Ledger policy"
                  : "Verify device / register"}
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

      <div className="panel">
        <h3>Danger zone</h3>
        <button
          type="button"
          className="secondary"
          disabled={working}
          onClick={() => void onDeleteWallet()}
        >
          Delete wallet
        </button>
      </div>
    </section>
  );
}
