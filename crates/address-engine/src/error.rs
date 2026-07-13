use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AddressError {
    #[error("descriptor parse error: {0}")]
    Parse(String),

    #[error("derivation error: {0}")]
    Derivation(String),

    #[error("unsupported multipath layout for change={is_change}")]
    UnsupportedMultipath { is_change: bool },

    #[error("address encoding error: {0}")]
    Encoding(String),
}
