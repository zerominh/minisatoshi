/** Types mirrored from apps/desktop/src-tauri/src/dto.rs (camelCase serde). */

/** `testnet` = classic testnet3; `testnet4` = Bitcoin testnet4. */
export type NetworkName = "mainnet" | "testnet" | "testnet4" | "signet" | "regtest";
export type ScriptTypeName = "taproot" | "wsh";
export type KeyRole = "investor" | "manager" | "recovery" | "cosigner" | "other";

export interface KeyConfig {
  id: string;
  role: KeyRole;
  xpub: string;
  fingerprint: string;
  origin_path?: string;
}

export interface FallbackPolicy {
  after: string;
  allow: string;
}

export interface PolicyExpression {
  primary: string;
  /** @deprecated prefer `fallbacks` */
  fallback?: FallbackPolicy | null;
  fallbacks?: FallbackPolicy[];
}

export interface PolicyConfig {
  version: number;
  network: NetworkName;
  script_type: ScriptTypeName;
  keys: KeyConfig[];
  policy: PolicyExpression;
}

/** Container + network (ex-`Wallet`). */
export interface WorkspaceDto {
  id: string;
  name: string;
  network: NetworkName;
  createdAt: number;
  updatedAt: number;
}

export interface WorkspaceSummaryDto {
  id: string;
  name: string;
  network: NetworkName;
  walletCount: number;
  createdAt: number;
}

/** Spendable descriptor / balance / send-receive unit (ex-`Vault`). */
export interface WalletDto {
  id: string;
  workspaceId: string;
  name: string;
  policy: PolicyConfig;
  descriptor: string;
  scriptType: ScriptTypeName;
  createdAt: number;
  /** Private keys are never persisted. */
  watchOnly: boolean;
}

export interface WalletSummaryDto {
  id: string;
  workspaceId: string;
  name: string;
  scriptType: ScriptTypeName;
  createdAt: number;
  /** Private keys are never persisted. */
  watchOnly: boolean;
}

export interface AddressDto {
  address: string;
  index: number;
  isChange: boolean;
}

export interface BalanceDto {
  confirmedSats: number;
  unconfirmedSats: number;
}

export interface CompileWalletResponse {
  descriptor: string;
  policyString: string;
}

/** Create a new container + network (ex-`Wallet`). */
export interface CreateWorkspaceRequest {
  name: string;
  network: NetworkName;
}

/** Create a new spendable wallet (ex-`Vault`) inside a workspace. */
export interface CreateWalletRequest {
  workspaceId: string;
  name: string;
  policy: PolicyConfig;
}

export interface UtxoDto {
  txid: string;
  vout: number;
  valueSats: number;
  address: string;
  confirmed: boolean;
  blockHeight?: number | null;
  derivationIndex: number;
  isChange: boolean;
}

export interface TxSummaryDto {
  txid: string;
  amountSats: number;
  confirmed: boolean;
  blockHeight?: number | null;
  /** Unix seconds (block time) when available from sync. */
  blockTime?: number | null;
}

export interface SyncResultDto {
  balance: BalanceDto;
  utxos: UtxoDto[];
  history: TxSummaryDto[];
}

export interface PsbtRecipientDto {
  address: string;
  amountSats: number;
}

export interface CreatePsbtRequest {
  walletId: string;
  recipients: PsbtRecipientDto[];
  feeRateSatPerVb: number;
  utxos: UtxoDto[];
  inputSequence?: number | null;
  changeIndex?: number | null;
}

export interface PsbtDto {
  base64: string;
  inputCount: number;
  outputCount: number;
}

export interface SignPsbtRequest {
  psbtBase64: string;
  secretKey: string;
  network: NetworkName;
  allowMainnetHotKeys?: boolean;
}

export interface HwDeviceDto {
  id: string;
  fingerprint: string;
  deviceType: string;
  model: string;
  path?: string | null;
  needsPin: boolean;
  needsPassphrase: boolean;
  error?: string | null;
}

export interface HwGetXpubRequest {
  fingerprint: string;
  derivationPath: string;
  hwiPath?: string | null;
  network?: NetworkName | null;
}

export interface HwXpubDto {
  fingerprint: string;
  derivationPath: string;
  xpub: string;
}

export interface HwSignPsbtRequest {
  fingerprint: string;
  psbtBase64: string;
  hwiPath?: string | null;
  network?: NetworkName | null;
  walletId?: string | null;
}

export interface HwStatusDto {
  available: boolean;
  path?: string | null;
  version?: string | null;
  source?: string | null;
  pinnedVersion: string;
  message?: string | null;
}

export interface Bip388PolicyDto {
  name: string;
  policy: string;
  keys: string[];
}

export interface VendorRegistrationDto {
  deviceType: string;
  title: string;
  body: string;
  instructions: string[];
}

