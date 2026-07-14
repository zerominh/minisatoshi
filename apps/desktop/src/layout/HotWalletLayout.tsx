import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { formatError, openHotWallet } from "../lib/api";
import { VaultProvider } from "../vault/VaultContext";
import { WalletShell } from "./WalletShell";

/**
 * Hot wallet detail = send / receive / tx like any Bitcoin wallet.
 * Storage still uses the vault row under the hood; UI never calls it a vault.
 */
export function HotWalletLayout() {
  const { id: hotWalletId = "" } = useParams();
  const [vaultId, setVaultId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setVaultId(null);
    setError(null);
    void (async () => {
      try {
        const vault = await openHotWallet(hotWalletId);
        if (!cancelled) setVaultId(vault.id);
      } catch (err) {
        if (!cancelled) setError(formatError(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [hotWalletId]);

  if (error) return <pre className="error">{error}</pre>;
  if (!vaultId) return <p className="muted">Opening wallet…</p>;

  return (
    <VaultProvider
      vaultId={vaultId}
      kind="hot"
      hotWalletId={hotWalletId}
      listPath="/hot-wallets"
    >
      <WalletShell />
    </VaultProvider>
  );
}
