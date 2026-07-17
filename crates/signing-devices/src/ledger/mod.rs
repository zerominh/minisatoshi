//! Ledger wallet-policy signing via ledger-bitcoin (Python subprocess).

mod cli;
mod device;
mod errors;
mod install;
mod store;

pub use cli::{
    ledger_chain, probe_device, register_wallet, resolve_ledger_cli, sign_psbt, LedgerCliConfig,
};
pub use device::{
    evaluate_ledger_readiness, expected_bitcoin_app_name, invalid_older_blocks_in_policy,
    parse_app_version, LedgerReadiness, BIP68_MAX_BLOCK_RELATIVE,
    MIN_BITCOIN_APP_TAPROOT_MINISCRIPT,
};
pub use errors::map_ledger_cli_error;
pub use install::{
    ensure_ledger_cli_script, ensure_ledger_runtime, find_ledger_runtime, install_ledger_runtime,
    ledger_hid_works, ledger_import_works, runtime_source_label, LedgerRuntimeSource,
    ResolvedLedgerRuntime, PINNED_LEDGER_BITCOIN_VERSION, RUNTIME_DEPS_TAG,
};
pub use store::{
    delete_registration, is_registered, load_registration, registration_stale_reason,
    save_registration, LedgerRegistration,
};
