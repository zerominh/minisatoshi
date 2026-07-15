import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { useFlash } from "../flash/FlashContext";
import { formatError, openHotWallet } from "../lib/api";
import { WalletProvider } from "../wallet/WalletContext";
import { WalletShell } from "./WalletShell";

/**
 * Hot wallet detail = send / receive / tx like any Bitcoin wallet.
 * Storage still uses the wallet row under the hood; UI never calls it a vault.
 */
export function HotWalletLayout() {
  const { id: hotWalletId = "" } = useParams();
  const { setError, clear } = useFlash();
  const [walletId, setWalletId] = useState<string | null>(null);
  const [opening, setOpening] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setWalletId(null);
    setOpening(true);
    clear();
    void (async () => {
      try {
        const wallet = await openHotWallet(hotWalletId);
        if (!cancelled) {
          setWalletId(wallet.id);
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

  if (!walletId) {
    return opening ? <p className="muted">Opening wallet…</p> : null;
  }

  return (
    <WalletProvider
      walletId={walletId}
      kind="hot"
      hotWalletId={hotWalletId}
      listPath="/hot-wallets"
    >
      <WalletShell />
    </WalletProvider>
  );
}
