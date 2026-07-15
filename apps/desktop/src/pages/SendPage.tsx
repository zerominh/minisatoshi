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
  createPsbt,
  finalizePsbt,
  formatError,
  getVault,
  hotKeystoreStatus,
  hwSignPsbt,
  listHotWallets,
  listSpendingPaths,
  signPsbtHot,
  signPsbtSoftware,
} from "../lib/api";
import { formatTimelockLabel } from "../lib/duration";
import {
  copyText,
  formatNetwork,
  formatSats,
  getEsploraUrl,
  getHwFingerprint,
  getHwiPath,
} from "../lib/settings";
import { savePsbtFileWithDialog, sanitizedFilename } from "../lib/download";
import { useSuccessPulse } from "../lib/useSuccessPulse";
import type {
  FinalizedTxDto,
  HotWalletSummaryDto,
  PsbtDto,
  SigningStatusDto,
  SpendingPathDto,
  UtxoDto,
  VaultDto,
} from "../lib/types";
import { useVault, useVaultIdFromRouteOrContext } from "../vault/VaultContext";

type SendStep = "compose" | "sign" | "broadcast" | "done";

const SEND_STEP_OFFSET: Record<SendStep, string> = {
  compose: "translateX(0)",
  sign: "translateX(-100%)",
  broadcast: "translateX(-200%)",
  done: "translateX(-300%)",
};

