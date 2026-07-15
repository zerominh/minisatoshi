import { FormEvent, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useFlash } from "../flash/FlashContext";
import { useT } from "../i18n/LocaleContext";
import {
  createWorkspace,
  deleteWorkspace,
  formatError,
  listWorkspaces,
  renameWorkspace,
} from "../lib/api";
import {
  formatNetwork,
  getActiveWorkspaceId,
  getPreferredNetwork,
  setActiveWorkspaceId,
} from "../lib/settings";
import type { NetworkName, WorkspaceSummaryDto } from "../lib/types";

export function WorkspacesPage() {
  const t = useT();
  const { setError, setMessage } = useFlash();
  const [workspaces, setWorkspaces] = useState<WorkspaceSummaryDto[]>([]);
  const [name, setName] = useState("");
  const [network, setNetwork] = useState<NetworkName>(getPreferredNetwork());
  const [activeId, setActiveId] = useState<string | null>(
    getActiveWorkspaceId(),
  );
  const [busy, setBusy] = useState(false);

  async function refresh() {
    const items = await listWorkspaces();
    setWorkspaces(items);
    const stillActive =
      activeId && items.some((w) => w.id === activeId) ? activeId : null;
    if (stillActive) {
      setActiveId(stillActive);
    } else if (items.length > 0) {
      setActiveWorkspaceId(items[0].id);
      setActiveId(items[0].id);
    } else {
      setActiveWorkspaceId(null);
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
      const workspace = await createWorkspace({ name, network });
      setActiveWorkspaceId(workspace.id);
      setActiveId(workspace.id);
      setName("");
      await refresh();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  function selectWorkspace(id: string) {
    setActiveWorkspaceId(id);
    setActiveId(id);
  }

  async function onRename(workspace: WorkspaceSummaryDto) {
    const next = window
      .prompt(t("workspaces.renamePrompt"), workspace.name)
      ?.trim();
    if (!next || next === workspace.name) return;
    setBusy(true);
    setError(null);
    try {
      await renameWorkspace(workspace.id, next);
      setMessage(t("workspaces.renamed", { name: next }));
      await refresh();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onDelete(workspace: WorkspaceSummaryDto) {
    const ok = window.confirm(
      t("workspaces.deleteConfirm", { name: workspace.name }),
    );
    if (!ok) return;
    setBusy(true);
    setError(null);
    try {
      await deleteWorkspace(workspace.id);
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
          <h2>{t("workspaces.title")}</h2>
          <p>{t("workspaces.subtitle")}</p>
        </div>
      </header>

      <form className="panel form-grid" onSubmit={(e) => void onSubmit(e)}>
        <h3>{t("workspaces.new")}</h3>
        <label>
          {t("workspaces.name")}
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Family fund"
            required
          />
        </label>
        <label>
          {t("workspaces.network")}
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
          {busy ? t("common.busy") : t("workspaces.new")}
        </button>
      </form>

      <div className="panel">
        <h3>{t("workspaces.title")}</h3>
        {workspaces.length === 0 ? (
          <p className="muted">{t("workspaces.empty")}</p>
        ) : (
          <ul className="list">
            {workspaces.map((workspace) => (
              <li key={workspace.id} className="list-item">
                <div>
                  <strong>{workspace.name}</strong>
                  <div className="muted">
                    {formatNetwork(workspace.network)} ·{" "}
                    {t("workspaces.walletCount", { n: workspace.walletCount })}
                  </div>
                </div>
                <div className="row-actions">
                  {activeId === workspace.id ? (
                    <span className="badge">{t("common.use")}</span>
                  ) : (
                    <button
                      type="button"
                      className="secondary"
                      onClick={() => selectWorkspace(workspace.id)}
                    >
                      {t("common.use")}
                    </button>
                  )}
                  <Link className="button-link" to="/wallets">
                    {t("nav.wallets")}
                  </Link>
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    onClick={() => void onRename(workspace)}
                  >
                    {t("common.rename")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    onClick={() => void onDelete(workspace)}
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
