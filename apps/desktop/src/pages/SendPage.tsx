import { FormEvent, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { createPsbt, formatError, getVault, syncVault } from "../lib/api";
import { copyText, formatSats, getEsploraUrl } from "../lib/settings";
import type { PsbtDto, SyncResultDto, UtxoDto, VaultDto } from "../lib/types";

export function SendPage() {
  const { id = "" } = useParams();
  const [vault, setVault] = useState<VaultDto | null>(null);
  const [sync, setSync] = useState<SyncResultDto | null>(null);
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [address, setAddress] = useState("");
  const [amount, setAmount] = useState("");
  const [feeRate, setFeeRate] = useState("2");
  const [psbt, setPsbt] = useState<PsbtDto | null>(null);
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
    try {
      const utxos = selectedUtxos();
      if (utxos.length === 0) {
        throw new Error("Select at least one UTXO (sync first)");
      }
      const amountSats = Number(amount);
      if (!Number.isFinite(amountSats) || amountSats <= 0) {
        throw new Error("Enter a valid amount in sats");
      }
      const result = await createPsbt({
        vaultId: id,
        recipients: [{ address: address.trim(), amountSats }],
        feeRateSatPerVb: Number(feeRate) || 2,
        utxos,
      });
      setPsbt(result);
      setMessage("Unsigned PSBT ready — paste into Sparrow to sign.");
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
            {vault?.name ?? "Vault"} · create unsigned PSBT · export to Sparrow
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
        <button type="submit" disabled={busy}>
          3. Create PSBT
        </button>
      </form>

      {error ? <pre className="error">{error}</pre> : null}
      {message ? <p className="status">{message}</p> : null}

      {psbt ? (
        <div className="panel">
          <h3>PSBT export</h3>
          <p className="muted">
            {psbt.inputCount} in / {psbt.outputCount} out · BIP174 base64
          </p>
          <textarea className="mono" rows={6} readOnly value={psbt.base64} />
          <button
            type="button"
            onClick={() =>
              void copyText(psbt.base64).then(() =>
                setMessage("Copied PSBT base64"),
              )
            }
          >
            Copy base64 for Sparrow
          </button>
        </div>
      ) : null}
    </section>
  );
}