export function SendPage() {
  const id = useVaultIdFromRouteOrContext();
  const {
    sync,
    runSync,
    vault: shellVault,
    busy: shellBusy,
    setError,
    setMessage,
  } = useVault();
  const [vault, setVault] = useState<VaultDto | null>(shellVault);
  const [step, setStep] = useState<SendStep>("compose");
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [address, setAddress] = useState("");
  const [amount, setAmount] = useState("");
  const [feeRate, setFeeRate] = useState("2");
  const [paths, setPaths] = useState<SpendingPathDto[]>([]);
  const [activePathId, setActivePathId] = useState("");
  const [inputSequence, setInputSequence] = useState("");
  const [psbt, setPsbt] = useState<PsbtDto | null>(null);
  const [signStatus, setSignStatus] = useState<SigningStatusDto | null>(null);
  const [secretKey, setSecretKey] = useState("");
  const [hotWallets, setHotWallets] = useState<HotWalletSummaryDto[]>([]);
  const [hotWalletId, setHotWalletId] = useState("");
  const [signMethod, setSignMethod] = useState<SignMethod>("hot");
  const [hwFingerprint, setHwFingerprint] = useState(getHwFingerprint());
  const [allowMainnetHotKeys, setAllowMainnetHotKeys] = useState(false);
  const [confirmMainnetHot, setConfirmMainnetHot] = useState(false);
  const [cosignerPsbt, setCosignerPsbt] = useState("");
  const [finalized, setFinalized] = useState<FinalizedTxDto | null>(null);
  const [broadcastConfirm, setBroadcastConfirm] = useState(false);
  const [broadcastTxid, setBroadcastTxid] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const { pulse, flash, is } = useSuccessPulse();
  const successMethod: SignMethod | null =
    pulse === "hot" ||
    pulse === "software" ||
    pulse === "hardware" ||
    pulse === "combine"
      ? pulse
      : null;

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
        const first = list[0]?.id ?? "";
        setActivePathId(first);
        const path = list.find((p) => p.id === first);
        if (path?.suggestedSequence != null) {
          setInputSequence(String(path.suggestedSequence));
        }
      })
      .catch((err) => setError(formatError(err)));
  }, [id]);

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
    const path = paths.find((p) => p.id === pathId);
    if (path?.suggestedSequence != null) {
      setInputSequence(String(path.suggestedSequence));
    } else {
      setInputSequence("");
    }
    if (psbt) {
      void refreshStatus(psbt.base64, pathId);
    }
  }

  useEffect(() => {
    if (!sync) return;
    setSelected((prev) => {
      const keys = Object.keys(prev);
      if (keys.length === 0) {
        const next: Record<string, boolean> = {};
        for (const utxo of sync.utxos) {
          next[`${utxo.txid}:${utxo.vout}`] = true;
        }
        return next;
      }
      return prev;
    });
  }, [sync]);

  async function onSync() {
    setBusy(true);
    setError(null);
    try {
      const result = await runSync();
      if (!result) return;
      const next: Record<string, boolean> = {};
      for (const utxo of result.utxos) {
        next[`${utxo.txid}:${utxo.vout}`] = true;
      }
      setSelected(next);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  function selectedUtxos(): UtxoDto[] {
    if (!sync) return [];
    return sync.utxos.filter((utxo) => selected[`${utxo.txid}:${utxo.vout}`]);
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setMessage(null);
    setPsbt(null);
    setFinalized(null);
    setSignStatus(null);
    setBroadcastConfirm(false);
    setBroadcastTxid(null);
    try {
      const utxos = selectedUtxos();
      if (utxos.length === 0) {
        throw new Error("Select at least one UTXO (sync first)");
      }
      const amountSats = Number(amount);
      if (!Number.isFinite(amountSats) || amountSats <= 0) {
        throw new Error("Enter a valid amount in sats");
      }
      const path = paths.find((p) => p.id === activePathId);
      if (path?.kind === "fallback" && !inputSequence.trim()) {
        throw new Error(
          "Timelock path requires an input sequence (BIP68) — select a path or enter older(N).",
        );
      }
      const seq = inputSequence.trim() ? Number(inputSequence) : undefined;
      if (seq !== undefined && (!Number.isFinite(seq) || seq < 0)) {
        throw new Error("Invalid input sequence");
      }
      const result = await createPsbt({
        vaultId: id,
        recipients: [{ address: address.trim(), amountSats }],
        feeRateSatPerVb: Number(feeRate) || 2,
        utxos,
        inputSequence: seq ?? null,
      });
      setPsbt(result);
      await refreshStatus(result.base64, activePathId);
      setStep("sign");
      setMessage(
        path
          ? `PSBT ready for “${path.label}” — sign required keys, then combine.`
          : "Unsigned PSBT ready.",
      );
      window.scrollTo({ top: 0, behavior: "smooth" });
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  function goBackToCompose() {
    if (step === "done") {
      startNewSend();
      return;
    }
    setStep("compose");
    setBroadcastConfirm(false);
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

  async function onExportPsbt() {
    if (!psbt || !vault) return;
    setError(null);
    try {
      const filename = `${sanitizedFilename(vault.name)}-draft.psbt`;
      const path = await savePsbtFileWithDialog(filename, psbt.base64);
      if (path) setMessage(`PSBT saved to ${path}`);
    } catch (err) {
      setError(formatError(err));
    }
  }

  function startNewSend() {
    setStep("compose");
    setPsbt(null);
    setFinalized(null);
    setSignStatus(null);
    setBroadcastConfirm(false);
    setBroadcastTxid(null);
    setAddress("");
    setAmount("");
    setSecretKey("");
    setCosignerPsbt("");
    setError(null);
    setMessage(null);
    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  async function onSign() {
    if (!psbt || !vault) return;
    if (vault.policy.network === "mainnet") {
      if (!allowMainnetHotKeys || !confirmMainnetHot) {
        setError(
          "Mainnet hot-key signing requires both confirmation checkboxes.",
        );
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
    if (!psbt || !vault || !hotWalletId) return;
    if (vault.policy.network === "mainnet") {
      if (!allowMainnetHotKeys || !confirmMainnetHot) {
        setError(
          "Mainnet hot-key signing requires both confirmation checkboxes.",
        );
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
    setBusy(true);
    setError(null);
    try {
      const tx = await finalizePsbt(psbt.base64);
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
      setMessage(
        `Confirm broadcast on ${vault ? formatNetwork(vault.policy.network) : "selected network"} — click Broadcast again.`,
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

  const activePath = paths.find((p) => p.id === activePathId);

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Send</h2>
          <p>
            {vault?.name ?? "Vault"}
            {vault ? ` · ${formatNetwork(vault.policy.network)}` : ""} · sign
            status · broadcast
          </p>
        </div>
        <Link className="button-link" to="../transactions" relative="path">
          Transactions
        </Link>
      </header>

      <nav className="send-steps" aria-label="Send steps">
        <button
          type="button"
          className={step === "compose" ? "send-step active" : "send-step"}
          onClick={goBackToCompose}
          disabled={step === "done"}
        >
          <span className="send-step-num">1</span>
          Compose
        </button>
        <span className="send-steps-divider" aria-hidden />
        <button
          type="button"
          className={step === "sign" ? "send-step active" : "send-step"}
          disabled={!psbt || step === "done"}
          onClick={goToSign}
        >
          <span className="send-step-num">2</span>
          Sign
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
          {step === "done" ? "Sent" : "Broadcast"}
        </button>
      </nav>

      <div className="send-wizard">
        <div
          className="send-wizard-track"
          data-step={step}
          style={{ transform: SEND_STEP_OFFSET[step] }}
        >
          <div className="send-wizard-pane">
            <div className="panel row-actions">
              <button
                type="button"
                disabled={busy || shellBusy}
                onClick={() => void onSync()}
              >
                {busy || shellBusy
                  ? "Working…"
                  : sync
                    ? "Refresh UTXOs"
                    : "Sync UTXOs"}
              </button>
              {sync ? (
                <span className="muted">
                  Balance {formatSats(sync.balance.confirmedSats)} ·{" "}
                  {sync.utxos.length} UTXO(s)
                </span>
              ) : null}
            </div>

            {sync && sync.utxos.length > 0 ? (
              <div className="panel">
                <h3>Select inputs</h3>
                <ul className="list compact">
                  {sync.utxos.map((utxo) => {
                    const key = `${utxo.txid}:${utxo.vout}`;
                    return (
                      <li key={key} className="list-item">
                        <label className="check-row">
                          <input
                            type="checkbox"
                            checked={!!selected[key]}
                            onChange={(e) =>
                              setSelected((prev) => ({
                                ...prev,
                                [key]: e.target.checked,
                              }))
                            }
                          />
                          <span>
                            {formatSats(utxo.valueSats)} · idx{" "}
                            {utxo.derivationIndex}
                            {utxo.isChange ? " (change)" : ""}
                          </span>
                        </label>
                      </li>
                    );
                  })}
                </ul>
              </div>
            ) : null}

            <form
              className="panel form-grid"
              onSubmit={(e) => void onSubmit(e)}
            >
              <h3>Recipient</h3>
              {paths.length > 0 ? (
                <label>
                  Active spending path
                  <select
                    value={activePathId}
                    onChange={(e) => onSelectPath(e.target.value)}
                  >
                    {paths.map((path) => (
                      <option key={path.id} value={path.id}>
                        {path.label}
                        {path.timelockBlocks != null
                          ? ` · older(${path.timelockBlocks})`
                          : ""}
                      </option>
                    ))}
                  </select>
                </label>
              ) : null}
              {activePath?.kind === "fallback" ? (
                <p className="muted">
                  Timelock path — set BIP68 sequence to match{" "}
                  {activePath.timelockBlocks != null
                    ? `older(${activePath.timelockBlocks})`
                    : formatTimelockLabel("?")}
                  . Spending too early will fail finalize/device checks.
                </p>
              ) : null}
              <label>
                Address
                <input
                  value={address}
                  onChange={(e) => setAddress(e.target.value)}
                  placeholder="tb1…"
                  required
                />
              </label>
              <label>
                Amount (sats)
                <input
                  type="number"
                  min={1}
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  required
                />
              </label>
              <label>
                Fee rate (sat/vB)
                <input
                  type="number"
                  min={1}
                  value={feeRate}
                  onChange={(e) => setFeeRate(e.target.value)}
                  required
                />
              </label>
              <label>
                Input sequence (BIP68 / older)
                <input
                  type="number"
                  min={0}
                  value={inputSequence}
                  onChange={(e) => setInputSequence(e.target.value)}
                  placeholder={
                    activePath?.kind === "fallback"
                      ? "required for timelock path"
                      : "empty = RBF default"
                  }
                />
              </label>
              <button type="submit" disabled={busy}>
                {busy ? "Creating…" : "Create PSBT →"}
              </button>
            </form>
            <p className="muted">
              Cosigner on another machine? Use{" "}
              <Link to="../sign-psbt" relative="path">
                Import PSBT
              </Link>
              .
            </p>
          </div>

          <div className="send-wizard-pane">
            {!psbt ? (
              <div className="panel">
                <p className="muted">
                  Create a PSBT first, then this step opens for signing.
                </p>
                <button
                  type="button"
                  className="secondary"
                  onClick={goBackToCompose}
                >
                  ← Back to compose
                </button>
              </div>
            ) : (
              <div className="panel form-grid">
                <div className="row-actions send-pane-header">
                  <button
                    type="button"
                    className="secondary"
                    onClick={goBackToCompose}
                  >
                    ← Edit draft
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
                  {psbt.inputCount} in / {psbt.outputCount} out · multi-device:
                  sign each required key, then Combine.
                </p>
                <textarea
                  className="mono"
                  rows={3}
                  readOnly
                  value={psbt.base64}
                />
                <div className="row-actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={() =>
                      void copyText(psbt.base64).then(() =>
                        setMessage("Copied PSBT base64"),
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
                  onSignSoftware={() => void onSign()}
                  onSignHardware={() => void onHwSign()}
                  onCombine={() => void onCombine()}
                />
                <div className="row-actions">
                  <button
                    type="button"
                    className={is("finalize") ? "btn-ok" : undefined}
                    disabled={busy}
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
                  onClick={goBackToCompose}
                >
                  ← Back to compose
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
                    {vault ? formatNetwork(vault.policy.network) : "unknown"}
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
                      Not finalized yet — broadcast will try to finalize from
                      the PSBT, or go back and Finalize first.
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
                  <p className="muted">
                    Confirm sending to{" "}
                    <strong>
                      {vault
                        ? formatNetwork(vault.policy.network)
                        : "unknown"}
                    </strong>
                    . Click Broadcast again to publish.
                  </p>
                ) : null}
                <button
                  type="button"
                  className={is("broadcast") ? "btn-ok" : undefined}
                  disabled={busy || (!psbt && !finalized)}
                  onClick={() => void onBroadcast()}
                >
                  {broadcastConfirm
                    ? `Confirm broadcast (${vault ? formatNetwork(vault.policy.network) : "network"})`
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
                  {vault ? formatNetwork(vault.policy.network) : "network"}
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
                  onClick={startNewSend}
                >
                  Send another
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
