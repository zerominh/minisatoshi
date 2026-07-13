use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("wallet not found: {0}")]
    WalletNotFound(String),

    #[error("vault not found: {0}")]
    VaultNotFound(String),

    #[error("address not found: {0}")]
    AddressNotFound(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
