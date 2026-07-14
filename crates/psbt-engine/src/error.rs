use thiserror::Error;

use address_engine::AddressError;
use blockchain::ChainError;
use descriptor_engine::DescriptorError;
use miniscript::psbt::Error as MiniscriptPsbtError;
use policy_engine::PolicyError;

#[derive(Debug, Error)]
pub enum PsbtError {
    #[error("policy error: {0}")]
    Policy(#[from] PolicyError),

    #[error("descriptor error: {0}")]
    Descriptor(#[from] DescriptorError),

    #[error("address error: {0}")]
    Address(#[from] AddressError),

    #[error("blockchain error: {0}")]
    Blockchain(#[from] ChainError),

    #[error("psbt error: {0}")]
    Psbt(String),

    #[error("miniscript psbt error: {0}")]
    Miniscript(#[from] MiniscriptPsbtError),

    #[error("bitcoin encode error: {0}")]
    Encode(String),

    #[error("invalid address: {0}")]
    InvalidAddress(String),

    #[error("insufficient funds: need {needed} sats, have {available} sats")]
    InsufficientFunds { needed: u64, available: u64 },

    #[error("no spendable inputs provided")]
    NoInputs,

    #[error("no payment outputs provided")]
    NoOutputs,

    #[error("signing error: {0}")]
    Signing(String),

    #[error("finalize error: {0}")]
    Finalize(String),

    #[error("psbt not finalized")]
    NotFinalized,
}

impl From<bitcoin::psbt::Error> for PsbtError {
    fn from(value: bitcoin::psbt::Error) -> Self {
        Self::Psbt(value.to_string())
    }
}

impl From<bitcoin::consensus::encode::Error> for PsbtError {
    fn from(value: bitcoin::consensus::encode::Error) -> Self {
        Self::Encode(value.to_string())
    }
}
