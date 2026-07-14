use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SignError {
    #[error("HWI binary not found or failed to start: {0}")]
    Binary(String),

    #[error("HWI returned an error: {0}")]
    Hwi(String),

    #[error("failed to parse HWI JSON: {0}")]
    Parse(String),

    #[error("device not found: {0}")]
    DeviceNotFound(String),

    #[error("unsupported operation: {0}")]
    Unsupported(String),

    #[error("user cancelled or device disconnected")]
    Cancelled,

    #[error("invalid derivation path: {0}")]
    InvalidPath(String),
}
