use policy_engine::NetworkName;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    Esplora,
    Electrum,
    Core,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerPreset {
    pub label: String,
    pub backend: BackendKind,
    pub url: String,
    pub network: NetworkName,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Balance {
    pub confirmed_sats: u64,
    pub unconfirmed_sats: u64,
}

impl Balance {
    pub fn zero() -> Self {
        Self::default()
    }

    pub fn total_sats(&self) -> u64 {
        self.confirmed_sats + self.unconfirmed_sats
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub value_sats: u64,
    pub address: String,
    pub confirmed: bool,
    pub block_height: Option<u32>,
    pub derivation_index: u32,
    pub is_change: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxSummary {
    pub txid: String,
    pub amount_sats: i64,
    pub confirmed: bool,
    pub block_height: Option<u32>,
    /// Unix seconds (block time). Esplora fills this; Electrum may leave `None`.
    pub block_time: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncProgress {
    pub scanned_addresses: u32,
    pub active_addresses: u32,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncResult {
    pub balance: Balance,
    pub utxos: Vec<Utxo>,
    pub history: Vec<TxSummary>,
    pub scanned_receive_count: u32,
    pub scanned_change_count: u32,
}
