import { Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "./layout/AppLayout";
import { HotWalletLayout } from "./layout/HotWalletLayout";
import { VaultLayout } from "./layout/VaultLayout";
import { HotWalletsPage } from "./pages/HotWalletsPage";
import { NewVaultPage } from "./pages/NewVaultPage";
import { ImportVaultPage } from "./pages/ImportVaultPage";
import { ReceivePage } from "./pages/ReceivePage";
import { SendPage } from "./pages/SendPage";
import { SettingsPage } from "./pages/SettingsPage";
import { ShareVaultPage } from "./pages/ShareVaultPage";
import { SignPsbtPage } from "./pages/SignPsbtPage";
import { TransactionsPage } from "./pages/TransactionsPage";
import { VaultAddressesPage } from "./pages/VaultAddressesPage";
import { VaultSettingsPage } from "./pages/VaultSettingsPage";
import { VaultTransactionsPage } from "./pages/VaultTransactionsPage";
import { VaultUtxosPage } from "./pages/VaultUtxosPage";
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
        <Route path="vaults/:id" element={<VaultLayout />}>
          <Route index element={<Navigate to="transactions" replace />} />
          <Route path="transactions" element={<VaultTransactionsPage />} />
          <Route path="send" element={<SendPage />} />
          <Route path="sign-psbt" element={<SignPsbtPage />} />
          <Route path="receive" element={<ReceivePage />} />
          <Route path="addresses" element={<VaultAddressesPage />} />
          <Route path="utxos" element={<VaultUtxosPage />} />
          <Route path="settings" element={<VaultSettingsPage />} />
        </Route>
        <Route path="vaults/:id/share" element={<ShareVaultPage />} />
        <Route path="hot-wallets" element={<HotWalletsPage />} />
        <Route path="hot-wallets/:id" element={<HotWalletLayout />}>
          <Route index element={<Navigate to="transactions" replace />} />
          <Route path="transactions" element={<VaultTransactionsPage />} />
          <Route path="send" element={<SendPage />} />
          <Route path="sign-psbt" element={<SignPsbtPage />} />
          <Route path="receive" element={<ReceivePage />} />
          <Route path="addresses" element={<VaultAddressesPage />} />
          <Route path="utxos" element={<VaultUtxosPage />} />
          <Route path="settings" element={<VaultSettingsPage />} />
        </Route>
        <Route path="hot-wallets/:id/share" element={<ShareVaultPage />} />
        <Route path="transactions" element={<TransactionsPage />} />
        <Route path="settings" element={<SettingsPage />} />
        <Route path="*" element={<Navigate to="/wallets" replace />} />
      </Route>
    </Routes>
  );
}

export default App;
