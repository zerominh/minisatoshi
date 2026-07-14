import { FormEvent, useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  analyzePsbtStatus,
  broadcastPsbt,
  combinePsbts,
  createPsbt,
  finalizePsbt,
  formatError,
  getVault,
  hwSignPsbt,
  listSpendingPaths,
  signPsbtSoftware,
  syncVault,
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
import type {
  FinalizedTxDto,
  PsbtDto,
  SigningStatusDto,
  SpendingPathDto,
  SyncResultDto,
  UtxoDto,
  VaultDto,
} from "../lib/types";

export function SendPage() {
  const { id = "" } = useParams();
  const [vault, setVault] = useState<VaultDto | null>(null);
  const [sync, setSync] = useState<SyncResultDto | null>(null);
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
  const [hwFingerprint, setHwFingerprint] = useState(getHwFingerprint());
  const [allowMainnetHotKeys, setAllowMainnetHotKeys] = useState(false);
  const [confirmMainnetHot, setConfirmMainnetHot] = useState(false);
  const [cosignerPsbt, setCosignerPsbt] = useState("");
  const [finalized, setFinalized] = useState<FinalizedTxDto | null>(null);
  const [broadcastConfirm, setBroadcastConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

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

  async function onSync() {
    setBusy(true);
    setError(null);
    try {
      const result = await syncVault(id, getEsploraUrl() || undefined);
      setSync(result);
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
      setMessage(
        path
          ? `PSBT ready for “${path.label}” — sign required keys, then combine.`
          : "Unsigned PSBT ready.",
      );
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
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
      setMessage(
        `Software signed ${signed.signedInputs}/${signed.totalInputs} input(s).`,
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
      setMessage(`Broadcast ok · ${formatNetwork(vault?.policy.network ?? "testnet")} · txid ${txid}`);
      if (sync) {
        const result = await syncVault(id, getEsploraUrl() || undefined);
        setSync(result);
      }
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
        <Link className="button-link" to={`/vaults/${id}`}>
          Back to vault
        </Link>
      </header>

      <div className="panel row-actions">
        <button type="button" disabled={busy} onClick={() => void onSync()}>
          {busy ? "Working…" : "1. Sync UTXOs"}
        </button>
        {sync ? (
          <span className="muted">
            Balance {formatSats(sync.balance.confirmedSats)} · {sync.utxos.length}{" "}
            UTXO(s)
          </span>
        ) : null}
      </div>

      {sync && sync.utxos.length > 0 ? (
        <div className="panel">
          <h3>Select inputs</h3>
          <ul className="list">
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
                      {formatSats(utxo.valueSats)} · idx {utxo.derivationIndex}
                      {utxo.isChange ? " (change)" : ""}
                    </span>
                  </label>
                </li>
              );
            })}
          </ul>
        </div>
      ) : null}

      <form className="panel form-grid" onSubmit={(e) => void onSubmit(e)}>
        <h3>2. Spending path & recipient</h3>
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
          3. Create PSBT
        </button>
      </form>

      {error ? <pre className="error">{error}</pre> : null}
      {message ? <p className="status">{message}</p> : null}

      {psbt ? (
        <div className="panel form-grid">
          <h3>4. Sign / combine</h3>
          {signStatus ? (
            <div className="panel" style={{ margin: 0 }}>
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
            {psbt.inputCount} in / {psbt.outputCount} out · multi-device: sign each
            required key, then Combine.
          </p>
          <textarea className="mono" rows={4} readOnly value={psbt.base64} />
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
              onClick={() => void refreshStatus(psbt.base64, activePathId)}
            >
              Refresh signature status
            </button>
          </div>
          <label>
            Descriptor secret (tprv/xprv… with path)
            <textarea
              rows={2}
              className="mono"
              value={secretKey}
              onChange={(e) => setSecretKey(e.target.value)}
              placeholder="tprv…/86'/1'/0'/0/*"
              autoComplete="off"
            />
          </label>
          {vault?.policy.network === "mainnet" ? (
            <>
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={allowMainnetHotKeys}
                  onChange={(e) => setAllowMainnetHotKeys(e.target.checked)}
                />
                <span>Allow mainnet hot-key signing (dangerous)</span>
              </label>
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={confirmMainnetHot}
                  onChange={(e) => setConfirmMainnetHot(e.target.checked)}
                />
                <span>I understand this exposes private key material on this machine</span>
              </label>
            </>
          ) : null}
          <button
            type="button"
            disabled={busy || !secretKey.trim()}
            onClick={() => void onSign()}
          >
            Sign with software key
          </button>
          <label>
            Hardware fingerprint (HWI)
            <input
              className="mono"
              value={hwFingerprint}
              onChange={(e) => setHwFingerprint(e.target.value)}
              placeholder="Set in Settings or paste here"
            />
          </label>
          <button
            type="button"
            className="secondary"
            disabled={busy || !hwFingerprint.trim()}
            onClick={() => void onHwSign()}
          >
            Sign with hardware
          </button>
          <label>
            Cosigner PSBT (base64)
            <textarea
              rows={3}
              className="mono"
              value={cosignerPsbt}
              onChange={(e) => setCosignerPsbt(e.target.value)}
              placeholder="Paste partially signed PSBT from another signer"
            />
          </label>
          <button
            type="button"
            className="secondary"
            disabled={busy || !cosignerPsbt.trim()}
            onClick={() => void onCombine()}
          >
            Combine with cosigner PSBT
          </button>
          <div className="row-actions">
            <button
              type="button"
              disabled={busy}
              onClick={() => void onFinalize()}
            >
              5. Finalize
            </button>
            <button
              type="button"
              disabled={busy || (!psbt && !finalized)}
              onClick={() => void onBroadcast()}
            >
              {broadcastConfirm
                ? `Confirm broadcast (${vault ? formatNetwork(vault.policy.network) : "network"})`
                : "6. Broadcast"}
            </button>
          </div>
          {broadcastConfirm ? (
            <p className="muted">
              Broadcasting to{" "}
              <strong>
                {vault ? formatNetwork(vault.policy.network) : "unknown"}
              </strong>
              {getEsploraUrl() ? ` via ${getEsploraUrl()}` : " via default Esplora"}
              . Click the button again to send.
            </p>
          ) : null}
        </div>
      ) : null}

      {finalized ? (
        <div className="panel form-grid">
          <h3>Finalized transaction</h3>
          <p className="mono wrap">txid {finalized.txid}</p>
          <textarea className="mono" rows={4} readOnly value={finalized.hex} />
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
        </div>
      ) : null}
    </section>
  );
}
