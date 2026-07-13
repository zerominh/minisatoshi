use thiserror::Error;

use address_engine::AddressError;
use descriptor_engine::DescriptorError;
use policy_engine::PolicyError;
use storage::StorageError;

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("policy error: {0}")]
    Policy(#[from] PolicyError),

    #[error("descriptor error: {0}")]
    Descriptor(#[from] DescriptorError),

    #[error("address error: {0}")]
    Address(#[from] AddressError),

    #[error("invalid network: {0}")]
    InvalidNetwork(String),

    #[error("wallet name must not be empty")]
    EmptyWalletName,

    #[error("vault name must not be empty")]
    EmptyVaultName,

    #[error("descriptor import is invalid: {0}")]
    InvalidDescriptor(String),

    #[error("invalid script type: {0}")]
    InvalidScriptType(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