export interface RegistrationPackageDto {
  vaultName: string;
  network: string;
  descriptor: string;
  bip388: Bip388PolicyDto;
  coldcardSdText: string;
  ledgerHmac?: string | null;
  vendors: VendorRegistrationDto[];
  hwiRegisterpolicySupported: boolean;
}

export interface HwRegisterRequest {
  walletId: string;
  fingerprint: string;
  hwiPath?: string | null;
}

export interface HwRegisterResultDto {
  ok: boolean;
  message: string;
  hmac?: string | null;
  package: RegistrationPackageDto;
  cosignerHints: string[];
}

export interface LedgerRegistrationStatusDto {
  registered: boolean;
  stale: boolean;
  staleReason?: string | null;
  fingerprint: string;
  pythonAvailable: boolean;
  ledgerCliReady: boolean;
  runtimeSource?: string | null;
  installedVersion?: string | null;
  appName?: string | null;
  appVersion?: string | null;
  expectedAppName: string;
  deviceConnected: boolean;
  warnings: string[];
  ready: boolean;
}

export interface LedgerRuntimeStatusDto {
  available: boolean;
  pythonPath?: string | null;
  scriptPath?: string | null;
  pinnedVersion: string;
  installedVersion?: string | null;
  source?: string | null;
  scriptReady: boolean;
  message?: string | null;
}

export interface SpendingPathDto {
  id: string;
  label: string;
  requiredKeys: string[];
  timelockBlocks?: number | null;
  suggestedSequence?: number | null;
  kind: "primary" | "fallback";
}

export type KeySignStatus = "signed" | "missing" | "unused";

export interface KeyStatusDto {
  id: string;
  fingerprint: string;
  role: string;
  status: KeySignStatus;
}

export interface PathStatusDto {
  path: SpendingPathDto;
  satisfied: boolean;
  missingKeys: string[];
  presentKeys: string[];
}

export interface SigningStatusDto {
  summary: string;
  keys: KeyStatusDto[];
  paths: PathStatusDto[];
  signedFingerprints: string[];
  signedInputCount: number;
  totalInputs: number;
  activePathId?: string | null;
}

export interface AnalyzePsbtRequest {
  walletId: string;
  psbtBase64: string;
  activePathId?: string | null;
}

export interface ImportDescriptorRequest {
  workspaceId: string;
  name: string;
  descriptor: string;
  policy?: PolicyConfig | null;
}

export interface ImportWalletBackupRequest {
  workspaceId: string;
  payload: string;
  name?: string | null;
}

export interface WalletBackupDto {
  formatVersion: string;
  name: string;
  network: NetworkName;
  descriptor: string;
  scriptType: ScriptTypeName;
  policy?: PolicyConfig | null;
  createdAt: number;
  json: string;
  descriptorTxt: string;
}

export interface BsmsExportDto {
  text: string;
  firstAddress: string;
}

export interface HotKeystoreStatusDto {
  exists: boolean;
  unlocked: boolean;
  path: string;
}

export interface HotWalletSummaryDto {
  id: string;
  name: string;
  network: NetworkName;
  fingerprint: string;
  originPath: string;
  xpub: string;
  linkedWorkspaceId?: string | null;
  linkedWalletId?: string | null;
  createdAt: number;
}

export interface CreateHotKeystoreRequest {
  masterPassword: string;
}

export interface UnlockHotKeystoreRequest {
  masterPassword: string;
}

export interface ImportHotWalletRequest {
  name: string;
  mnemonicOrJson: string;
  bip39Passphrase?: string;
  network: NetworkName;
  /** Optional; empty → auto storage parent for the network. */
  workspaceId?: string;
  accountPath?: string | null;
  createNestedWallet?: boolean;
}

export interface ImportHotWalletResultDto {
  hotWallet: HotWalletSummaryDto;
  wallet?: WalletDto | null;
}

export interface SignPsbtHotRequest {
  psbtBase64: string;
  hotWalletId: string;
  network: NetworkName;
  allowMainnetHotKeys?: boolean;
}

export interface SignedPsbtDto {
  base64: string;
  inputCount: number;
  outputCount: number;
  signedInputs: number;
  totalInputs: number;
}

export interface CombinePsbtRequest {
  parts: string[];
}

export interface TxOutputDto {
  address?: string | null;
  amountSats: number;
}

export interface FinalizedTxDto {
  hex: string;
  txid: string;
  fullySigned: boolean;
  outputs: TxOutputDto[];
}

export interface BroadcastTxRequest {
  walletId: string;
  psbtBase64?: string | null;
  txHex?: string | null;
  esploraUrl?: string | null;
}

export interface SparrowExportDto {
  name: string;
  descriptor: string;
  network: NetworkName;
  importInstructions: string;
}

export interface ServerPresetDto {
  label: string;
  backend: string;
  url: string;
  network: NetworkName;
}
