use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChainError {
    #[error("http error: {0}")]
    Http(String),

    #[error("api error: {0}")]
    Api(String),

    #[error("address error: {0}")]
    Address(#[from] address_engine::AddressError),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("broadcast failed: {0}")]
    Broadcast(String),
}
