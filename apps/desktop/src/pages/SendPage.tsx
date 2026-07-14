import { FormEvent, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  broadcastPsbt,
  combinePsbts,
  createPsbt,
  finalizePsbt,
  formatError,
  getVault,
  signPsbtSoftware,
  syncVault,
} from "../lib/api";
import { copyText, formatSats, getEsploraUrl } from "../lib/settings";
import type {
  FinalizedTxDto,
  PsbtDto,
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
  const [inputSequence, setInputSequence] = useState("");
  const [psbt, setPsbt] = useState<PsbtDto | null>(null);
  const [secretKey, setSecretKey] = useState("");
  const [allowMainnetHotKeys, setAllowMainnetHotKeys] = useState(false);
  const [cosignerPsbt, setCosignerPsbt] = useState("");
  const [finalized, setFinalized] = useState<FinalizedTxDto | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

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
    try {
      const utxos = selectedUtxos();
      if (utxos.length === 0) {
        throw new Error("Select at least one UTXO (sync first)");
      }
      const amountSats = Number(amount);
      if (!Number.isFinite(amountSats) || amountSats <= 0) {
        throw new Error("Enter a valid amount in sats");
      }
      const seq = inputSequence.trim()
        ? Number(inputSequence)
        : undefined;
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
      setMessage("Unsigned PSBT ready — sign below (software / cosigner).");
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onSign() {
    if (!psbt || !vault) return;
    setBusy(true);
    setError(null);
    try {
      const signed = await signPsbtSoftware({
        psbtBase64: psbt.base64,
        secretKey: secretKey.trim(),
        network: vault.policy.network,
        allowMainnetHotKeys,
      });
      setPsbt({
        base64: signed.base64,
        inputCount: signed.inputCount,
        outputCount: signed.outputCount,
      });
      setSecretKey("");
      setMessage(
        `Signed ${signed.signedInputs}/${signed.totalInputs} input(s) — combine or finalize when ready.`,
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
    setBusy(true);
    setError(null);
    try {
      const txid = await broadcastPsbt({
        vaultId: id,
        psbtBase64: finalized ? null : psbt?.base64 ?? null,
        txHex: finalized?.hex ?? null,
        esploraUrl: getEsploraUrl() || null,
      });
      setMessage(`Broadcast ok · txid ${txid}`);
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
          <h2>Send</h2>
          <p>
            {vault?.name ?? "Vault"} · PSBT · software sign (Sprint 9) ·
            broadcast
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
        <h3>2. Recipient</h3>
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
          Input sequence (optional, BIP68 for older/timelock path)
          <input
            type="number"
            min={0}
            value={inputSequence}
            onChange={(e) => setInputSequence(e.target.value)}
            placeholder="leave empty for RBF default"
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
          <p className="muted">
            {psbt.inputCount} in / {psbt.outputCount} out · BIP174 base64.
            Software keys are for testnet/dev — mainnet requires explicit
            allow.
          </p>
          <textarea className="mono" rows={5} readOnly value={psbt.base64} />
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
            <label className="check-row">
              <input
                type="checkbox"
                checked={allowMainnetHotKeys}
                onChange={(e) => setAllowMainnetHotKeys(e.target.checked)}
              />
              <span>Allow mainnet hot-key signing (dangerous)</span>
            </label>
          ) : null}
          <button
            type="button"
            disabled={busy || !secretKey.trim()}
            onClick={() => void onSign()}
          >
            Sign with software key
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
              6. Broadcast
            </button>
          </div>
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
