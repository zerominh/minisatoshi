import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useFlash } from "../flash/FlashContext";
import { useT } from "../i18n/LocaleContext";
import {
  deleteWallet,
  formatError,
  hotKeystoreStatus,
  listHotWallets,
  listWallets,
  listWorkspaces,
  renameWallet,
} from "../lib/api";
import {
  formatNetwork,
  getActiveWorkspaceId,
  setActiveWorkspaceId,
} from "../lib/settings";
import type { WalletSummaryDto, WorkspaceSummaryDto } from "../lib/types";

export function WalletsPage() {
  const t = useT();
  const { setError, setMessage } = useFlash();
  const [workspaces, setWorkspaces] = useState<WorkspaceSummaryDto[]>([]);
  const [workspaceId, setWorkspaceId] = useState<string | null>(
    getActiveWorkspaceId(),
  );
  const [wallets, setWallets] = useState<WalletSummaryDto[]>([]);
  const [busy, setBusy] = useState(false);

  async function refreshWallets(id: string | null) {
    if (!id) {
      setWallets([]);
      return;
    }
    const all = await listWallets(id);
    // Hot wallets own their UI under /hot-wallets — hide their storage rows here.
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
      // keystore locked / unavailable — show full list
    }
    setWallets(all.filter((w) => !hide.has(w.id)));
  }

  useEffect(() => {
    void (async () => {
      try {
        const items = await listWorkspaces();
        setWorkspaces(items);
        const selected =
          workspaceId && items.some((w) => w.id === workspaceId)
            ? workspaceId
            : (items[0]?.id ?? null);
        if (selected !== workspaceId) {
          setWorkspaceId(selected);
          if (selected) setActiveWorkspaceId(selected);
        }
      } catch (err) {
        setError(formatError(err));
      }
    })();
  }, []);

  useEffect(() => {
    void refreshWallets(workspaceId).catch((err) => setError(formatError(err)));
  }, [workspaceId]);

  async function onRename(wallet: WalletSummaryDto) {
    const next = window.prompt(t("wallets.renamePrompt"), wallet.name)?.trim();
    if (!next || next === wallet.name) return;
    setBusy(true);
    setError(null);
    try {
      await renameWallet(wallet.id, next);
      setMessage(t("wallets.renamed", { name: next }));
      await refreshWallets(workspaceId);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onDelete(wallet: WalletSummaryDto) {
    const ok = window.confirm(t("wallets.deleteConfirm", { name: wallet.name }));
    if (!ok) return;
    setBusy(true);
    setError(null);
    try {
      await deleteWallet(wallet.id);
      await refreshWallets(workspaceId);
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
          <p>{t("newWallet.subtitle")}</p>
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

      {workspaces.length === 0 ? (
        <div className="panel">
          <p className="muted">{t("workspaces.empty")}</p>
          <Link className="button-link" to="/workspaces">
            {t("nav.workspaces")}
          </Link>
        </div>
      ) : (
        <>
          <div className="panel form-grid">
            <label>
              {t("wallets.workspaceFilter")}
              <select
                value={workspaceId ?? ""}
                onChange={(e) => {
                  setWorkspaceId(e.target.value);
                  setActiveWorkspaceId(e.target.value);
                }}
              >
                {workspaces.map((workspace) => (
                  <option key={workspace.id} value={workspace.id}>
                    {workspace.name} ({formatNetwork(workspace.network)})
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className="panel">
            {wallets.length === 0 ? (
              <p className="muted">{t("wallets.empty")}</p>
            ) : (
              <ul className="list">
                {wallets.map((wallet) => (
                  <li key={wallet.id} className="list-item">
                    <div>
                      <strong>{wallet.name}</strong>
                      <div className="muted">
                        {wallet.scriptType}{" "}
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
        </>
      )}
    </section>
  );
}
