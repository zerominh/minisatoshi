import { VaultProvider } from "../vault/VaultContext";
import { WalletShell } from "./WalletShell";

export function VaultLayout() {
  return (
    <VaultProvider kind="vault" listPath="/vaults">
      <WalletShell />
    </VaultProvider>
  );
}
