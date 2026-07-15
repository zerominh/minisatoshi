import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useFlash } from "../flash/FlashContext";
import { useT } from "../i18n/LocaleContext";
import {
  deleteWallet,
  formatError,
  hotKeystoreStatus,
  listHotWallets,
  renameWallet,
} from "../lib/api";
import { formatNetwork } from "../lib/settings";
import {
  listAllWallets,
  type WalletListItem,
} from "../lib/workspaceAuto";
import type { NetworkName } from "../lib/types";

export function WalletsPage() {
  const t = useT();
  const { setError, setMessage } = useFlash();
  const [wallets, setWallets] = useState<WalletListItem[]>([]);
  const [networkFilter, setNetworkFilter] = useState<NetworkName | "all">(
    "all",
  );
  const [busy, setBusy] = useState(false);

  async function refresh() {
    const all = await listAllWallets();
    let hide = new Set<string>();
    try {
      const st = await hotKeystoreStatus();
      if (st.unlocked) {
        const hot = await listHotWallets();
        hide = new Set(
          hot
            .map((h) => h.linkedWalletId)
            .filter((wid): wid is string => Boolean(wid)),
        );
      }
    } catch {
      // keystore locked
    }
    setWallets(all.filter((w) => !hide.has(w.id)));
  }

  useEffect(() => {
    void refresh().catch((err) => setError(formatError(err)));
  }, [setError]);

  const visible =
    networkFilter === "all"
      ? wallets
      : wallets.filter((w) => w.network === networkFilter);

  async function onRename(wallet: WalletListItem) {
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

  async function onDelete(wallet: WalletListItem) {
    const ok = window.confirm(t("wallets.deleteConfirm", { name: wallet.name }));
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
        <div className="row-actions">
          <Link className="button-link" to="/wallets/import">
            {t("wallets.import")}
          </Link>
          <Link className="button-link primary" to="/wallets/new">
            {t("wallets.create")}
          </Link>
        </div>
      </header>

      <div className="panel form-grid">
        <label>
          {t("wallets.network")}
          <select
            value={networkFilter}
            onChange={(e) =>
              setNetworkFilter(
                e.target.value === "all"
                  ? "all"
                  : (e.target.value as NetworkName),
              )
            }
          >
            <option value="all">{t("wallets.allNetworks")}</option>
            <option value="testnet">Testnet3</option>
            <option value="testnet4">Testnet4</option>
            <option value="signet">Signet</option>
            <option value="regtest">Regtest</option>
            <option value="mainnet">Mainnet</option>
          </select>
        </label>
      </div>

      <div className="panel">
        {visible.length === 0 ? (
          <div>
            <p className="muted">{t("wallets.empty")}</p>
            <Link className="button-link primary" to="/wallets/new">
              {t("wallets.create")}
            </Link>
          </div>
        ) : (
          <ul className="list">
            {visible.map((wallet) => (
              <li key={wallet.id} className="list-item">
                <div>
                  <strong>{wallet.name}</strong>
                  <div className="muted">
                    {wallet.scriptType} · {formatNetwork(wallet.network)}{" "}
                    {wallet.watchOnly ? (
                      <span className="badge watch-only">
                        {t("shell.watchOnly")}
                      </span>
                    ) : null}
                  </div>
                </div>
                <div className="row-actions">
                  <Link className="button-link" to={`/wallets/${wallet.id}`}>
                    {t("common.open")}
                  </Link>
                  <Link
                    className="button-link"
                    to={`/wallets/${wallet.id}/share`}
                  >
                    {t("common.share")}
                  </Link>
                  <Link
                    className="button-link"
                    to={`/wallets/${wallet.id}/receive`}
                  >
                    {t("shell.tab.receive")}
                  </Link>
                  <Link
                    className="button-link"
                    to={`/wallets/${wallet.id}/send`}
                  >
                    {t("shell.tab.send")}
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
