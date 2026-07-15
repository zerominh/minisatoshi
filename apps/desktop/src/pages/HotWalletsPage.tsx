import { FormEvent, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  MnemonicGrid,
  mnemonicIsComplete,
  wordsToMnemonic,
  type WordCount,
} from "../components/MnemonicGrid";
import { useFlash } from "../flash/FlashContext";
import { useT } from "../i18n/LocaleContext";
import {
  createHotKeystore,
  deleteHotWallet,
  formatError,
  hotKeystoreStatus,
  importHotWallet,
  listHotWallets,
  lockHotKeystore,
  renameHotWallet,
  unlockHotKeystore,
} from "../lib/api";
import { formatNetwork, getPreferredNetwork } from "../lib/settings";
import type {
  HotKeystoreStatusDto,
  HotWalletSummaryDto,
  NetworkName,
} from "../lib/types";

const NETWORKS: NetworkName[] = [
  "testnet4",
  "testnet",
  "signet",
  "regtest",
  "mainnet",
];

export function HotWalletsPage() {
  const t = useT();
  const navigate = useNavigate();
  const { setError, setMessage } = useFlash();
  const [status, setStatus] = useState<HotKeystoreStatusDto | null>(null);
  const [hotWallets, setHotWallets] = useState<HotWalletSummaryDto[]>([]);
  const [password, setPassword] = useState("");
  const [name, setName] = useState("My hot wallet");
  const [wordCount, setWordCount] = useState<WordCount>(24);
  const [words, setWords] = useState<string[]>(() => Array(24).fill(""));
  const [advancedJson, setAdvancedJson] = useState(false);
  const [jsonPayload, setJsonPayload] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [network, setNetwork] = useState<NetworkName>(getPreferredNetwork());
  const [busy, setBusy] = useState(false);
  /** BIP-39 import form — collapsed until user clicks Add. */
  const [importOpen, setImportOpen] = useState(false);

  async function refresh() {
    const st = await hotKeystoreStatus();
    setStatus(st);
    if (st.unlocked) {
      setHotWallets(await listHotWallets());
    } else {
      setHotWallets([]);
    }
  }

  useEffect(() => {
    void refresh().catch((err) => setError(formatError(err)));
  }, []);

  async function onCreateOrUnlock(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      if (status?.exists) {
        await unlockHotKeystore({ masterPassword: password });
        setMessage("Hot keystore unlocked");
      } else {
        await createHotKeystore({ masterPassword: password });
        setMessage("Hot keystore created — keep this master password");
      }
      setPassword("");
      await refresh();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onLock() {
    setBusy(true);
    try {
      await lockHotKeystore();
      closeImportForm();
      setMessage("Locked");
      await refresh();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onImport(event: FormEvent) {
    event.preventDefault();
    const mnemonicOrJson = advancedJson
      ? jsonPayload.trim()
      : wordsToMnemonic(words);
    if (!advancedJson && !mnemonicIsComplete(words, wordCount)) {
      setError(
        `Enter all ${wordCount} valid BIP-39 words (or paste / scan SeedQR)`,
      );
      return;
    }
    if (!mnemonicOrJson) {
      setError("Mnemonic required");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const result = await importHotWallet({
        name,
        mnemonicOrJson,
        bip39Passphrase: passphrase || undefined,
        network,
        workspaceId: "",
        createNestedWallet: true,
      });
      resetImportForm();
      setImportOpen(false);
      setMessage(`Imported “${result.hotWallet.name}”`);
      await refresh();
      navigate(`/hot-wallets/${result.hotWallet.id}`);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  function resetImportForm() {
    setWords(Array(wordCount).fill(""));
    setJsonPayload("");
    setPassphrase("");
    setAdvancedJson(false);
    setName("My hot wallet");
    setNetwork(getPreferredNetwork());
  }

  function closeImportForm() {
    resetImportForm();
    setImportOpen(false);
  }

  async function onRename(hw: HotWalletSummaryDto) {
    const next = window.prompt("Rename hot wallet", hw.name)?.trim();
    if (!next || next === hw.name) return;
    setBusy(true);
    setError(null);
    try {
      await renameHotWallet(hw.id, next);
      setMessage(`Renamed to “${next}”`);
      await refresh();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onDelete(id: string) {
    setBusy(true);
    try {
      await deleteHotWallet(id);
      setMessage("Hot wallet removed from keystore");
      await refresh();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  const canImport = advancedJson
    ? jsonPayload.trim().length > 0
    : mnemonicIsComplete(words, wordCount);

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>{t("hotWallets.title")}</h2>
          <p>{t("hotWallets.subtitle")}</p>
        </div>
      </header>

      <div className="panel form-grid">
        <h3>
          Keystore{" "}
          {status?.unlocked ? (
            <span className="badge">Unlocked</span>
          ) : (
            <span className="badge watch-only">Locked</span>
          )}
        </h3>
        <p className="muted mono wrap">{status?.path ?? "…"}</p>
        {!status?.unlocked ? (
          <form
            className="form-grid"
            onSubmit={(e) => void onCreateOrUnlock(e)}
          >
            <label>
              Master password
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
                autoComplete="new-password"
              />
            </label>
            <button type="submit" disabled={busy || !password}>
              {busy
                ? "…"
                : status?.exists
                  ? "Unlock"
                  : "Create encrypted keystore"}
            </button>
          </form>
        ) : (
          <button
            type="button"
            className="secondary"
            onClick={() => void onLock()}
          >
            Lock keystore
          </button>
        )}
      </div>

      {status?.unlocked ? (
        <>
          <div className="panel">
            <header className="page-header">
              <div>
                <h3>Your hot wallets</h3>
                <p className="muted">
                  Tap a wallet to open Transactions / Send / Receive.
                </p>
              </div>
              {!importOpen ? (
                <button
                  type="button"
                  className="primary"
                  disabled={busy}
                  onClick={() => setImportOpen(true)}
                >
                  Add
                </button>
              ) : null}
            </header>
            {hotWallets.length === 0 ? (
              <p className="muted">None yet.</p>
            ) : (
              <ul className="list">
                {hotWallets.map((hw) => (
                  <li key={hw.id} className="list-item">
                    <button
                      type="button"
                      className="list-item-main"
                      disabled={busy}
                      onClick={() => navigate(`/hot-wallets/${hw.id}`)}
                    >
                      <strong>{hw.name}</strong>
                      <div className="muted">
                        {formatNetwork(hw.network)} · fp{" "}
                        <span className="mono">{hw.fingerprint}</span>
                      </div>
                      <div className="mono wrap muted">
                        {hw.xpub.slice(0, 24)}…
                      </div>
                    </button>
                    <div className="row-actions">
                      <button
                        type="button"
                        className="secondary"
                        disabled={busy}
                        onClick={() => void onRename(hw)}
                      >
                        Rename
                      </button>
                      <button
                        type="button"
                        className="secondary"
                        disabled={busy}
                        onClick={() => void onDelete(hw.id)}
                      >
                        Delete
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {importOpen ? (
            <form
              className="panel form-grid"
              onSubmit={(e) => void onImport(e)}
            >
              <header className="page-header">
                <div>
                  <h3>Import BIP-39 seed</h3>
                  <p className="muted">
                    BIP-86 Taproot on {formatNetwork(network)}. SeedQR from
                    Sparrow / SeedSigner supported.
                  </p>
                </div>
                <button
                  type="button"
                  className="secondary"
                  disabled={busy}
                  onClick={closeImportForm}
                >
                  Cancel
                </button>
              </header>
              <label>
                Network
                <select
                  value={network}
                  onChange={(e) => setNetwork(e.target.value as NetworkName)}
                >
                  {NETWORKS.map((n) => (
                    <option key={n} value={n}>
                      {formatNetwork(n)}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Display name
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  required
                />
              </label>

              <label className="check-row">
                <input
                  type="checkbox"
                  checked={advancedJson}
                  onChange={(e) => setAdvancedJson(e.target.checked)}
                />
                <span>Paste JSON / raw text instead of word grid</span>
              </label>

              {advancedJson ? (
                <label>
                  Mnemonic JSON {'{ "mnemonic": "…" }'} or raw words
                  <textarea
                    className="mono"
                    rows={4}
                    value={jsonPayload}
                    onChange={(e) => setJsonPayload(e.target.value)}
                    required
                    autoComplete="off"
                  />
                </label>
              ) : (
                <MnemonicGrid
                  wordCount={wordCount}
                  onWordCountChange={setWordCount}
                  words={words}
                  onWordsChange={setWords}
                  disabled={busy}
                />
              )}

              <label>
                BIP-39 passphrase (optional)
                <input
                  type="password"
                  value={passphrase}
                  onChange={(e) => setPassphrase(e.target.value)}
                  autoComplete="off"
                />
              </label>
              <div className="row-actions">
                <button
                  type="button"
                  className="secondary"
                  disabled={busy}
                  onClick={closeImportForm}
                >
                  Cancel
                </button>
                <button type="submit" disabled={busy || !canImport}>
                  {busy ? "Importing…" : "Import hot wallet"}
                </button>
              </div>
            </form>
          ) : null}
        </>
      ) : null}
    </section>
  );
}
