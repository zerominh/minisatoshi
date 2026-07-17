import { FormEvent, useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  PsbtSignMethodPanel,
  type SignMethod,
} from "../components/PsbtSignMethodPanel";
import { BroadcastConfirmSummary } from "../components/BroadcastConfirmSummary";
import { useT } from "../i18n/LocaleContext";
import {
  analyzePsbtStatus,
  broadcastPsbt,
  combinePsbts,
  finalizePsbt,
  formatError,
  getWallet,
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
import { useSuccessPulse } from "../lib/useSuccessPulse";
import { pathSatisfied } from "../lib/signingStatus";
import type {
  FinalizedTxDto,
  HotWalletSummaryDto,
  PsbtDto,
  SigningStatusDto,
  SpendingPathDto,
  WalletDto,
} from "../lib/types";
import { useWallet, useWalletIdFromRouteOrContext } from "../wallet/WalletContext";

type Step = "import" | "sign" | "broadcast" | "done";

const STEP_OFFSET: Record<Step, string> = {
  import: "translateX(0)",
  sign: "translateX(-100%)",
  broadcast: "translateX(-200%)",
  done: "translateX(-300%)",
};

/**
 * Cosigner / air-gap: import a PSBT created elsewhere, sign locally, copy back.
 * Sparrow often cannot sign Miniscript Taproot wallets — use this instead.
 * When enough signatures are present, Finalize → Broadcast → Sent (same as Send).
 */
export function SignPsbtPage() {
  const t = useT();
  const id = useWalletIdFromRouteOrContext();
  const {
    wallet: shellWallet,
    setError,
    setMessage,
    runSync,
  } = useWallet();
  const [wallet, setWallet] = useState<WalletDto | null>(shellWallet);
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
  const [broadcastTxid, setBroadcastTxid] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const readyToFinalize = pathSatisfied(signStatus, activePathId);
  const { pulse, flash, is } = useSuccessPulse();
  const successMethod: SignMethod | null =
    pulse === "hot" ||
    pulse === "software" ||
    pulse === "hardware" ||
    pulse === "combine"
      ? pulse
      : null;

  useEffect(() => {
    if (shellWallet) setWallet(shellWallet);
  }, [shellWallet]);

  useEffect(() => {
    void getWallet(id)
      .then((w) => {
        setWallet(w);
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
        const linked = list.find((h) => h.linkedWalletId === id);
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
          walletId: id,
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
    setBroadcastTxid(null);
    setSecretKey("");
    setCosignerPsbt("");
    setError(null);
    setMessage(null);
    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  function goToSign() {
    if (!psbt || step === "done") return;
    setStep("sign");
    setBroadcastConfirm(false);
    setError(null);
    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  function goToBroadcast() {
    if (!psbt || step === "done") return;
    setStep("broadcast");
    setBroadcastConfirm(false);
    setError(null);
    window.scrollTo({ top: 0, behavior: "smooth" });
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
      await loadImportedPsbt(file.contents, file.path);
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function loadImportedPsbt(rawInput: string, fromPath?: string) {
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
      setFinalized(null);
      setBroadcastTxid(null);
      const status = await analyzePsbtStatus({
        walletId: id,
        psbtBase64: result.base64,
        activePathId: activePathId || null,
      }).catch(() => null);
      setSignStatus(status);
      setStep("sign");
      const enough = pathSatisfied(status, activePathId);
      const prefix = fromPath ? `Loaded ${fromPath} · ` : "";
      setMessage(
        enough
          ? `${prefix}Imported (${result.inputCount} in / ${result.outputCount} out) — enough signatures; Finalize or Broadcast.`
          : `${prefix}Imported (${result.inputCount} in / ${result.outputCount} out) — sign, then Copy/Export or Finalize when ready.`,
      );
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onExportPsbt() {
    if (!psbt || !wallet) return;
    setError(null);
    try {
      const filename = `${sanitizedFilename(wallet.name)}-partial.psbt`;
      const path = await savePsbtFileWithDialog(filename, psbt.base64);
      if (path) setMessage(`PSBT saved to ${path}`);
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function onSignSoftware() {
    if (!psbt || !wallet) return;
    if (wallet.policy.network === "mainnet") {
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
        network: wallet.policy.network,
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
      flash("software");
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
    if (!psbt || !wallet || !hotWalletId) return;
    if (wallet.policy.network === "mainnet") {
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
        network: wallet.policy.network,
        allowMainnetHotKeys,
      });
      const next = {
        base64: signed.base64,
        inputCount: signed.inputCount,
        outputCount: signed.outputCount,
      };
      setPsbt(next);
      await refreshStatus(next.base64, activePathId);
      flash("hot");
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
    if (!psbt || !wallet) return;
    if (!hwFingerprint.trim()) {
      setError(t("send.hwNoFingerprint"));
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const signed = await hwSignPsbt({
        fingerprint: hwFingerprint.trim(),
        psbtBase64: psbt.base64,
        hwiPath: getHwiPath() || null,
        network: wallet.policy.network,
        walletId: id,
      });
      const next = {
        base64: signed.base64,
        inputCount: signed.inputCount,
        outputCount: signed.outputCount,
      };
      setPsbt(next);
      await refreshStatus(next.base64, activePathId);
      flash("hardware");
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
      flash("combine");
      setMessage("PSBTs combined.");
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onFinalize() {
    if (!psbt) return;
    if (!readyToFinalize) {
      setError(t("send.finalizeBlocked"));
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const tx = await finalizePsbt(psbt.base64, id);
      setFinalized(tx);
      flash("finalize");
      setMessage(`Finalized ${tx.txid} — review and broadcast`);
      window.setTimeout(() => {
        setStep("broadcast");
        window.scrollTo({ top: 0, behavior: "smooth" });
      }, 480);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onBroadcast() {
    if (!broadcastConfirm) {
      setBroadcastConfirm(true);
      setMessage(null);
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const txid = await broadcastPsbt({
        walletId: id,
        psbtBase64: finalized ? null : (psbt?.base64 ?? null),
        txHex: finalized?.hex ?? null,
        esploraUrl: getEsploraUrl() || null,
      });
      setBroadcastConfirm(false);
      setBroadcastTxid(txid);
      flash("broadcast");
      setMessage(null);
      setStep("done");
      window.scrollTo({ top: 0, behavior: "smooth" });
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
          <h2>{t("signPsbt.title")}</h2>
          <p>
            {wallet?.name ?? t("shell.wallet")}
            {wallet ? ` · ${formatNetwork(wallet.policy.network)}` : ""} ·{" "}
            {t("signPsbt.subtitle")}
          </p>
        </div>
        <Link className="button-link" to="../send" relative="path">
          {t("signPsbt.sendLink")}
        </Link>
      </header>

      <nav className="send-steps" aria-label="Import PSBT steps">
        <button
          type="button"
          className={step === "import" ? "send-step active" : "send-step"}
          onClick={resetImport}
          disabled={step === "done"}
        >
          <span className="send-step-num">1</span>
          {t("signPsbt.import")}
        </button>
        <span className="send-steps-divider" aria-hidden />
        <button
          type="button"
          className={step === "sign" ? "send-step active" : "send-step"}
          disabled={!psbt || step === "done"}
          onClick={goToSign}
        >
          <span className="send-step-num">2</span>
          {t("signPsbt.sign")}
        </button>
        <span className="send-steps-divider" aria-hidden />
        <button
          type="button"
          className={
            step === "broadcast" || step === "done"
              ? "send-step active"
              : "send-step"
          }
          disabled={!psbt || step === "done"}
          onClick={goToBroadcast}
        >
          <span className="send-step-num">{step === "done" ? "✓" : "3"}</span>
          {step === "done" ? t("send.sent") : t("send.broadcast")}
        </button>
      </nav>

      <div className="send-wizard">
        <div
          className="send-wizard-track"
          data-step={step}
          style={{ transform: STEP_OFFSET[step] }}
        >
          <div className="send-wizard-pane">
            <form
              className="panel form-grid"
              onSubmit={(e) => void onImport(e)}
            >
              <h3>Paste PSBT</h3>
              <p className="muted">
                1) Import this wallet on this machine (backup / descriptor). 2)
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
                  <h3>{readyToFinalize ? "Sign · ready" : "Sign"}</h3>
                </div>
                {readyToFinalize ? (
                  <p className="muted">
                    Enough signatures for this path — Finalize and Broadcast
                    here, or Copy/Export PSBT back to the creator.
                  </p>
                ) : null}
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
                  wallet={wallet}
                  busy={busy}
                  successMethod={successMethod}
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
                  onHwError={setError}
                />

                <div className="row-actions">
                  <button
                    type="button"
                    className={is("finalize") ? "btn-ok" : undefined}
                    disabled={busy || (!readyToFinalize && !is("finalize"))}
                    onClick={() => void onFinalize()}
                  >
                    {busy
                      ? "Finalizing…"
                      : is("finalize")
                        ? "Finalized ✓"
                        : "Finalize →"}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    disabled={!psbt}
                    onClick={goToBroadcast}
                  >
                    Broadcast →
                  </button>
                </div>
              </div>
            )}
          </div>

          <div className="send-wizard-pane">
            {!psbt ? (
              <div className="panel">
                <p className="muted">Sign a PSBT first, then broadcast.</p>
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
                    onClick={goToSign}
                  >
                    ← Back to sign
                  </button>
                  <h3>Broadcast</h3>
                </div>
                <p className="muted">
                  Network:{" "}
                  <strong>
                    {wallet ? formatNetwork(wallet.policy.network) : "unknown"}
                  </strong>
                  {getEsploraUrl()
                    ? ` · ${getEsploraUrl()}`
                    : " · default Esplora"}
                </p>
                {finalized ? (
                  <>
                    <p className="mono wrap">txid {finalized.txid}</p>
                    <textarea
                      className="mono"
                      rows={4}
                      readOnly
                      value={finalized.hex}
                    />
                    <button
                      type="button"
                      className="secondary"
                      onClick={() =>
                        void copyText(finalized.hex).then(() =>
                          setMessage("Copied tx hex"),
                        )
                      }
                    >
                      Copy hex
                    </button>
                  </>
                ) : (
                  <>
                    <p className="muted">
                      {readyToFinalize
                        ? "Signatures look complete — Finalize first, or broadcast will try to finalize from the PSBT."
                        : "Not finalized yet — broadcast will try to finalize from the PSBT, or go back and Finalize when signatures are enough."}
                    </p>
                    <textarea
                      className="mono"
                      rows={3}
                      readOnly
                      value={psbt.base64}
                    />
                    <button
                      type="button"
                      className={is("finalize") ? "btn-ok secondary" : "secondary"}
                      disabled={busy}
                      onClick={() => void onFinalize()}
                    >
                      {is("finalize") ? "Finalized ✓" : "Finalize first"}
                    </button>
                  </>
                )}
                {broadcastConfirm ? (
                  <BroadcastConfirmSummary
                    fromWallet={wallet?.name ?? t("shell.wallet")}
                    network={wallet?.policy.network}
                    outputs={finalized?.outputs}
                  />
                ) : null}
                <button
                  type="button"
                  className={is("broadcast") ? "btn-ok" : undefined}
                  disabled={busy || (!psbt && !finalized)}
                  onClick={() => void onBroadcast()}
                >
                  {busy && broadcastConfirm
                    ? "Broadcasting…"
                    : broadcastConfirm
                      ? `Confirm broadcast (${wallet ? formatNetwork(wallet.policy.network) : "network"})`
                      : is("broadcast")
                        ? "Broadcast ✓"
                        : "Broadcast"}
                </button>
              </div>
            )}
          </div>

          <div className="send-wizard-pane">
            <div className="panel send-success">
              <p className="send-success-eyebrow">Sent</p>
              <h3>Transaction broadcast</h3>
              <p className="muted">
                Published on{" "}
                <strong>
                  {wallet ? formatNetwork(wallet.policy.network) : "network"}
                </strong>
                . It may take a moment to appear in history after sync.
              </p>
              {broadcastTxid ? (
                <div className="send-success-txid">
                  <span className="muted">txid</span>
                  <p className="mono wrap">{broadcastTxid}</p>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() =>
                      void copyText(broadcastTxid).then(() =>
                        setMessage("Copied txid"),
                      )
                    }
                  >
                    Copy txid
                  </button>
                </div>
              ) : null}
              <div className="row-actions send-success-actions">
                <Link
                  className="button-link primary"
                  to="../transactions"
                  relative="path"
                >
                  View transactions
                </Link>
                <button
                  type="button"
                  className="secondary"
                  onClick={resetImport}
                >
                  Import another
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
