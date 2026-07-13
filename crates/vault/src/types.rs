use address_engine::DerivedAddress;
use wallet_core::Vault;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Balance {
    pub confirmed_sats: u64,
    pub unconfirmed_sats: u64,
}

impl Balance {
    pub fn zero() -> Self {
        Self {
            confirmed_sats: 0,
            unconfirmed_sats: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxSummary {
    pub txid: String,
    pub amount_sats: i64,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultWithAddress {
    pub vault: Vault,
    pub receive_address: DerivedAddress,
}
