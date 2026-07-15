import { FormEvent, useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  PsbtSignMethodPanel,
  type SignMethod,
} from "../components/PsbtSignMethodPanel";
import {
  analyzePsbtStatus,
  broadcastPsbt,
  combinePsbts,
  finalizePsbt,
  formatError,
  getVault,
  hotKeystoreStatus,
  hwSignPsbt,
  importPsbt,
  listHotWallets,
  listSpendingPaths,
  signPsbtHot,
  signPsbtSoftware,
} from "../lib/api";
import {
  copyText,
  formatNetwork,
  getEsploraUrl,
  getHwFingerprint,
  getHwiPath,
} from "../lib/settings";
import {
  openPsbtFileWithDialog,
  savePsbtFileWithDialog,
  sanitizedFilename,
} from "../lib/download";
import type {
  FinalizedTxDto,
  HotWalletSummaryDto,
  PsbtDto,
  SigningStatusDto,
  SpendingPathDto,
  VaultDto,
} from "../lib/types";
import { useVault, useVaultIdFromRouteOrContext } from "../vault/VaultContext";

type Step = "import" | "sign";

/**
 * Cosigner / air-gap: import a PSBT created elsewhere, sign locally, copy back.
 * Sparrow often cannot sign Miniscript Taproot vaults — use this instead.
 */
