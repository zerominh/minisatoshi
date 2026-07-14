use thiserror::Error;

#[derive(Debug, Error)]
pub enum HotKeystoreError {
    #[error("keystore is locked — unlock with master password first")]
    Locked,

    #[error("keystore already unlocked")]
    AlreadyUnlocked,

    #[error("wrong master password")]
    WrongPassword,

    #[error("keystore file is corrupt or unsupported: {0}")]
    Corrupt(String),

    #[error("invalid mnemonic: {0}")]
    Mnemonic(String),

    #[error("derivation failed: {0}")]
    Derive(String),

    #[error("hot wallet not found: {0}")]
    NotFound(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("serialization: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Message(String),
}
