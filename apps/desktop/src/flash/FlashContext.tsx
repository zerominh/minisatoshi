import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";

type FlashContextValue = {
  error: string | null;
  message: string | null;
  setError: (value: string | null) => void;
  setMessage: (value: string | null) => void;
  clear: () => void;
};

const FlashContext = createContext<FlashContextValue | null>(null);

export function FlashProvider({ children }: { children: ReactNode }) {
  const [error, setErrorState] = useState<string | null>(null);
  const [message, setMessageState] = useState<string | null>(null);

  const setError = useCallback((value: string | null) => {
    setErrorState(value);
    if (value) setMessageState(null);
  }, []);

  const setMessage = useCallback((value: string | null) => {
    setMessageState(value);
    if (value) setErrorState(null);
  }, []);

  const clear = useCallback(() => {
    setErrorState(null);
    setMessageState(null);
  }, []);

  const value = useMemo(
    () => ({ error, message, setError, setMessage, clear }),
    [error, message, setError, setMessage, clear],
  );

  return (
    <FlashContext.Provider value={value}>{children}</FlashContext.Provider>
  );
}

export function useFlash(): FlashContextValue {
  const ctx = useContext(FlashContext);
  if (!ctx) {
    throw new Error("useFlash must be used inside FlashProvider");
  }
  return ctx;
}

/** Sticky error / success strip at the top of main content. */
export function FlashBanner() {
  const { error, message, clear } = useFlash();
  if (!error && !message) return null;

  return (
    <div
      className={error ? "flash-banner flash-error" : "flash-banner flash-ok"}
      role={error ? "alert" : "status"}
    >
      <pre className="flash-banner-text">{error ?? message}</pre>
      <button
        type="button"
        className="flash-banner-dismiss"
        aria-label="Dismiss"
        onClick={clear}
      >
        ✕
      </button>
    </div>
  );
}
