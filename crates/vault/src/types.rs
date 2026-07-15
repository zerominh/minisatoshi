use address_engine::DerivedAddress;
use wallet_core::Wallet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletWithAddress {
    pub wallet: Wallet,
    pub receive_address: DerivedAddress,
}
