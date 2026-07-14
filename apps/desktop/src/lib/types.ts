/** Types mirrored from apps/desktop/src-tauri/src/dto.rs (camelCase serde). */

export type NetworkName = "mainnet" | "testnet" | "signet" | "regtest";
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
  fallback?: FallbackPolicy | null;
}

export interface PolicyConfig {
  version: number;
  network: NetworkName;
  script_type: ScriptTypeName;
  keys: KeyConfig[];
  policy: PolicyExpression;
}

export interface WalletDto {
  id: string;
  name: string;
  network: NetworkName;
  createdAt: number;
  updatedAt: number;
}

export interface WalletSummaryDto {
  id: string;
  name: string;
  network: NetworkName;
  vaultCount: number;
  createdAt: number;
}

export interface VaultDto {
  id: string;
  walletId: string;
  name: string;
  policy: PolicyConfig;
  descriptor: string;
  scriptType: ScriptTypeName;
  createdAt: number;
}

export interface VaultSummaryDto {
  id: string;
  walletId: string;
  name: string;
  scriptType: ScriptTypeName;
  createdAt: number;
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

export interface CompileVaultResponse {
  descriptor: string;
  policyString: string;
}

export interface CreateWalletRequest {
  name: string;
  network: NetworkName;
}

export interface CreateVaultRequest {
  walletId: string;
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
  vaultId: string;
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
