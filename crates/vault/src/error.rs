use thiserror::Error;

use address_engine::AddressError;
use policy_engine::PolicyError;
use wallet_core::WalletError;

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("wallet error: {0}")]
    Wallet(#[from] WalletError),

    #[error("address error: {0}")]
    Address(#[from] AddressError),

    #[error("policy error: {0}")]
    Policy(#[from] PolicyError),
}
