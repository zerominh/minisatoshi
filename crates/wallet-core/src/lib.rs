//! Wallet lifecycle: create, open, backup, vault + descriptor management.

mod backup;
mod error;
mod store;
mod types;

pub use backup::{VaultBackup, VAULT_BACKUP_FORMAT};
pub use error::WalletError;
pub use store::WalletStore;
pub use types::{Vault, VaultSummary, Wallet, WalletSummary};

pub use address_engine::DerivedAddress;
