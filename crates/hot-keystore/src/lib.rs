//! Master-password encrypted hot-wallet keystore for test signing.
//!
//! File format (`hot-keystore.v1`): Argon2id → XChaCha20-Poly1305 sealed JSON.
//! True SQLCipher for the shared app DB is deferred (cannot link plain SQLite +
//! SQLCipher in one binary); this store is the encrypted key vault for hot keys.

mod derive;
mod error;
mod store;

pub use derive::{
    account_policy_key, bitcoin_network, derive_bip86_account, ImportHotWalletRequest,
};
pub use error::HotKeystoreError;
pub use store::{
    HotKeystore, HotWalletRecord, HotWalletSummary, KEYSTORE_FILENAME,
};
