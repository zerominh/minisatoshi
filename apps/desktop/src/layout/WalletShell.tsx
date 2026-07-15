import { useCallback, useState } from "react";
import { NavLink, Outlet, Link } from "react-router-dom";
import { useT } from "../i18n/LocaleContext";
import { formatSyncAge } from "../lib/formatSyncAge";
import { formatNetwork, formatSats } from "../lib/settings";
import { useVault } from "../vault/VaultContext";

const tabs = [
  { to: "transactions", labelKey: "shell.tab.transactions" as const, end: true },
  { to: "send", labelKey: "shell.tab.send" as const },
  { to: "sign-psbt", labelKey: "shell.tab.signPsbt" as const },
  { to: "receive", labelKey: "shell.tab.receive" as const },
  { to: "addresses", labelKey: "shell.tab.addresses" as const },
  { to: "utxos", labelKey: "shell.tab.utxos" as const },
  { to: "settings", labelKey: "shell.tab.settings" as const },
] as const;

export function WalletShell() {
  const t = useT();
  const {
    vault,
    sync,
    lastSyncedAt,
    busy,
    syncing,
    error,
    runSync,
    kind,
    listPath,
    hotWalletId,
  } = useVault();
  const [syncTitle, setSyncTitle] = useState(() =>
    formatSyncAge(lastSyncedAt, t),
  );

  const refreshSyncTitle = useCallback(() => {
    setSyncTitle(formatSyncAge(lastSyncedAt, t));
  }, [lastSyncedAt, t]);

  if (!vault && !error) {
    return <p className="muted">{t("common.loading")}</p>;
  }
  if (!vault) {
    return null;
  }

  const label = kind === "hot" ? t("shell.hotWallet") : t("shell.vault");
  const listLabel =
    kind === "hot" ? t("shell.allHotWallets") : t("shell.allVaults");
  const shareTo =
    kind === "hot" && hotWalletId
      ? `/hot-wallets/${hotWalletId}/share`
      : `/vaults/${vault.id}/share`;

  return (
    <div className="vault-shell">
      <aside className="vault-nav">
        <div className="vault-nav-head">
          <p className="muted">{label}</p>
          <strong>{vault.name}</strong>
          <p className="muted">
            {vault.scriptType} · {formatNetwork(vault.policy.network)}
            {kind === "hot" ? (
              <span className="badge">{t("shell.hotBadge")}</span>
            ) : vault.watchOnly ? (
              <span className="badge watch-only">{t("shell.watchOnly")}</span>
            ) : null}
          </p>
          {sync ? (
            <p className="vault-balance">
              {formatSats(sync.balance.confirmedSats)}
            </p>
          ) : (
            <p className="muted">{t("shell.notSynced")}</p>
          )}
          <button
            type="button"
            className="secondary"
            disabled={busy}
            onClick={() => void runSync()}
            onMouseEnter={refreshSyncTitle}
            onFocus={refreshSyncTitle}
            title={syncTitle}
          >
            {busy
              ? t("common.syncing")
              : syncing
                ? t("common.updating")
                : sync
                  ? t("common.synced")
                  : t("common.sync")}
          </button>
        </div>
        <nav>
          {tabs.map((tab) => (
            <NavLink
              key={tab.to}
              to={tab.to}
              end={"end" in tab ? tab.end : false}
              className={({ isActive }) =>
                isActive ? "vault-nav-active" : undefined
              }
            >
              {t(tab.labelKey)}
            </NavLink>
          ))}
        </nav>
        <div className="vault-nav-foot">
          <Link className="button-link" to={shareTo}>
            {t("common.share")}
          </Link>
          <Link className="button-link" to={listPath}>
            {listLabel}
          </Link>
        </div>
      </aside>
      <div className="vault-main">
        <Outlet />
      </div>
    </div>
  );
}
