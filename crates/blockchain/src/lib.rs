//! Blockchain backends (Esplora / Electrum / Core) + Sparrow interop.

mod backend;
mod core_rpc;
mod electrum;
mod error;
mod esplora;
mod query;
mod scanner;
mod sparrow;
mod types;

pub use backend::BlockchainBackend;
pub use core_rpc::CoreRpcBackend;
pub use electrum::{default_electrum_url, ElectrumBackend};
pub use error::ChainError;
pub use esplora::{default_esplora_url, EsploraBackend};
pub use query::DescriptorQuery;
pub use scanner::{build_scan_plan, ScannedAddress, ScanPlan, DEFAULT_GAP_LIMIT};
pub use sparrow::{default_server_presets, export_watch_only_wallet, SparrowWalletExport};
pub use types::{
    BackendKind, Balance, ServerPreset, SyncProgress, SyncResult, TxSummary, Utxo,
};
