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
import { useT } from "../i18n/LocaleContext";
import { formatError, getWallet, syncWallet } from "../lib/api";
import { getEsploraUrl } from "../lib/settings";
import type { SyncResultDto, WalletDto } from "../lib/types";

export type WalletShellKind = "wallet" | "hot";

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

type WalletContextValue = {
  /** Internal storage id (descriptor / UTXOs) — same engine for wallet & hot. */
  walletId: string;
  wallet: WalletDto | null;
  sync: SyncResultDto | null;
  /** Client timestamp of last successful sync (`null` if never). */
  lastSyncedAt: number | null;
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
  refreshWallet: () => Promise<void>;
  runSync: (options?: SyncOptions) => Promise<SyncResultDto | null>;
};

const WalletContext = createContext<WalletContextValue | null>(null);

const syncCache = new Map<string, CacheEntry>();

type ProviderProps = {
  children: ReactNode;
  /** When set (hot shell), ignore route `:id` for wallet fetch. */
  walletId?: string;
  kind?: WalletShellKind;
  hotWalletId?: string | null;
  listPath?: string;
};

export function WalletProvider({
  children,
  walletId: walletIdProp,
  kind = "wallet",
  hotWalletId = null,
  listPath,
}: ProviderProps) {
  const { id: routeId = "" } = useParams();
  const id = walletIdProp ?? routeId;
  const {
    error,
    message,
    setError,
    setMessage,
    clear: clearFlash,
  } = useFlash();
  const t = useT();
  const [wallet, setWallet] = useState<WalletDto | null>(null);
  const [sync, setSync] = useState<SyncResultDto | null>(
    () => syncCache.get(id)?.result ?? null,
  );
  const [lastSyncedAt, setLastSyncedAt] = useState<number | null>(
    () => syncCache.get(id)?.at ?? null,
  );
  const [busy, setBusy] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const syncingRef = useRef(false);
  const idRef = useRef(id);
  idRef.current = id;

  const refreshWallet = useCallback(async () => {
    if (!id) return;
    const next = await getWallet(id);
    setWallet(next);
  }, [id]);

  const runSync = useCallback(
    async (options?: SyncOptions) => {
      const quiet = options?.quiet === true;
      const walletId = idRef.current;
      if (!walletId || syncingRef.current) return null;
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
        const result = await syncWallet(walletId, getEsploraUrl() || undefined);
        // Drop result if user navigated away mid-flight.
        if (idRef.current !== walletId) return null;
        const at = Date.now();
        syncCache.set(walletId, { result, at });
        setSync(result);
        setLastSyncedAt(at);
        if (!quiet) setMessage(t("sync.chainComplete"));
        return result;
      } catch (err) {
        if (idRef.current !== walletId) return null;
        // Background failures must not freeze tabs with a blocking error banner.
        if (!quiet) setError(formatError(err));
        return null;
      } finally {
        syncingRef.current = false;
        setSyncing(false);
        if (!quiet) setBusy(false);
      }
    },
    [setError, setMessage, t],
  );

  useEffect(() => {
    const cached = syncCache.get(id);
    setSync(cached?.result ?? null);
    setLastSyncedAt(cached?.at ?? null);
    clearFlash();
    setBusy(false);
    let cancelled = false;

    void (async () => {
      try {
        await refreshWallet();
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
  }, [id, refreshWallet, runSync, clearFlash, setError]);

  const resolvedListPath =
    listPath ?? (kind === "hot" ? "/hot-wallets" : "/wallets");

  const value = useMemo(
    () => ({
      walletId: id,
      wallet,
      sync,
      lastSyncedAt,
      busy,
      syncing,
      error,
      message,
      kind,
      hotWalletId,
      listPath: resolvedListPath,
      setError,
      setMessage,
      refreshWallet,
      runSync,
    }),
    [
      id,
      wallet,
      sync,
      lastSyncedAt,
      busy,
      syncing,
      error,
      message,
      kind,
      hotWalletId,
      resolvedListPath,
      refreshWallet,
      runSync,
    ],
  );

  return (
    <WalletContext.Provider value={value}>{children}</WalletContext.Provider>
  );
}

export function useWallet(): WalletContextValue {
  const ctx = useContext(WalletContext);
  if (!ctx) {
    throw new Error("useWallet must be used inside WalletProvider");
  }
  return ctx;
}

/** Prefer context wallet id when inside a shell; else route param. */
export function useWalletIdFromRouteOrContext(): string {
  const { id = "" } = useParams();
  const ctx = useContext(WalletContext);
  return ctx?.walletId || id;
}
