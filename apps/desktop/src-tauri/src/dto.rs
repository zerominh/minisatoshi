//! Serializable DTOs shared across the Tauri IPC boundary.

use policy_engine::{NetworkName, PolicyConfig, ScriptTypeName};
use serde::{Deserialize, Serialize};
use wallet_core::{Vault, VaultSummary, Wallet, WalletSummary};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletDto {
    pub id: String,
    pub name: String,
    pub network: NetworkName,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletSummaryDto {
    pub id: String,
    pub name: String,
    pub network: NetworkName,
    pub vault_count: usize,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultDto {
    pub id: String,
    pub wallet_id: String,
    pub name: String,
    pub policy: PolicyConfig,
    pub descriptor: String,
    pub script_type: ScriptTypeName,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultSummaryDto {
    pub id: String,
    pub wallet_id: String,
    pub name: String,
    pub script_type: ScriptTypeName,
    pub created_at: i64,
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
pub struct CompileVaultResponse {
    pub descriptor: String,
    pub policy_string: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWalletRequest {
    pub name: String,
    pub network: NetworkName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVaultRequest {
    pub wallet_id: String,
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
    pub vault_id: String,
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

impl From<Wallet> for WalletDto {
    fn from(value: Wallet) -> Self {
        Self {
            id: value.id,
            name: value.name,
            network: value.network,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<WalletSummary> for WalletSummaryDto {
    fn from(value: WalletSummary) -> Self {
        Self {
            id: value.id,
            name: value.name,
            network: value.network,
            vault_count: value.vault_count,
            created_at: value.created_at,
        }
    }
}

impl From<Vault> for VaultDto {
    fn from(value: Vault) -> Self {
        Self {
            id: value.id,
            wallet_id: value.wallet_id,
            name: value.name,
            policy: value.policy,
            descriptor: value.descriptor,
            script_type: value.script_type,
            created_at: value.created_at,
        }
    }
}

impl From<VaultSummary> for VaultSummaryDto {
    fn from(value: VaultSummary) -> Self {
        Self {
            id: value.id,
            wallet_id: value.wallet_id,
            name: value.name,
            script_type: value.script_type,
            created_at: value.created_at,
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
