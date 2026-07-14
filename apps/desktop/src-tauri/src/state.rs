//! Shared application state for the Tauri backend.

use std::path::PathBuf;
use std::sync::Mutex;

use wallet_core::WalletStore;

pub struct AppState {
    pub store: Mutex<WalletStore>,
    /// App data directory (DB, bundled HWI, …).
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn open(data_dir: PathBuf, db_path: PathBuf) -> Result<Self, String> {
        let store = WalletStore::open(&db_path).map_err(|e| e.to_string())?;
        Ok(Self {
            store: Mutex::new(store),
            data_dir,
        })
    }

    pub fn with_store<T>(
        &self,
        f: impl FnOnce(&WalletStore) -> Result<T, String>,
    ) -> Result<T, String> {
        let store = self
            .store
            .lock()
            .map_err(|_| "wallet store lock poisoned".to_string())?;
        f(&store)
    }
}
