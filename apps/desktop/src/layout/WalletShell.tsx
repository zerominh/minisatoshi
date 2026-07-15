import { NavLink, Outlet, Link } from "react-router-dom";
import { formatNetwork, formatSats } from "../lib/settings";
import { useVault } from "../vault/VaultContext";

const tabs = [
  { to: "transactions", label: "Transactions", end: true },
  { to: "send", label: "Send" },
  { to: "sign-psbt", label: "Import PSBT" },
  { to: "receive", label: "Receive" },
  { to: "addresses", label: "Addresses" },
  { to: "utxos", label: "UTXOs" },
  { to: "settings", label: "Settings" },
] as const;

export function WalletShell() {
  const {
    vault,
    sync,
    busy,
    syncing,
    error,
    runSync,
    kind,
    listPath,
    hotWalletId,
  } = useVault();

  if (!vault && !error) {
    return <p className="muted">Loading…</p>;
  }
  if (!vault) {
    return null;
  }

  const label = kind === "hot" ? "Hot wallet" : "Vault";
  const listLabel = kind === "hot" ? "All hot wallets" : "All vaults";
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
              <span className="badge">Hot</span>
            ) : vault.watchOnly ? (
              <span className="badge watch-only">Watch-only</span>
            ) : null}
          </p>
          {sync ? (
            <p className="vault-balance">
              {formatSats(sync.balance.confirmedSats)}
            </p>
          ) : (
            <p className="muted">Not synced</p>
          )}
          <button
            type="button"
            className="secondary"
            disabled={busy}
            onClick={() => void runSync()}
            title="Sync now (background refresh every 2 min; does not lock the UI)"
          >
            {busy ? "Syncing…" : syncing ? "Updating…" : sync ? "Synced" : "Sync"}
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
              {tab.label}
            </NavLink>
          ))}
        </nav>
        <div className="vault-nav-foot">
          <Link className="button-link" to={shareTo}>
            Share
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
