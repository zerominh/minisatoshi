import { NavLink, Outlet } from "react-router-dom";
import { FlashBanner, FlashProvider } from "../flash/FlashContext";
import { useLocale, useT } from "../i18n/LocaleContext";
import type { Locale } from "../i18n/en";
import { LOCALES } from "../i18n/en";

const links = [
  { to: "/wallets", labelKey: "nav.wallets" as const },
  { to: "/vaults", labelKey: "nav.vaults" as const },
  { to: "/hot-wallets", labelKey: "nav.hotWallets" as const },
  { to: "/transactions", labelKey: "nav.transactions" as const },
  { to: "/settings", labelKey: "nav.settings" as const },
];

export function AppLayout() {
  const t = useT();
  const { locale, setLocale } = useLocale();

  return (
    <FlashProvider>
      <div className="app">
        <aside className="sidebar">
          <h1>Minisatoshi</h1>
          <p className="sidebar-tag">{t("app.tagline")}</p>
          <nav>
            {links.map((link) => (
              <NavLink
                key={link.to}
                to={link.to}
                className={({ isActive }) =>
                  isActive ? "nav-active" : undefined
                }
              >
                {t(link.labelKey)}
              </NavLink>
            ))}
          </nav>
          <label className="sidebar-lang">
            <span className="muted">{t("settings.language")}</span>
            <select
              value={locale}
              onChange={(e) => setLocale(e.target.value as Locale)}
              aria-label={t("settings.language")}
            >
              {LOCALES.map((item) => (
                <option key={item.id} value={item.id}>
                  {t(item.labelKey)}
                </option>
              ))}
            </select>
          </label>
        </aside>
        <main className="content">
          <FlashBanner />
          <Outlet />
        </main>
      </div>
    </FlashProvider>
  );
}
