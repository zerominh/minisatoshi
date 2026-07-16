import { invoke } from "@tauri-apps/api/core";
import type {
  AddressDto,
  BalanceDto,
  CompileWalletResponse,
  CreatePsbtRequest,
  CreateWalletRequest,
  CreateWorkspaceRequest,
  FinalizedTxDto,
  NetworkName,
  PolicyConfig,
  PsbtDto,
  ServerPresetDto,
  SignPsbtRequest,
  SignedPsbtDto,
  SparrowExportDto,
  SyncResultDto,
  WalletDto,
  WalletSummaryDto,
  WorkspaceDto,
  WorkspaceSummaryDto,
  BroadcastTxRequest,
  CombinePsbtRequest,
  HwDeviceDto,
  HwGetXpubRequest,
  HwSignPsbtRequest,
  HwStatusDto,
  HwRegisterRequest,
  HwRegisterResultDto,
  HwXpubDto,
  RegistrationPackageDto,
  AnalyzePsbtRequest,
  SigningStatusDto,
  SpendingPathDto,
  ImportDescriptorRequest,
  ImportWalletBackupRequest,
  WalletBackupDto,
  BsmsExportDto,
  SignPsbtHotRequest,
  HotKeystoreStatusDto,
  CreateHotKeystoreRequest,
  UnlockHotKeystoreRequest,
  HotWalletSummaryDto,
  ImportHotWalletRequest,
  ImportHotWalletResultDto,
} from "./types";

export async function compileWalletDescriptor(
  config: PolicyConfig,
): Promise<CompileWalletResponse> {
  return invoke("compile_wallet_descriptor", { config });
}

export async function createWorkspace(
  request: CreateWorkspaceRequest,
): Promise<WorkspaceDto> {
  return invoke("create_workspace", { request });
}

export async function listWorkspaces(): Promise<WorkspaceSummaryDto[]> {
  return invoke("list_workspaces");
}

export async function deleteWorkspace(workspaceId: string): Promise<void> {
  return invoke("delete_workspace", { workspaceId });
}

export async function renameWorkspace(
  workspaceId: string,
  name: string,
): Promise<WorkspaceDto> {
  return invoke("rename_workspace", { workspaceId, name });
}

export async function deleteWallet(walletId: string): Promise<void> {
  return invoke("delete_wallet", { walletId });
}

export async function renameWallet(
  walletId: string,
  name: string,
): Promise<WalletDto> {
  return invoke("rename_wallet", { walletId, name });
}

export async function createWallet(
  request: CreateWalletRequest,
): Promise<WalletDto> {
  return invoke("create_wallet", { request });
}

export async function importDescriptor(
  request: ImportDescriptorRequest,
): Promise<WalletDto> {
  return invoke("import_descriptor", { request });
}

export async function importWalletBackup(
  request: ImportWalletBackupRequest,
): Promise<WalletDto> {
  return invoke("import_wallet_backup", { request });
}

export async function exportWalletBackup(
  walletId: string,
): Promise<WalletBackupDto> {
  return invoke("export_wallet_backup", { walletId });
}

export async function exportBsms(walletId: string): Promise<BsmsExportDto> {
  return invoke("export_bsms", { walletId });
}

export async function listWallets(
  workspaceId: string,
): Promise<WalletSummaryDto[]> {
  return invoke("list_wallets", { workspaceId });
}

export async function getWallet(walletId: string): Promise<WalletDto> {
  return invoke("get_wallet", { walletId });
}

export async function newReceiveAddress(
  walletId: string,
): Promise<AddressDto> {
  return invoke("new_receive_address", { walletId });
}

export async function listAddresses(
  walletId: string,
): Promise<AddressDto[]> {
  return invoke("list_addresses", { walletId });
}

export async function getBalance(
  walletId: string,
  esploraUrl?: string,
): Promise<BalanceDto> {
  return invoke("get_balance", { walletId, esploraUrl: esploraUrl ?? null });
}

export async function syncWallet(
  walletId: string,
  esploraUrl?: string,
): Promise<SyncResultDto> {
  return invoke("sync_wallet", { walletId, esploraUrl: esploraUrl ?? null });
}

export async function createPsbt(
  request: CreatePsbtRequest,
): Promise<PsbtDto> {
  return invoke("create_psbt", { request });
}

