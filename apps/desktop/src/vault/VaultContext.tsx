import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { useParams } from "react-router-dom";
import { formatError, getVault, syncVault } from "../lib/api";
import { getEsploraUrl } from "../lib/settings";
import type { SyncResultDto, VaultDto } from "../lib/types";

export type WalletShellKind = "vault" | "hot";

type VaultContextValue = {
  /** Internal storage id (descriptor / UTXOs) — same engine for vault & hot. */
  vaultId: string;
  vault: VaultDto | null;
  sync: SyncResultDto | null;
  busy: boolean;
  error: string | null;
  message: string | null;
  kind: WalletShellKind;
  hotWalletId: string | null;
  /** Back-nav target for the shell footer. */
  listPath: string;
  setError: (value: string | null) => void;
  setMessage: (value: string | null) => void;
  refreshVault: () => Promise<void>;
  runSync: () => Promise<SyncResultDto | null>;
};

const VaultContext = createContext<VaultContextValue | null>(null);

const syncCache = new Map<string, SyncResultDto>();

type ProviderProps = {
  children: ReactNode;
  /** When set (hot shell), ignore route `:id` for vault fetch. */
  vaultId?: string;
  kind?: WalletShellKind;
  hotWalletId?: string | null;
  listPath?: string;
};

export function VaultProvider({
  children,
  vaultId: vaultIdProp,
  kind = "vault",
  hotWalletId = null,
  listPath,
}: ProviderProps) {
  const { id: routeId = "" } = useParams();
  const id = vaultIdProp ?? routeId;
  const [vault, setVault] = useState<VaultDto | null>(null);
  const [sync, setSync] = useState<SyncResultDto | null>(
    () => syncCache.get(id) ?? null,
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const refreshVault = useCallback(async () => {
    if (!id) return;
    const next = await getVault(id);
    setVault(next);
  }, [id]);

  useEffect(() => {
    setSync(syncCache.get(id) ?? null);
    setError(null);
    setMessage(null);
    void refreshVault().catch((err) => setError(formatError(err)));
  }, [id, refreshVault]);

  const runSync = useCallback(async () => {
    if (!id) return null;
    setBusy(true);
    setError(null);
    try {
      const result = await syncVault(id, getEsploraUrl() || undefined);
      syncCache.set(id, result);
      setSync(result);
      setMessage("Chain sync complete");
      return result;
    } catch (err) {
      setError(formatError(err));
      return null;
    } finally {
      setBusy(false);
    }
  }, [id]);

  const resolvedListPath =
    listPath ?? (kind === "hot" ? "/hot-wallets" : "/vaults");

  const value = useMemo(
    () => ({
      vaultId: id,
      vault,
      sync,
      busy,
      error,
      message,
      kind,
      hotWalletId,
      listPath: resolvedListPath,
      setError,
      setMessage,
      refreshVault,
      runSync,
    }),
    [
      id,
      vault,
      sync,
      busy,
      error,
      message,
      kind,
      hotWalletId,
      resolvedListPath,
      refreshVault,
      runSync,
    ],
  );

  return (
    <VaultContext.Provider value={value}>{children}</VaultContext.Provider>
  );
}

export function useVault(): VaultContextValue {
  const ctx = useContext(VaultContext);
  if (!ctx) {
    throw new Error("useVault must be used inside VaultProvider");
  }
  return ctx;
}

/** Prefer context vault id when inside a shell; else route param. */
export function useVaultIdFromRouteOrContext(): string {
  const { id = "" } = useParams();
  const ctx = useContext(VaultContext);
  return ctx?.vaultId || id;
}
