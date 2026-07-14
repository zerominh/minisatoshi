import { NavLink, Outlet } from "react-router-dom";

const links = [
  { to: "/wallets", label: "Wallets" },
  { to: "/vaults", label: "Vaults" },
  { to: "/hot-wallets", label: "Hot wallets" },
  { to: "/transactions", label: "Transactions" },
  { to: "/settings", label: "Settings" },
];

export function AppLayout() {
  return (
    <div className="app">
      <aside className="sidebar">
        <h1>Minisatoshi</h1>
        <p className="sidebar-tag">Bitcoin Vault Engine</p>
        <nav>
          {links.map((link) => (
            <NavLink
              key={link.to}
              to={link.to}
              className={({ isActive }) => (isActive ? "nav-active" : undefined)}
            >
              {link.label}
            </NavLink>
          ))}
        </nav>
      </aside>
      <main className="content">
        <Outlet />
      </main>
    </div>
  );
}
