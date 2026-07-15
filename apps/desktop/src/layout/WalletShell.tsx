import { useCallback, useState } from "react";
import { NavLink, Outlet, Link } from "react-router-dom";
import { useT } from "../i18n/LocaleContext";
import { formatSyncAge } from "../lib/formatSyncAge";
import { formatNetwork, formatSats } from "../lib/settings";
import { useWallet } from "../wallet/WalletContext";

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
    wallet,
    sync,
    lastSyncedAt,
    busy,
    syncing,
    error,
    runSync,
    kind,
    listPath,
    hotWalletId,
  } = useWallet();
  const [syncTitle, setSyncTitle] = useState(() =>
    formatSyncAge(lastSyncedAt, t),
  );

  const refreshSyncTitle = useCallback(() => {
    setSyncTitle(formatSyncAge(lastSyncedAt, t));
  }, [lastSyncedAt, t]);

  if (!wallet && !error) {
    return <p className="muted">{t("common.loading")}</p>;
  }
  if (!wallet) {
    return null;
  }

  const label = kind === "hot" ? t("shell.hotWallet") : t("shell.wallet");
  const listLabel =
    kind === "hot" ? t("shell.allHotWallets") : t("shell.allWallets");
  const shareTo =
    kind === "hot" && hotWalletId
      ? `/hot-wallets/${hotWalletId}/share`
      : `/wallets/${wallet.id}/share`;

  return (
    <div className="wallet-shell">
      <aside className="wallet-nav">
        <div className="wallet-nav-head">
          <p className="muted">{label}</p>
          <strong>{wallet.name}</strong>
          <p className="muted">
            {wallet.scriptType} · {formatNetwork(wallet.policy.network)}
            {kind === "hot" ? (
              <span className="badge">{t("shell.hotBadge")}</span>
            ) : wallet.watchOnly ? (
              <span className="badge watch-only">{t("shell.watchOnly")}</span>
            ) : null}
          </p>
          {sync ? (
            <p className="wallet-balance">
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
                isActive ? "wallet-nav-active" : undefined
              }
            >
              {t(tab.labelKey)}
            </NavLink>
          ))}
        </nav>
        <div className="wallet-nav-foot">
          <Link className="button-link" to={shareTo}>
            {t("common.share")}
          </Link>
          <Link className="button-link" to={listPath}>
            {listLabel}
          </Link>
        </div>
      </aside>
      <div className="wallet-main">
        <Outlet />
      </div>
    </div>
  );
}
