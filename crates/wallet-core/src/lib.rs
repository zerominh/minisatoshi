//! Wallet lifecycle: create, open, backup, wallet + descriptor management.

mod backup;
mod error;
mod import_parse;
mod store;
mod types;

pub use backup::{
    WalletBackup, LEGACY_VAULT_BACKUP_FORMAT, WALLET_BACKUP_FORMAT,
};
pub use error::WalletError;
pub use import_parse::{
    format_bsms, parse_watch_only_payload, ImportSource, ParsedWatchOnlyImport,
};
pub use store::WalletStore;
pub use types::{Wallet, WalletSummary, Workspace, WorkspaceSummary};

pub use address_engine::DerivedAddress;
