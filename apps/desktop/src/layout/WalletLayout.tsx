import { WalletProvider } from "../wallet/WalletContext";
import { WalletShell } from "./WalletShell";

export function WalletLayout() {
  return (
    <WalletProvider kind="wallet" listPath="/wallets">
      <WalletShell />
    </WalletProvider>
  );
}
