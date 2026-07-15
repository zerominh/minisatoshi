import { Navigate, Route, Routes, useParams } from "react-router-dom";
import { AppLayout } from "./layout/AppLayout";
import { HotWalletLayout } from "./layout/HotWalletLayout";
import { WalletLayout } from "./layout/WalletLayout";
import { HotWalletsPage } from "./pages/HotWalletsPage";
import { NewWalletPage } from "./pages/NewWalletPage";
import { ImportWalletPage } from "./pages/ImportWalletPage";
import { ReceivePage } from "./pages/ReceivePage";
import { SendPage } from "./pages/SendPage";
import { SettingsPage } from "./pages/SettingsPage";
import { ShareWalletPage } from "./pages/ShareWalletPage";
import { SignPsbtPage } from "./pages/SignPsbtPage";
import { TransactionsPage } from "./pages/TransactionsPage";
import { WalletAddressesPage } from "./pages/WalletAddressesPage";
import { WalletSettingsPage } from "./pages/WalletSettingsPage";
import { WalletTransactionsPage } from "./pages/WalletTransactionsPage";
import { WalletUtxosPage } from "./pages/WalletUtxosPage";
import { WalletsPage } from "./pages/WalletsPage";
import { WorkspacesPage } from "./pages/WorkspacesPage";

/** Old `/vaults/:id/*` bookmarks (pre-rename) → new `/wallets/:id/*`. */
function LegacyVaultDetailRedirect() {
  const { id = "", "*": rest } = useParams();
  const suffix = rest ? `/${rest}` : "";
  return <Navigate to={`/wallets/${id}${suffix}`} replace />;
}

function App() {
  return (
    <Routes>
      <Route element={<AppLayout />}>
        <Route index element={<Navigate to="/workspaces" replace />} />
        <Route path="workspaces" element={<WorkspacesPage />} />
        <Route path="wallets" element={<WalletsPage />} />
        <Route path="wallets/new" element={<NewWalletPage />} />
        <Route path="wallets/import" element={<ImportWalletPage />} />
        <Route path="wallets/:id" element={<WalletLayout />}>
          <Route index element={<Navigate to="transactions" replace />} />
          <Route path="transactions" element={<WalletTransactionsPage />} />
          <Route path="send" element={<SendPage />} />
          <Route path="sign-psbt" element={<SignPsbtPage />} />
          <Route path="receive" element={<ReceivePage />} />
          <Route path="addresses" element={<WalletAddressesPage />} />
          <Route path="utxos" element={<WalletUtxosPage />} />
          <Route path="settings" element={<WalletSettingsPage />} />
        </Route>
        <Route path="wallets/:id/share" element={<ShareWalletPage />} />
        <Route path="hot-wallets" element={<HotWalletsPage />} />
        <Route path="hot-wallets/:id" element={<HotWalletLayout />}>
          <Route index element={<Navigate to="transactions" replace />} />
          <Route path="transactions" element={<WalletTransactionsPage />} />
          <Route path="send" element={<SendPage />} />
          <Route path="sign-psbt" element={<SignPsbtPage />} />
          <Route path="receive" element={<ReceivePage />} />
          <Route path="addresses" element={<WalletAddressesPage />} />
          <Route path="utxos" element={<WalletUtxosPage />} />
          <Route path="settings" element={<WalletSettingsPage />} />
        </Route>
        <Route path="hot-wallets/:id/share" element={<ShareWalletPage />} />
        <Route path="transactions" element={<TransactionsPage />} />
        <Route path="settings" element={<SettingsPage />} />

        {/* Legacy routes (pre-rename): container used to live at /vaults, spendable at /vaults/:id. */}
        <Route path="vaults" element={<Navigate to="/wallets" replace />} />
        <Route path="vaults/new" element={<Navigate to="/wallets/new" replace />} />
        <Route
          path="vaults/import"
          element={<Navigate to="/wallets/import" replace />}
        />
        <Route path="vaults/:id/*" element={<LegacyVaultDetailRedirect />} />

        <Route path="*" element={<Navigate to="/workspaces" replace />} />
      </Route>
    </Routes>
  );
}

export default App;
