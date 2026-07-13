use address_engine::DerivedAddress;
use wallet_core::Vault;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultWithAddress {
    pub vault: Vault,
    pub receive_address: DerivedAddress,
}
