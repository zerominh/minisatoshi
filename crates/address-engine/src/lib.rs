//! Bitcoin address derivation from output descriptors.

mod derive;
mod error;
mod types;

pub use derive::{
    derive_address, derive_address_for_vault, derive_address_from_descriptor,
    derive_address_from_policy, new_change_address, new_receive_address,
};
pub use error::AddressError;
pub use types::DerivedAddress;
