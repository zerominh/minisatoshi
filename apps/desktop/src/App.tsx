import { Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "./layout/AppLayout";
import { NewVaultPage } from "./pages/NewVaultPage";
import { ImportVaultPage } from "./pages/ImportVaultPage";
import { ReceivePage } from "./pages/ReceivePage";
import { SendPage } from "./pages/SendPage";
import { SettingsPage } from "./pages/SettingsPage";
import { TransactionsPage } from "./pages/TransactionsPage";
import { VaultDetailPage } from "./pages/VaultDetailPage";
import { VaultsPage } from "./pages/VaultsPage";
import { WalletsPage } from "./pages/WalletsPage";

function App() {
  return (
    <Routes>
      <Route element={<AppLayout />}>
        <Route index element={<Navigate to="/wallets" replace />} />
        <Route path="wallets" element={<WalletsPage />} />
        <Route path="vaults" element={<VaultsPage />} />
        <Route path="vaults/new" element={<NewVaultPage />} />
        <Route path="vaults/import" element={<ImportVaultPage />} />
        <Route path="vaults/:id" element={<VaultDetailPage />} />
        <Route path="vaults/:id/receive" element={<ReceivePage />} />
        <Route path="vaults/:id/send" element={<SendPage />} />
        <Route path="transactions" element={<TransactionsPage />} />
        <Route path="settings" element={<SettingsPage />} />
        <Route path="*" element={<Navigate to="/wallets" replace />} />
      </Route>
    </Routes>
  );
}

export default App;
