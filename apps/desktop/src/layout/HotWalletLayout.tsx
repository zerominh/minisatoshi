import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { useFlash } from "../flash/FlashContext";
import { formatError, openHotWallet } from "../lib/api";
import { VaultProvider } from "../vault/VaultContext";
import { WalletShell } from "./WalletShell";

/**
 * Hot wallet detail = send / receive / tx like any Bitcoin wallet.
 * Storage still uses the vault row under the hood; UI never calls it a vault.
 */
export function HotWalletLayout() {
  const { id: hotWalletId = "" } = useParams();
  const { setError, clear } = useFlash();
  const [vaultId, setVaultId] = useState<string | null>(null);
  const [opening, setOpening] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setVaultId(null);
    setOpening(true);
    clear();
    void (async () => {
      try {
        const vault = await openHotWallet(hotWalletId);
        if (!cancelled) {
          setVaultId(vault.id);
          setOpening(false);
        }
      } catch (err) {
        if (!cancelled) {
          setError(formatError(err));
          setOpening(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [hotWalletId, clear, setError]);

  if (!vaultId) {
    return opening ? <p className="muted">Opening wallet…</p> : null;
  }

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
