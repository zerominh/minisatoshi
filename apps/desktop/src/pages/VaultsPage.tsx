import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useFlash } from "../flash/FlashContext";
import { useT } from "../i18n/LocaleContext";
import {
  deleteVault,
  formatError,
  hotKeystoreStatus,
  listHotWallets,
  listVaults,
  listWallets,
  renameVault,
} from "../lib/api";
import { formatNetwork, getActiveWalletId, setActiveWalletId } from "../lib/settings";
import type { VaultSummaryDto, WalletSummaryDto } from "../lib/types";

export function VaultsPage() {
  const t = useT();
  const { setError, setMessage } = useFlash();
  const [wallets, setWallets] = useState<WalletSummaryDto[]>([]);
  const [walletId, setWalletId] = useState<string | null>(getActiveWalletId());
  const [vaults, setVaults] = useState<VaultSummaryDto[]>([]);
  const [busy, setBusy] = useState(false);

  async function refreshVaults(id: string | null) {
    if (!id) {
      setVaults([]);
      return;
    }
    const all = await listVaults(id);
    // Hot wallets own their UI under /hot-wallets — hide their storage rows here.
    let hide = new Set<string>();
    try {
      const st = await hotKeystoreStatus();
      if (st.unlocked) {
        const hot = await listHotWallets();
        hide = new Set(
          hot
            .map((h) => h.linkedVaultId)
            .filter((vid): vid is string => Boolean(vid)),
        );
      }
    } catch {
      // keystore locked / unavailable — show full list
    }
    setVaults(all.filter((v) => !hide.has(v.id)));
  }

  useEffect(() => {
    void (async () => {
      try {
        const items = await listWallets();
        setWallets(items);
        const selected =
          walletId && items.some((w) => w.id === walletId)
            ? walletId
            : (items[0]?.id ?? null);
        if (selected !== walletId) {
          setWalletId(selected);
          if (selected) setActiveWalletId(selected);
        }
      } catch (err) {
        setError(formatError(err));
      }
    })();
  }, []);

  useEffect(() => {
    void refreshVaults(walletId).catch((err) => setError(formatError(err)));
  }, [walletId]);

  async function onRename(vault: VaultSummaryDto) {
    const next = window.prompt(t("vaults.renamePrompt"), vault.name)?.trim();
    if (!next || next === vault.name) return;
    setBusy(true);
    setError(null);
    try {
      await renameVault(vault.id, next);
      setMessage(t("vaults.renamed", { name: next }));
      await refreshVaults(walletId);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onDelete(vault: VaultSummaryDto) {
    const ok = window.confirm(t("vaults.deleteConfirm", { name: vault.name }));
    if (!ok) return;
    setBusy(true);
    setError(null);
    try {
      await deleteVault(vault.id);
      await refreshVaults(walletId);
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
          <h2>{t("vaults.title")}</h2>
          <p>{t("newVault.subtitle")}</p>
        </div>
        <div className="row-actions">
          <Link className="button-link" to="/vaults/import">
            {t("vaults.import")}
          </Link>
          <Link className="button-link primary" to="/vaults/new">
            {t("vaults.create")}
          </Link>
        </div>
      </header>

      {wallets.length === 0 ? (
        <div className="panel">
          <p className="muted">{t("wallets.empty")}</p>
          <Link className="button-link" to="/wallets">
            {t("nav.wallets")}
          </Link>
        </div>
      ) : (
        <>
          <div className="panel form-grid">
            <label>
              {t("vaults.walletFilter")}
              <select
                value={walletId ?? ""}
                onChange={(e) => {
                  setWalletId(e.target.value);
                  setActiveWalletId(e.target.value);
                }}
              >
                {wallets.map((wallet) => (
                  <option key={wallet.id} value={wallet.id}>
                    {wallet.name} ({formatNetwork(wallet.network)})
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className="panel">
            {vaults.length === 0 ? (
              <p className="muted">{t("vaults.empty")}</p>
            ) : (
              <ul className="list">
                {vaults.map((vault) => (
                  <li key={vault.id} className="list-item">
                    <div>
                      <strong>{vault.name}</strong>
                      <div className="muted">
                        {vault.scriptType}{" "}
                        {vault.watchOnly ? (
                          <span className="badge watch-only">
                            {t("shell.watchOnly")}
                          </span>
                        ) : null}
                      </div>
                    </div>
                    <div className="row-actions">
                      <Link className="button-link" to={`/vaults/${vault.id}`}>
                        {t("common.open")}
                      </Link>
                      <Link
                        className="button-link"
                        to={`/vaults/${vault.id}/share`}
                      >
                        {t("common.share")}
                      </Link>
                      <Link
                        className="button-link"
                        to={`/vaults/${vault.id}/receive`}
                      >
                        {t("shell.tab.receive")}
                      </Link>
                      <Link
                        className="button-link"
                        to={`/vaults/${vault.id}/send`}
                      >
                        {t("shell.tab.send")}
                      </Link>
                      <button
                        type="button"
                        className="secondary"
                        disabled={busy}
                        onClick={() => void onRename(vault)}
                      >
                        {t("common.rename")}
                      </button>
                      <button
                        type="button"
                        className="secondary"
                        disabled={busy}
                        onClick={() => void onDelete(vault)}
                      >
                        {t("common.delete")}
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </>
      )}
    </section>
  );
}