/** Parse / normalize a base64 PSBT for the Sign step (cosigner / air-gap). */
export async function importPsbt(psbtBase64: string): Promise<PsbtDto> {
  return invoke("import_psbt", { psbtBase64 });
}

export async function signPsbtSoftware(
  request: SignPsbtRequest,
): Promise<SignedPsbtDto> {
  return invoke("sign_psbt_software", { request });
}

export async function signPsbtHot(
  request: SignPsbtHotRequest,
): Promise<SignedPsbtDto> {
  return invoke("sign_psbt_hot", { request });
}

export async function hotKeystoreStatus(): Promise<HotKeystoreStatusDto> {
  return invoke("hot_keystore_status");
}

export async function createHotKeystore(
  request: CreateHotKeystoreRequest,
): Promise<HotKeystoreStatusDto> {
  return invoke("create_hot_keystore", { request });
}

export async function unlockHotKeystore(
  request: UnlockHotKeystoreRequest,
): Promise<HotKeystoreStatusDto> {
  return invoke("unlock_hot_keystore", { request });
}

export async function lockHotKeystore(): Promise<HotKeystoreStatusDto> {
  return invoke("lock_hot_keystore");
}

export async function listHotWallets(): Promise<HotWalletSummaryDto[]> {
  return invoke("list_hot_wallets");
}

export async function importHotWallet(
  request: ImportHotWalletRequest,
): Promise<ImportHotWalletResultDto> {
  return invoke("import_hot_wallet", { request });
}

/** Resolve (or create) the nested wallet for a hot wallet — opens Transactions/Send/Receive. */
export async function openHotWallet(
  hotWalletId: string,
  workspaceId?: string | null,
): Promise<WalletDto> {
  return invoke("open_hot_wallet", {
    hotWalletId,
    workspaceId: workspaceId ?? null,
  });
}

export async function renameHotWallet(
  hotWalletId: string,
  name: string,
): Promise<HotWalletSummaryDto> {
  return invoke("rename_hot_wallet", { hotWalletId, name });
}

export async function deleteHotWallet(hotWalletId: string): Promise<void> {
  return invoke("delete_hot_wallet", { hotWalletId });
}

export async function listHwDevices(
  hwiPath?: string | null,
  network?: NetworkName | null,
): Promise<HwDeviceDto[]> {
  return invoke("list_hw_devices", {
    hwiPath: hwiPath ?? null,
    network: network ?? null,
  });
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

export async function getHwiStatus(
  hwiPath?: string | null,
): Promise<HwStatusDto> {
  return invoke("get_hwi_status", { hwiPath: hwiPath ?? null });
}

export async function ensureHwiInstalled(
  hwiPath?: string | null,
): Promise<HwStatusDto> {
  return invoke("ensure_hwi_installed", { hwiPath: hwiPath ?? null });
}

export async function prepareHwRegistration(
  walletId: string,
): Promise<RegistrationPackageDto> {
  return invoke("prepare_hw_registration", { walletId });
}

export async function hwRegisterWallet(
  request: HwRegisterRequest,
): Promise<HwRegisterResultDto> {
  return invoke("hw_register_wallet", { request });
}

export async function listSpendingPaths(
  walletId: string,
): Promise<SpendingPathDto[]> {
  return invoke("list_spending_paths", { walletId });
}

export async function analyzePsbtStatus(
  request: AnalyzePsbtRequest,
): Promise<SigningStatusDto> {
  return invoke("analyze_psbt_status", { request });
}

export async function combinePsbts(
  request: CombinePsbtRequest,
): Promise<PsbtDto> {
  return invoke("combine_psbts", { request });
}

export async function finalizePsbt(
  psbtBase64: string,
  walletId?: string | null,
): Promise<FinalizedTxDto> {
  return invoke("finalize_psbt_cmd", {
    psbtBase64,
    walletId: walletId ?? null,
  });
}

export async function broadcastPsbt(
  request: BroadcastTxRequest,
): Promise<string> {
  return invoke("broadcast_psbt_cmd", { request });
}

export async function exportSparrowWallet(
  walletId: string,
): Promise<SparrowExportDto> {
  return invoke("export_sparrow_wallet", { walletId });
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
