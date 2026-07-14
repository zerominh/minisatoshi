import { invoke } from "@tauri-apps/api/core";
import type {
  AddressDto,
  BalanceDto,
  CompileVaultResponse,
  CreatePsbtRequest,
  CreateVaultRequest,
  CreateWalletRequest,
  NetworkName,
  PolicyConfig,
  PsbtDto,
  ServerPresetDto,
  SparrowExportDto,
  VaultDto,
  VaultSummaryDto,
  WalletDto,
  WalletSummaryDto,
} from "./types";

export async function compileVaultDescriptor(
  config: PolicyConfig,
): Promise<CompileVaultResponse> {
  return invoke("compile_vault_descriptor", { config });
}

export async function createWallet(
  request: CreateWalletRequest,
): Promise<WalletDto> {
  return invoke("create_wallet", { request });
}

export async function listWallets(): Promise<WalletSummaryDto[]> {
  return invoke("list_wallets");
}

export async function createVault(
  request: CreateVaultRequest,
): Promise<VaultDto> {
  return invoke("create_vault", { request });
}

export async function listVaults(
  walletId: string,
): Promise<VaultSummaryDto[]> {
  return invoke("list_vaults", { walletId });
}

export async function getVault(vaultId: string): Promise<VaultDto> {
  return invoke("get_vault", { vaultId });
}

export async function newReceiveAddress(
  vaultId: string,
): Promise<AddressDto> {
  return invoke("new_receive_address", { vaultId });
}

export async function getBalance(
  vaultId: string,
  esploraUrl?: string,
): Promise<BalanceDto> {
  return invoke("get_balance", { vaultId, esploraUrl: esploraUrl ?? null });
}

export async function createPsbt(
  request: CreatePsbtRequest,
): Promise<PsbtDto> {
  return invoke("create_psbt", { request });
}

export async function exportSparrowWallet(
  vaultId: string,
): Promise<SparrowExportDto> {
  return invoke("export_sparrow_wallet", { vaultId });
}

export async function listServerPresets(
  network: NetworkName,
): Promise<ServerPresetDto[]> {
  return invoke("list_server_presets", { network });
}
