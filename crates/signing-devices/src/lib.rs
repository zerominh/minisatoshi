//! Hardware signing device abstraction (HWI subprocess).

mod error;
mod hwi;
mod install;
mod registration;
mod types;

pub use error::SignError;
pub use hwi::{parse_derivation_path, parse_enumerate_json, HwiClient, HwiConfig, HwiDeviceSigner};
pub use install::{
    bundled_hwi_binary, ensure_hwi, find_hwi, hwi_works, install_hwi, HwiSource, ResolvedHwi,
    PINNED_HWI_VERSION,
};
pub use registration::{
    build_registration_package, descriptor_to_bip388, hwi_chain, primary_cosigner_hints,
    Bip388Policy, RegistrationPackage, VendorRegistration,
};
pub use types::{DeviceInfo, DeviceType};

use bitcoin::bip32::DerivationPath;/// Common interface for hardware (or mock) signers.
pub trait HardwareSigner: Send + Sync {
    fn device_id(&self) -> &str;
    fn fingerprint(&self) -> Result<String, SignError>;
    fn get_xpub(&self, path: &DerivationPath) -> Result<String, SignError>;
    fn register_policy(&self, _descriptor: &str) -> Result<(), SignError> {
        Err(SignError::Unsupported(
            "register_policy not implemented for this backend".into(),
        ))
    }
    fn sign_psbt(&self, psbt_base64: &str) -> Result<String, SignError>;
}

/// List connected devices via the given HWI client.
pub fn list_devices(client: &HwiClient) -> Result<Vec<DeviceInfo>, SignError> {
    client.enumerate()
}
