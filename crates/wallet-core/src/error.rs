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

    #[error("workspace name must not be empty")]
    EmptyWorkspaceName,

    #[error("wallet name must not be empty")]
    EmptyWalletName,

    #[error("descriptor import is invalid: {0}")]
    InvalidDescriptor(String),

    #[error("network mismatch: workspace is {workspace}, backup/policy is {provided}")]
    NetworkMismatch {
        workspace: String,
        provided: String,
    },

    #[error("invalid script type: {0}")]
    InvalidScriptType(String),

    #[error("unsupported backup format: {0}")]
    UnsupportedBackupFormat(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
