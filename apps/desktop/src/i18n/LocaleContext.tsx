import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { getLocale, setLocale as persistLocale } from "../lib/settings";
import { en, type Locale, type MessageKey, type Messages } from "./en";
import { vi } from "./vi";

const catalogs: Record<Locale, Messages> = {
  en,
  vi,
};

type Vars = Record<string, string | number>;

function interpolate(template: string, vars?: Vars): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (_, key: string) =>
    vars[key] != null ? String(vars[key]) : `{${key}}`,
  );
}

type LocaleContextValue = {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (key: MessageKey, vars?: Vars) => string;
};

const LocaleContext = createContext<LocaleContextValue | null>(null);

export function LocaleProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => getLocale());

  const setLocale = useCallback((next: Locale) => {
    persistLocale(next);
    setLocaleState(next);
    if (typeof document !== "undefined") {
      document.documentElement.lang = next === "vi" ? "vi" : "en";
    }
  }, []);

  const t = useCallback(
    (key: MessageKey, vars?: Vars) => {
      const catalog = catalogs[locale] ?? catalogs.en;
      const raw = catalog[key] ?? catalogs.en[key] ?? key;
      return interpolate(raw, vars);
    },
    [locale],
  );

  const value = useMemo(
    () => ({ locale, setLocale, t }),
    [locale, setLocale, t],
  );

  return (
    <LocaleContext.Provider value={value}>{children}</LocaleContext.Provider>
  );
}

export function useLocale(): LocaleContextValue {
  const ctx = useContext(LocaleContext);
  if (!ctx) {
    throw new Error("useLocale must be used inside LocaleProvider");
  }
  return ctx;
}

export function useT() {
  return useLocale().t;
}
