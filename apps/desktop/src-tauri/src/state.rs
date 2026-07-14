//! Shared application state for the Tauri backend.

use std::path::PathBuf;
use std::sync::Mutex;

use hot_keystore::HotKeystore;
use wallet_core::WalletStore;

pub struct AppState {
    pub store: Mutex<WalletStore>,
    /// Unlocked hot keystore for test software signing (None when locked).
    pub hot_keystore: Mutex<Option<HotKeystore>>,
    /// App data directory (DB, bundled HWI, hot keystore, …).
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn open(data_dir: PathBuf, db_path: PathBuf) -> Result<Self, String> {
        let store = WalletStore::open(&db_path).map_err(|e| e.to_string())?;
        Ok(Self {
            store: Mutex::new(store),
            hot_keystore: Mutex::new(None),
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

    pub fn with_hot_mut<T>(
        &self,
        f: impl FnOnce(&mut Option<HotKeystore>) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut guard = self
            .hot_keystore
            .lock()
            .map_err(|_| "hot keystore lock poisoned".to_string())?;
        f(&mut guard)
    }

    pub fn with_hot_unlocked<T>(
        &self,
        f: impl FnOnce(&HotKeystore) -> Result<T, String>,
    ) -> Result<T, String> {
        self.with_hot_mut(|slot| {
            let ks = slot
                .as_ref()
                .ok_or_else(|| "hot keystore is locked — unlock in Hot wallets".to_string())?;
            f(ks)
        })
    }

    pub fn with_hot_unlocked_mut<T>(
        &self,
        f: impl FnOnce(&mut HotKeystore) -> Result<T, String>,
    ) -> Result<T, String> {
        self.with_hot_mut(|slot| {
            let ks = slot
                .as_mut()
                .ok_or_else(|| "hot keystore is locked — unlock in Hot wallets".to_string())?;
            f(ks)
        })
    }
}
