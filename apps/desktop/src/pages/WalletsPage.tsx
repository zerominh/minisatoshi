import { FormEvent, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useFlash } from "../flash/FlashContext";
import { useT } from "../i18n/LocaleContext";
import {
  createWallet,
  deleteWallet,
  formatError,
  listWallets,
  renameWallet,
} from "../lib/api";
import {
  formatNetwork,
  getActiveWalletId,
  getPreferredNetwork,
  setActiveWalletId,
} from "../lib/settings";
import type { NetworkName, WalletSummaryDto } from "../lib/types";

export function WalletsPage() {
  const t = useT();
  const { setError, setMessage } = useFlash();
  const [wallets, setWallets] = useState<WalletSummaryDto[]>([]);
  const [name, setName] = useState("");
  const [network, setNetwork] = useState<NetworkName>(getPreferredNetwork());
  const [activeId, setActiveId] = useState<string | null>(getActiveWalletId());
  const [busy, setBusy] = useState(false);

  async function refresh() {
    const items = await listWallets();
    setWallets(items);
    const stillActive =
      activeId && items.some((w) => w.id === activeId) ? activeId : null;
    if (stillActive) {
      setActiveId(stillActive);
    } else if (items.length > 0) {
      setActiveWalletId(items[0].id);
      setActiveId(items[0].id);
    } else {
      setActiveWalletId(null);
      setActiveId(null);
    }
  }

  useEffect(() => {
    void refresh().catch((err) => setError(formatError(err)));
  }, []);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const wallet = await createWallet({ name, network });
      setActiveWalletId(wallet.id);
      setActiveId(wallet.id);
      setName("");
      await refresh();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  function selectWallet(id: string) {
    setActiveWalletId(id);
    setActiveId(id);
  }

  async function onRename(wallet: WalletSummaryDto) {
    const next = window.prompt(t("wallets.renamePrompt"), wallet.name)?.trim();
    if (!next || next === wallet.name) return;
    setBusy(true);
    setError(null);
    try {
      await renameWallet(wallet.id, next);
      setMessage(t("wallets.renamed", { name: next }));
      await refresh();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onDelete(wallet: WalletSummaryDto) {
    const ok = window.confirm(
      t("wallets.deleteConfirm", { name: wallet.name }),
    );
    if (!ok) return;
    setBusy(true);
    setError(null);
    try {
      await deleteWallet(wallet.id);
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
          <h2>{t("wallets.title")}</h2>
          <p>{t("wallets.subtitle")}</p>
        </div>
      </header>

      <form className="panel form-grid" onSubmit={(e) => void onSubmit(e)}>
        <h3>{t("wallets.new")}</h3>
        <label>
          {t("wallets.name")}
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Family fund"
            required
          />
        </label>
        <label>
          {t("wallets.network")}
          <select
            value={network}
            onChange={(e) => setNetwork(e.target.value as NetworkName)}
          >
            <option value="testnet">Testnet3</option>
            <option value="testnet4">Testnet4</option>
            <option value="signet">Signet</option>
            <option value="regtest">Regtest</option>
            <option value="mainnet">Mainnet</option>
          </select>
        </label>
        <button type="submit" disabled={busy || !name.trim()}>
          {busy ? t("common.busy") : t("wallets.new")}
        </button>
      </form>

      <div className="panel">
        <h3>{t("wallets.title")}</h3>
        {wallets.length === 0 ? (
          <p className="muted">{t("wallets.empty")}</p>
        ) : (
          <ul className="list">
            {wallets.map((wallet) => (
              <li key={wallet.id} className="list-item">
                <div>
                  <strong>{wallet.name}</strong>
                  <div className="muted">
                    {formatNetwork(wallet.network)} ·{" "}
                    {t("wallets.vaultCount", { n: wallet.vaultCount })}
                  </div>
                </div>
                <div className="row-actions">
                  {activeId === wallet.id ? (
                    <span className="badge">{t("common.use")}</span>
                  ) : (
                    <button
                      type="button"
                      className="secondary"
                      onClick={() => selectWallet(wallet.id)}
                    >
                      {t("common.use")}
                    </button>
                  )}
                  <Link className="button-link" to="/vaults">
                    {t("nav.vaults")}
                  </Link>
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    onClick={() => void onRename(wallet)}
                  >
                    {t("common.rename")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    onClick={() => void onDelete(wallet)}
                  >
                    {t("common.delete")}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}