export function SignPsbtPage() {
  const id = useVaultIdFromRouteOrContext();
  const {
    vault: shellVault,
    setError,
    setMessage,
    runSync,
  } = useVault();
  const [vault, setVault] = useState<VaultDto | null>(shellVault);
  const [step, setStep] = useState<Step>("import");
  const [importBase64, setImportBase64] = useState("");
  const [paths, setPaths] = useState<SpendingPathDto[]>([]);
  const [activePathId, setActivePathId] = useState("");
  const [psbt, setPsbt] = useState<PsbtDto | null>(null);
  const [signStatus, setSignStatus] = useState<SigningStatusDto | null>(null);
  const [secretKey, setSecretKey] = useState("");
  const [hotWallets, setHotWallets] = useState<HotWalletSummaryDto[]>([]);
  const [hotWalletId, setHotWalletId] = useState("");
  const [signMethod, setSignMethod] = useState<SignMethod>("hardware");
  const [hwFingerprint, setHwFingerprint] = useState(getHwFingerprint());
  const [allowMainnetHotKeys, setAllowMainnetHotKeys] = useState(false);
  const [confirmMainnetHot, setConfirmMainnetHot] = useState(false);
  const [cosignerPsbt, setCosignerPsbt] = useState("");
  const [finalized, setFinalized] = useState<FinalizedTxDto | null>(null);
  const [broadcastConfirm, setBroadcastConfirm] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (shellVault) setVault(shellVault);
  }, [shellVault]);

  useEffect(() => {
    void getVault(id)
      .then((v) => {
        setVault(v);
        return listSpendingPaths(id);
      })
      .then((list) => {
        setPaths(list);
        setActivePathId(list[0]?.id ?? "");
      })
      .catch((err) => setError(formatError(err)));
  }, [id, setError]);

  useEffect(() => {
    void (async () => {
      try {
        const st = await hotKeystoreStatus();
        if (!st.unlocked) {
          setHotWallets([]);
          return;
        }
        const list = await listHotWallets();
        setHotWallets(list);
        const linked = list.find((h) => h.linkedVaultId === id);
        if (linked) {
          setHotWalletId(linked.id);
          setSignMethod("hot");
        }
      } catch {
        setHotWallets([]);
      }
    })();
  }, [id]);

  const refreshStatus = useCallback(
    async (base64: string, pathId: string) => {
      try {
        const status = await analyzePsbtStatus({
          vaultId: id,
          psbtBase64: base64,
          activePathId: pathId || null,
        });
        setSignStatus(status);
      } catch {
        setSignStatus(null);
      }
    },
    [id],
  );

  function onSelectPath(pathId: string) {
    setActivePathId(pathId);
    if (psbt) void refreshStatus(psbt.base64, pathId);
  }

  function resetImport() {
    setStep("import");
    setPsbt(null);
    setSignStatus(null);
    setFinalized(null);
    setBroadcastConfirm(false);
    setSecretKey("");
    setCosignerPsbt("");
    setError(null);
    setMessage(null);
  }

  async function onImport(event: FormEvent) {
    event.preventDefault();
    await loadImportedPsbt(importBase64);
  }

  async function onImportFromFile() {
    setError(null);
    try {
      const file = await openPsbtFileWithDialog();
      if (!file) return;
      setImportBase64(file.contents.trim());
      await loadImportedPsbt(file.contents);
      setMessage(`Loaded ${file.path}`);
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function loadImportedPsbt(rawInput: string) {
    const raw = rawInput.trim();
    if (!raw) {
      setError("Paste a base64 PSBT or open a .psbt file");
      return;
    }
    setBusy(true);
    setError(null);
    setMessage(null);
    setFinalized(null);
    setBroadcastConfirm(false);
    try {
      const result = await importPsbt(raw);
      setPsbt(result);
      setImportBase64("");
      await refreshStatus(result.base64, activePathId);
      setStep("sign");
      setMessage(
        `Imported (${result.inputCount} in / ${result.outputCount} out) — sign, then Copy or Export PSBT back to the creator.`,
      );
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onExportPsbt() {
    if (!psbt || !vault) return;
    setError(null);
    try {
      const filename = `${sanitizedFilename(vault.name)}-partial.psbt`;
      const path = await savePsbtFileWithDialog(filename, psbt.base64);
      if (path) setMessage(`PSBT saved to ${path}`);
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function onSignSoftware() {
    if (!psbt || !vault) return;
    if (vault.policy.network === "mainnet") {
      if (!allowMainnetHotKeys || !confirmMainnetHot) {
        setError("Mainnet hot-key signing requires both confirmation checkboxes.");
        return;
      }
    }
    setBusy(true);
    setError(null);
    try {
      const signed = await signPsbtSoftware({
        psbtBase64: psbt.base64,
        secretKey: secretKey.trim(),
        network: vault.policy.network,
        allowMainnetHotKeys,
      });
      const next = {
        base64: signed.base64,
        inputCount: signed.inputCount,
        outputCount: signed.outputCount,
      };
      setPsbt(next);
      setSecretKey("");
      await refreshStatus(next.base64, activePathId);
      setMessage(
        `Software signed ${signed.signedInputs}/${signed.totalInputs} input(s).`,
      );
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onSignHot() {
    if (!psbt || !vault || !hotWalletId) return;
    if (vault.policy.network === "mainnet") {
      if (!allowMainnetHotKeys || !confirmMainnetHot) {
        setError("Mainnet hot-key signing requires both confirmation checkboxes.");
        return;
      }
    }
    setBusy(true);
    setError(null);
    try {
      const signed = await signPsbtHot({
        psbtBase64: psbt.base64,
        hotWalletId,
        network: vault.policy.network,
        allowMainnetHotKeys,
      });
      const next = {
        base64: signed.base64,
        inputCount: signed.inputCount,
        outputCount: signed.outputCount,
      };
      setPsbt(next);
      await refreshStatus(next.base64, activePathId);
      setMessage(
        `Hot wallet signed ${signed.signedInputs}/${signed.totalInputs} input(s).`,
      );
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onHwSign() {
    if (!psbt || !hwFingerprint.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const signed = await hwSignPsbt({
        fingerprint: hwFingerprint.trim(),
        psbtBase64: psbt.base64,
        hwiPath: getHwiPath() || null,
      });
      const next = {
        base64: signed.base64,
        inputCount: signed.inputCount,
        outputCount: signed.outputCount,
      };
      setPsbt(next);
      await refreshStatus(next.base64, activePathId);
      setMessage(
        `Hardware signed ${signed.signedInputs}/${signed.totalInputs} input(s).`,
      );
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onCombine() {
    if (!psbt || !cosignerPsbt.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const combined = await combinePsbts({
        parts: [psbt.base64, cosignerPsbt.trim()],
      });
      setPsbt(combined);
      setCosignerPsbt("");
      await refreshStatus(combined.base64, activePathId);
      setMessage("PSBTs combined.");
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onFinalize() {
    if (!psbt) return;
    setBusy(true);
    setError(null);
    try {
      const tx = await finalizePsbt(psbt.base64);
      setFinalized(tx);
      setMessage(`Finalized ${tx.txid}`);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onBroadcast() {
    if (!broadcastConfirm) {
      setBroadcastConfirm(true);
      setMessage(
        `Confirm broadcast on ${vault ? formatNetwork(vault.policy.network) : "network"} — click Broadcast again.`,
      );
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const txid = await broadcastPsbt({
        vaultId: id,
        psbtBase64: finalized ? null : (psbt?.base64 ?? null),
        txHex: finalized?.hex ?? null,
        esploraUrl: getEsploraUrl() || null,
      });
      setBroadcastConfirm(false);
      setMessage(`Broadcast ok · ${formatNetwork(vault?.policy.network ?? "testnet")} · ${txid}`);
      await runSync({ quiet: true });
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
          <h2>Import PSBT to sign</h2>
          <p>
            {vault?.name ?? "Vault"}
            {vault ? ` · ${formatNetwork(vault.policy.network)}` : ""} · cosigner
            / air-gap (Sparrow usually cannot sign this policy)
          </p>
        </div>
        <Link className="button-link" to="../send" relative="path">
          Send
        </Link>
      </header>

      <nav className="send-steps" aria-label="Import PSBT steps">
        <button
          type="button"
          className={step === "import" ? "send-step active" : "send-step"}
          onClick={resetImport}
        >
          <span className="send-step-num">1</span>
          Import
        </button>
        <span className="send-steps-divider" aria-hidden />
        <button
          type="button"
          className={step === "sign" ? "send-step active" : "send-step"}
          disabled={!psbt}
          onClick={() => psbt && setStep("sign")}
        >
          <span className="send-step-num">2</span>
          Sign & return
        </button>
      </nav>

      <div className="send-wizard">
        <div
          className="send-wizard-track"
          style={{
            transform:
              step === "sign" ? "translateX(-100%)" : "translateX(0)",
          }}
        >
          <div className="send-wizard-pane">
            <form
              className="panel form-grid"
              onSubmit={(e) => void onImport(e)}
            >
              <h3>Paste PSBT</h3>
              <p className="muted">
                1) Import this vault on this machine (backup / descriptor). 2)
                Paste the base64 PSBT from the creator. 3) Sign and{" "}
                <strong>Copy PSBT</strong> back for Combine / Broadcast.
              </p>
              <label>
                PSBT (base64)
                <textarea
                  className="mono"
                  rows={8}
                  value={importBase64}
                  onChange={(e) => setImportBase64(e.target.value)}
                  placeholder="cHNidP8BA… or use Open file"
                  required
                />
              </label>
              {paths.length > 0 ? (
                <label>
                  Active spending path (signature status)
                  <select
                    value={activePathId}
                    onChange={(e) => onSelectPath(e.target.value)}
                  >
                    {paths.map((path) => (
                      <option key={path.id} value={path.id}>
                        {path.label}
                      </option>
                    ))}
                  </select>
                </label>
              ) : null}
              <div className="row-actions">
                <button
                  type="button"
                  className="secondary"
                  disabled={busy}
                  onClick={() => void onImportFromFile()}
                >
                  Open .psbt file…
                </button>
                <button type="submit" disabled={busy || !importBase64.trim()}>
                  {busy ? "Importing…" : "Import →"}
                </button>
              </div>
            </form>
          </div>

          <div className="send-wizard-pane">
            {!psbt ? (
              <div className="panel">
                <p className="muted">Import a PSBT first.</p>
                <button
                  type="button"
                  className="secondary"
                  onClick={resetImport}
                >
                  ← Back to import
                </button>
              </div>
            ) : (
              <div className="panel form-grid">
                <div className="row-actions send-pane-header">
                  <button
                    type="button"
                    className="secondary"
                    onClick={resetImport}
                  >
                    ← Import another
                  </button>
                  <h3>Sign</h3>
                </div>
                {signStatus ? (
                  <div className="send-sign-status">
                    <p>
                      <strong>{signStatus.summary}</strong>
                    </p>
                    <ul className="list compact">
                      {signStatus.keys.map((key) => (
                        <li key={key.id}>
                          <span className="mono">
                            {key.id} · {key.fingerprint}
                          </span>{" "}
                          <span className="muted">({key.role})</span> —{" "}
                          {key.status === "signed"
                            ? "signed"
                            : key.status === "unused"
                              ? "not needed on this path"
                              : "missing"}
                        </li>
                      ))}
                    </ul>
                    <p className="muted">
                      Inputs with sigs: {signStatus.signedInputCount}/
                      {signStatus.totalInputs}
                    </p>
                  </div>
                ) : null}
                <p className="muted">
                  {psbt.inputCount} in / {psbt.outputCount} out
                </p>
                <textarea
                  className="mono"
                  rows={4}
                  readOnly
                  value={psbt.base64}
                />
                <div className="row-actions">
                  <button
                    type="button"
                    onClick={() =>
                      void copyText(psbt.base64).then(() =>
                        setMessage("Copied PSBT — send back to the creator"),
                      )
                    }
                  >
                    Copy PSBT
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    onClick={() => void onExportPsbt()}
                  >
                    Export .psbt file…
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    onClick={() => void refreshStatus(psbt.base64, activePathId)}
                  >
                    Refresh status
                  </button>
                </div>

                <PsbtSignMethodPanel
                  method={signMethod}
                  onMethodChange={setSignMethod}
                  vault={vault}
                  busy={busy}
                  hotWallets={hotWallets}
                  hotWalletId={hotWalletId}
                  onHotWalletIdChange={setHotWalletId}
                  secretKey={secretKey}
                  onSecretKeyChange={setSecretKey}
                  hwFingerprint={hwFingerprint}
                  onHwFingerprintChange={setHwFingerprint}
                  cosignerPsbt={cosignerPsbt}
                  onCosignerPsbtChange={setCosignerPsbt}
                  allowMainnetHotKeys={allowMainnetHotKeys}
                  onAllowMainnetHotKeysChange={setAllowMainnetHotKeys}
                  confirmMainnetHot={confirmMainnetHot}
                  onConfirmMainnetHotChange={setConfirmMainnetHot}
                  onSignHot={() => void onSignHot()}
                  onSignSoftware={() => void onSignSoftware()}
                  onSignHardware={() => void onHwSign()}
                  onCombine={() => void onCombine()}
                />

                <div className="row-actions">
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    onClick={() => void onFinalize()}
                  >
                    Finalize
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy || (!psbt && !finalized)}
                    onClick={() => void onBroadcast()}
                  >
                    {broadcastConfirm ? "Confirm broadcast" : "Broadcast"}
                  </button>
                </div>
                {finalized ? (
                  <p className="mono wrap muted">txid {finalized.txid}</p>
                ) : null}
              </div>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
