import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useParams } from "react-router-dom";
import { useFlash } from "../flash/FlashContext";
import { formatError, getVault, syncVault } from "../lib/api";
import { getEsploraUrl } from "../lib/settings";
import type { SyncResultDto, VaultDto } from "../lib/types";

export type WalletShellKind = "vault" | "hot";

/** Skip open-sync if cache is newer than this (no periodic polling). */
const FRESH_SYNC_MS = 30_000;

type SyncOptions = {
  /** Background sync: no busy lock, no toast, soft errors. */
  quiet?: boolean;
};

type CacheEntry = {
  result: SyncResultDto;
  at: number;
};

type VaultContextValue = {
  /** Internal storage id (descriptor / UTXOs) — same engine for vault & hot. */
  vaultId: string;
  vault: VaultDto | null;
  sync: SyncResultDto | null;
  /** True only during a manual Sync click — backgrounds never set this. */
  busy: boolean;
  /** True while any sync (manual or auto) is in flight. */
  syncing: boolean;
  error: string | null;
  message: string | null;
  kind: WalletShellKind;
  hotWalletId: string | null;
  /** Back-nav target for the shell footer. */
  listPath: string;
  setError: (value: string | null) => void;
  setMessage: (value: string | null) => void;
  refreshVault: () => Promise<void>;
  runSync: (options?: SyncOptions) => Promise<SyncResultDto | null>;
};

const VaultContext = createContext<VaultContextValue | null>(null);

const syncCache = new Map<string, CacheEntry>();

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
  const {
    error,
    message,
    setError,
    setMessage,
    clear: clearFlash,
  } = useFlash();
  const [vault, setVault] = useState<VaultDto | null>(null);
  const [sync, setSync] = useState<SyncResultDto | null>(
    () => syncCache.get(id)?.result ?? null,
  );
  const [busy, setBusy] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const syncingRef = useRef(false);
  const idRef = useRef(id);
  idRef.current = id;

  const refreshVault = useCallback(async () => {
    if (!id) return;
    const next = await getVault(id);
    setVault(next);
  }, [id]);

  const runSync = useCallback(
    async (options?: SyncOptions) => {
      const quiet = options?.quiet === true;
      const vaultId = idRef.current;
      if (!vaultId || syncingRef.current) return null;
      if (quiet && typeof document !== "undefined" && document.hidden) {
        return null;
      }

      syncingRef.current = true;
      setSyncing(true);
      if (!quiet) {
        setBusy(true);
        setError(null);
      }
      try {
        const result = await syncVault(vaultId, getEsploraUrl() || undefined);
        // Drop result if user navigated away mid-flight.
        if (idRef.current !== vaultId) return null;
        syncCache.set(vaultId, { result, at: Date.now() });
        setSync(result);
        if (!quiet) setMessage("Chain sync complete");
        return result;
      } catch (err) {
        if (idRef.current !== vaultId) return null;
        // Background failures must not freeze tabs with a blocking error banner.
        if (!quiet) setError(formatError(err));
        return null;
      } finally {
        syncingRef.current = false;
        setSyncing(false);
        if (!quiet) setBusy(false);
      }
    },
    [setError, setMessage],
  );

  useEffect(() => {
    const cached = syncCache.get(id);
    setSync(cached?.result ?? null);
    clearFlash();
    setBusy(false);
    let cancelled = false;

    void (async () => {
      try {
        await refreshVault();
      } catch (err) {
        if (!cancelled) setError(formatError(err));
        return;
      }
      if (cancelled) return;

      const age = cached ? Date.now() - cached.at : Number.POSITIVE_INFINITY;
      if (age < FRESH_SYNC_MS) return;

      // One sync on open only — no interval / tab-focus polling (Esplora rate limits).
      await new Promise((r) => window.setTimeout(r, 400));
      if (cancelled) return;
      await runSync({ quiet: true });
    })();

    return () => {
      cancelled = true;
    };
  }, [id, refreshVault, runSync, clearFlash, setError]);

  const resolvedListPath =
    listPath ?? (kind === "hot" ? "/hot-wallets" : "/vaults");

  const value = useMemo(
    () => ({
      vaultId: id,
      vault,
      sync,
      busy,
      syncing,
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
      syncing,
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
