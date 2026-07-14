import { invoke } from "@tauri-apps/api/core";
import type {
  AddressDto,
  BalanceDto,
  CompileVaultResponse,
  CreatePsbtRequest,
  CreateVaultRequest,
  CreateWalletRequest,
  FinalizedTxDto,
  NetworkName,
  PolicyConfig,
  PsbtDto,
  ServerPresetDto,
  SignPsbtRequest,
  SignedPsbtDto,
  SparrowExportDto,
  SyncResultDto,
  VaultDto,
  VaultSummaryDto,
  WalletDto,
  WalletSummaryDto,
  BroadcastTxRequest,
  CombinePsbtRequest,
  HwDeviceDto,
  HwGetXpubRequest,
  HwSignPsbtRequest,
  HwXpubDto,
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

export async function syncVault(
  vaultId: string,
  esploraUrl?: string,
): Promise<SyncResultDto> {
  return invoke("sync_vault", { vaultId, esploraUrl: esploraUrl ?? null });
}

export async function createPsbt(
  request: CreatePsbtRequest,
): Promise<PsbtDto> {
  return invoke("create_psbt", { request });
}

export async function signPsbtSoftware(
  request: SignPsbtRequest,
): Promise<SignedPsbtDto> {
  return invoke("sign_psbt_software", { request });
}

export async function listHwDevices(
  hwiPath?: string | null,
): Promise<HwDeviceDto[]> {
  return invoke("list_hw_devices", { hwiPath: hwiPath ?? null });
}

export async function hwGetXpub(
  request: HwGetXpubRequest,
): Promise<HwXpubDto> {
  return invoke("hw_get_xpub", { request });
}

export async function hwSignPsbt(
  request: HwSignPsbtRequest,
): Promise<SignedPsbtDto> {
  return invoke("hw_sign_psbt", { request });
}

export async function combinePsbts(
  request: CombinePsbtRequest,
): Promise<PsbtDto> {
  return invoke("combine_psbts", { request });
}

export async function finalizePsbt(
  psbtBase64: string,
): Promise<FinalizedTxDto> {
  return invoke("finalize_psbt_cmd", { psbtBase64 });
}

export async function broadcastPsbt(
  request: BroadcastTxRequest,
): Promise<string> {
  return invoke("broadcast_psbt_cmd", { request });
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

export async function appVersion(): Promise<string> {
  return invoke("app_version");
}

export function formatError(err: unknown): string {
  let message: string;
  if (err instanceof Error) message = err.message;
  else if (typeof err === "string") message = err;
  else message = String(err);
  return message.replace(/\b[xt]prv[1-9A-HJ-NP-Za-km-z]+/gi, "[redacted-private-key]");
}
