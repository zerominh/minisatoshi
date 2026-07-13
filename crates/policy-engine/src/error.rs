use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PolicyError {
    #[error("unsupported policy schema version: {0}")]
    UnsupportedVersion(u32),

    #[error("policy must contain at least one key")]
    EmptyKeys,

    #[error("duplicate key id: {0}")]
    DuplicateKeyId(String),

    #[error("invalid fingerprint for key '{key}': {reason}")]
    InvalidFingerprint { key: String, reason: String },

    #[error("invalid xpub for key '{key}': {reason}")]
    InvalidXpub { key: String, reason: String },

    #[error("unknown key referenced in policy: {0}")]
    UnknownKey(String),

    #[error("invalid policy expression: {0}")]
    InvalidExpression(String),

    #[error("invalid timelock duration: {0}")]
    InvalidDuration(String),

    #[error("fallback key '{key}' is not defined")]
    UnknownFallbackKey { key: String },

    #[error("miniscript compile error: {0}")]
    MiniscriptCompile(String),
}
