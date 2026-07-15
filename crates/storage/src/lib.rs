//! SQLite persistence layer for Minisatoshi.

mod db;
mod error;
mod models;
mod schema;

pub use db::Database;
pub use error::StorageError;
pub use models::{
    AddressRecord, LabelRecord, NewAddress, NewLabel, NewTransaction, NewWallet, NewWorkspace,
    TransactionRecord, WalletRecord, WorkspaceRecord,
};
