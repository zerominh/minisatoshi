import { FormEvent, useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import {
  createHotKeystore,
  deleteHotWallet,
  formatError,
  hotKeystoreStatus,
  importHotWallet,
  listHotWallets,
  listWallets,
  lockHotKeystore,
  unlockHotKeystore,
} from "../lib/api";
import {
  formatNetwork,
  getActiveWalletId,
  getPreferredNetwork,
  setActiveWalletId,
} from "../lib/settings";
import type {
  HotKeystoreStatusDto,
  HotWalletSummaryDto,
  NetworkName,
  WalletSummaryDto,
} from "../lib/types";

export function HotWalletsPage() {
  const navigate = useNavigate();
  const [status, setStatus] = useState<HotKeystoreStatusDto | null>(null);
  const [wallets, setWallets] = useState<WalletSummaryDto[]>([]);
  const [hotWallets, setHotWallets] = useState<HotWalletSummaryDto[]>([]);
  const [password, setPassword] = useState("");
  const [name, setName] = useState("Test hot");
  const [mnemonic, setMnemonic] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [walletId, setWalletId] = useState(getActiveWalletId() ?? "");
  const [network, setNetwork] = useState<NetworkName>(getPreferredNetwork());
  const [createNested, setCreateNested] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

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
    void (async () => {
      try {
        const items = await listWallets();
        setWallets(items);
        if (!walletId && items[0]) {
          setWalletId(items[0].id);
          setActiveWalletId(items[0].id);
          setNetwork(items[0].network);
        } else if (walletId) {
          const w = items.find((i) => i.id === walletId);
          if (w) setNetwork(w.network);
        }
        await refresh();
      } catch (err) {
        setError(formatError(err));
      }
    })();
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
    if (!walletId) {
      setError("Select a parent wallet first");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const result = await importHotWallet({
        name,
        mnemonicOrJson: mnemonic,
        bip39Passphrase: passphrase || undefined,
        network,
        walletId,
        createNestedVault: createNested,
      });
      setMnemonic("");
      setPassphrase("");
      setMessage(
        result.vault
          ? `Imported “${result.hotWallet.name}” → vault ${result.vault.name}`
          : `Imported “${result.hotWallet.name}” (no nested vault)`,
      );
      await refresh();
      if (result.vault) {
        navigate(`/vaults/${result.vault.id}`);
      }
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

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Hot wallets</h2>
          <p>
            BIP-39 import for quick test signing (Sparrow-like software wallet).
            Nested vault lives under a Minisatoshi wallet. Encrypted with master
            password (Argon2id + XChaCha20).
          </p>
        </div>
        <Link className="button-link" to="/vaults">
          Vaults
        </Link>
      </header>

      {error ? <pre className="error">{error}</pre> : null}
      {message ? <p className="status">{message}</p> : null}

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
          <form className="form-grid" onSubmit={(e) => void onCreateOrUnlock(e)}>
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
          <button type="button" className="secondary" onClick={() => void onLock()}>
            Lock keystore
          </button>
        )}
      </div>

      {status?.unlocked ? (
        <>
          <form className="panel form-grid" onSubmit={(e) => void onImport(e)}>
            <h3>Import BIP-39 / Sparrow seed JSON</h3>
            <label>
              Parent wallet (nested vault)
              <select
                value={walletId}
                onChange={(e) => {
                  setWalletId(e.target.value);
                  setActiveWalletId(e.target.value);
                  const w = wallets.find((i) => i.id === e.target.value);
                  if (w) setNetwork(w.network);
                }}
                required
              >
                {wallets.map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.name} ({formatNetwork(w.network)})
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
            <label>
              Mnemonic (12/24 words) or JSON {"{ \"mnemonic\": \"…\" }"}
              <textarea
                className="mono"
                rows={4}
                value={mnemonic}
                onChange={(e) => setMnemonic(e.target.value)}
                required
                autoComplete="off"
              />
            </label>
            <label>
              BIP-39 passphrase (optional)
              <input
                type="password"
                value={passphrase}
                onChange={(e) => setPassphrase(e.target.value)}
                autoComplete="off"
              />
            </label>
            <label className="check-row">
              <input
                type="checkbox"
                checked={createNested}
                onChange={(e) => setCreateNested(e.target.checked)}
              />
              <span>
                Create nested Taproot vault (policy <span className="mono">A</span>
                ) under parent wallet — Sparrow-style “wallet in wallet”
              </span>
            </label>
            <p className="muted">
              Derives BIP-86 account on {formatNetwork(network)}. Use testnet /
              signet for experiments.
            </p>
            <button type="submit" disabled={busy || !mnemonic.trim()}>
              {busy ? "Importing…" : "Import hot wallet"}
            </button>
          </form>

          <div className="panel">
            <h3>Stored hot wallets</h3>
            {hotWallets.length === 0 ? (
              <p className="muted">None yet.</p>
            ) : (
              <ul className="list">
                {hotWallets.map((hw) => (
                  <li key={hw.id} className="list-item">
                    <div>
                      <strong>{hw.name}</strong>
                      <div className="muted">
                        {formatNetwork(hw.network)} · fp{" "}
                        <span className="mono">{hw.fingerprint}</span>
                      </div>
                      <div className="mono wrap muted">{hw.xpub.slice(0, 24)}…</div>
                    </div>
                    <div className="row-actions">
                      {hw.linkedVaultId ? (
                        <Link
                          className="button-link"
                          to={`/vaults/${hw.linkedVaultId}`}
                        >
                          Open vault
                        </Link>
                      ) : null}
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
        </>
      ) : null}
    </section>
  );
}
