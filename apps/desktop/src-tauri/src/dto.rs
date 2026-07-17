//! Serializable DTOs shared across the Tauri IPC boundary.

use policy_engine::{NetworkName, PolicyConfig, ScriptTypeName};
use serde::{Deserialize, Serialize};
use wallet_core::{Wallet, WalletSummary, Workspace, WorkspaceSummary};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenTextFileDto {
    pub path: String,
    pub contents: String,
}

/// Container + network (ex-`Wallet`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDto {
    pub id: String,
    pub name: String,
    pub network: NetworkName,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummaryDto {
    pub id: String,
    pub name: String,
    pub network: NetworkName,
    pub wallet_count: usize,
    pub created_at: i64,
}

/// Spendable descriptor / balance / send-receive unit (ex-`Vault`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletDto {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub policy: PolicyConfig,
    pub descriptor: String,
    pub script_type: ScriptTypeName,
    pub created_at: i64,
    /// Always true: private keys are never persisted in Minisatoshi.
    pub watch_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletSummaryDto {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub script_type: ScriptTypeName,
    pub created_at: i64,
    /// Always true: private keys are never persisted in Minisatoshi.
    pub watch_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressDto {
    pub address: String,
    pub index: u32,
    pub is_change: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceDto {
    pub confirmed_sats: u64,
    pub unconfirmed_sats: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileWalletResponse {
    pub descriptor: String,
    pub policy_string: String,
}

/// Create a new container + network (ex-`Wallet`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub network: NetworkName,
}

/// Create a new spendable wallet (ex-`Vault`) inside a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWalletRequest {
    pub workspace_id: String,
    pub name: String,
    pub policy: PolicyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UtxoDto {
    pub txid: String,
    pub vout: u32,
    pub value_sats: u64,
    pub address: String,
    pub confirmed: bool,
    pub block_height: Option<u32>,
    pub derivation_index: u32,
    pub is_change: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PsbtRecipientDto {
    pub address: String,
    pub amount_sats: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePsbtRequest {
    pub wallet_id: String,
    pub recipients: Vec<PsbtRecipientDto>,
    pub fee_rate_sat_per_vb: u64,
    pub utxos: Vec<UtxoDto>,
    pub input_sequence: Option<u32>,
    pub change_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PsbtDto {
    pub base64: String,
    pub input_count: usize,
    pub output_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignPsbtRequest {
    pub psbt_base64: String,
    pub secret_key: String,
    pub network: NetworkName,
    #[serde(default)]
    pub allow_mainnet_hot_keys: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HwDeviceDto {
    pub id: String,
    pub fingerprint: String,
    pub device_type: String,
    pub model: String,
    pub path: Option<String>,
    pub needs_pin: bool,
    pub needs_passphrase: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HwGetXpubRequest {
    pub fingerprint: String,
    pub derivation_path: String,
    #[serde(default)]
    pub hwi_path: Option<String>,
    #[serde(default)]
    pub network: Option<NetworkName>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HwXpubDto {
    pub fingerprint: String,
    pub derivation_path: String,
    pub xpub: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HwSignPsbtRequest {
    pub fingerprint: String,
    pub psbt_base64: String,
    #[serde(default)]
    pub hwi_path: Option<String>,
    #[serde(default)]
    pub network: Option<NetworkName>,
    #[serde(default)]
    pub wallet_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HwStatusDto {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub source: Option<String>,
    pub pinned_version: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HwRegisterRequest {
    pub wallet_id: String,
    pub fingerprint: String,
    #[serde(default)]
    pub hwi_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HwRegisterResultDto {
    pub ok: bool,
    pub message: String,
    pub hmac: Option<String>,
    pub package: signing_devices::RegistrationPackage,
    pub cosigner_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerRegistrationStatusDto {
    pub registered: bool,
    pub stale: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_reason: Option<String>,
    pub fingerprint: String,
    pub python_available: bool,
    pub ledger_cli_ready: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerRuntimeStatusDto {
    pub available: bool,
    pub python_path: Option<String>,
    pub script_path: Option<String>,
    pub pinned_version: String,
    pub installed_version: Option<String>,
    pub source: Option<String>,
    pub script_ready: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzePsbtRequest {
    pub wallet_id: String,
    pub psbt_base64: String,
    #[serde(default)]
    pub active_path_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDescriptorRequest {
    pub workspace_id: String,
    pub name: String,
    pub descriptor: String,
    #[serde(default)]
    pub policy: Option<PolicyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWalletBackupRequest {
    pub workspace_id: String,
    /// Raw JSON / BSMS / bare descriptor (watch-only import).
    pub payload: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletBackupDto {
    pub format_version: String,
    pub name: String,
    pub network: NetworkName,
    pub descriptor: String,
    pub script_type: ScriptTypeName,
    pub policy: Option<PolicyConfig>,
    pub created_at: i64,
    pub json: String,
    pub descriptor_txt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BsmsExportDto {
    pub text: String,
    pub first_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotKeystoreStatusDto {
    pub exists: bool,
    pub unlocked: bool,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotWalletSummaryDto {
    pub id: String,
    pub name: String,
    pub network: NetworkName,
    pub fingerprint: String,
    pub origin_path: String,
    pub xpub: String,
    pub linked_workspace_id: Option<String>,
    pub linked_wallet_id: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHotKeystoreRequest {
    pub master_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockHotKeystoreRequest {
    pub master_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportHotWalletRequestDto {
    pub name: String,
    /// BIP-39 mnemonic, or Sparrow/Electrum-ish JSON with `mnemonic` / `seed` field.
    pub mnemonic_or_json: String,
    #[serde(default)]
    pub bip39_passphrase: String,
    pub network: NetworkName,
    /// Optional storage parent; empty → auto-pick/create a workspace for this network.
    #[serde(default)]
    pub workspace_id: String,
    #[serde(default)]
    pub account_path: Option<String>,
    #[serde(default)]
    pub create_nested_wallet: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportHotWalletResultDto {
    pub hot_wallet: HotWalletSummaryDto,
    pub wallet: Option<WalletDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignPsbtHotRequest {
    pub psbt_base64: String,
    pub hot_wallet_id: String,
    pub network: NetworkName,
    #[serde(default)]
    pub allow_mainnet_hot_keys: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedPsbtDto {
    pub base64: String,
    pub input_count: usize,
    pub output_count: usize,
    pub signed_inputs: usize,
    pub total_inputs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombinePsbtRequest {
    pub parts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxOutputDto {
    pub address: Option<String>,
    pub amount_sats: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalizedTxDto {
    pub hex: String,
    pub txid: String,
    pub fully_signed: bool,
    pub outputs: Vec<TxOutputDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastTxRequest {
    pub wallet_id: String,
    pub psbt_base64: Option<String>,
    pub tx_hex: Option<String>,
    pub esplora_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SparrowExportDto {
    pub name: String,
    pub descriptor: String,
    pub network: NetworkName,
    pub import_instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerPresetDto {
    pub label: String,
    pub backend: String,
    pub url: String,
    pub network: NetworkName,
}

impl From<Workspace> for WorkspaceDto {
    fn from(value: Workspace) -> Self {
        Self {
            id: value.id,
            name: value.name,
            network: value.network,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<WorkspaceSummary> for WorkspaceSummaryDto {
    fn from(value: WorkspaceSummary) -> Self {
        Self {
            id: value.id,
            name: value.name,
            network: value.network,
            wallet_count: value.wallet_count,
            created_at: value.created_at,
        }
    }
}

impl From<Wallet> for WalletDto {
    fn from(value: Wallet) -> Self {
        Self {
            id: value.id,
            workspace_id: value.workspace_id,
            name: value.name,
            policy: value.policy,
            descriptor: value.descriptor,
            script_type: value.script_type,
            created_at: value.created_at,
            watch_only: true,
        }
    }
}

impl From<WalletSummary> for WalletSummaryDto {
    fn from(value: WalletSummary) -> Self {
        Self {
            id: value.id,
            workspace_id: value.workspace_id,
            name: value.name,
            script_type: value.script_type,
            created_at: value.created_at,
            watch_only: true,
        }
    }
}

impl From<address_engine::DerivedAddress> for AddressDto {
    fn from(value: address_engine::DerivedAddress) -> Self {
        Self {
            address: value.address,
            index: value.index,
            is_change: value.is_change,
        }
    }
}

impl From<blockchain::Balance> for BalanceDto {
    fn from(value: blockchain::Balance) -> Self {
        Self {
            confirmed_sats: value.confirmed_sats,
            unconfirmed_sats: value.unconfirmed_sats,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxSummaryDto {
    pub txid: String,
    pub amount_sats: i64,
    pub confirmed: bool,
    pub block_height: Option<u32>,
    /// Unix seconds when confirmed (from Esplora `block_time`).
    pub block_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResultDto {
    pub balance: BalanceDto,
    pub utxos: Vec<UtxoDto>,
    pub history: Vec<TxSummaryDto>,
}

impl From<blockchain::Utxo> for UtxoDto {
    fn from(value: blockchain::Utxo) -> Self {
        Self {
            txid: value.txid,
            vout: value.vout,
            value_sats: value.value_sats,
            address: value.address,
            confirmed: value.confirmed,
            block_height: value.block_height,
            derivation_index: value.derivation_index,
            is_change: value.is_change,
        }
    }
}

impl From<blockchain::TxSummary> for TxSummaryDto {
    fn from(value: blockchain::TxSummary) -> Self {
        Self {
            txid: value.txid,
            amount_sats: value.amount_sats,
            confirmed: value.confirmed,
            block_height: value.block_height,
            block_time: value.block_time,
        }
    }
}

impl From<blockchain::SyncResult> for SyncResultDto {
    fn from(value: blockchain::SyncResult) -> Self {
        Self {
            balance: BalanceDto::from(value.balance),
            utxos: value.utxos.into_iter().map(UtxoDto::from).collect(),
            history: value.history.into_iter().map(TxSummaryDto::from).collect(),
        }
    }
}
