//! Hardware signing device abstraction (HWI subprocess).

mod error;
mod hwi;
mod install;
mod ledger;
mod registration;
mod types;

pub use error::SignError;
pub use hwi::{
    is_registerpolicy_unavailable, parse_derivation_path, parse_enumerate_json, HwiClient,
    HwiConfig, HwiDeviceSigner,
};
pub use install::{
    bundled_hwi_binary, ensure_hwi, find_hwi, hwi_works, install_hwi, HwiSource, ResolvedHwi,
    PINNED_HWI_VERSION,
};
pub use ledger::{
    delete_registration, ensure_ledger_cli_script, ensure_ledger_runtime, find_ledger_runtime,
    install_ledger_runtime, is_registered as ledger_is_registered, ledger_chain,
    ledger_import_works, load_registration, map_ledger_cli_error, register_wallet as ledger_register_wallet,
    registration_stale_reason, resolve_ledger_cli, runtime_source_label, save_registration,
    sign_psbt as ledger_sign_psbt, LedgerCliConfig, LedgerRegistration, LedgerRuntimeSource,
    ResolvedLedgerRuntime, PINNED_LEDGER_BITCOIN_VERSION,
};
pub use registration::{
    bip388_policy_fingerprint, build_registration_package, descriptor_to_bip388,
    find_key_by_fingerprint, hwi_chain, is_taproot_script_path_miniscript,
    ledger_registers_on_first_psbt, primary_cosigner_hints, single_key_display_descriptor,
    to_ledger_wallet_policy, validate_ledger_policy_template, Bip388Policy, RegistrationPackage,
    VendorRegistration, LEDGER_MAX_POLICY_TEMPLATE_LEN,
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
