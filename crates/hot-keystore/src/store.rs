//! Sealed on-disk hot keystore (Argon2id + XChaCha20-Poly1305).

use std::fs;
use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use policy_engine::NetworkName;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::error::HotKeystoreError;

pub const KEYSTORE_FILENAME: &str = "hot-keystore.v1";
const MAGIC: &[u8] = b"MSHOT1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const KEY_LEN: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotWalletRecord {
    pub id: String,
    pub name: String,
    pub network: NetworkName,
    pub fingerprint: String,
    pub origin_path: String,
    pub xpub: String,
    pub mnemonic: String,
    pub bip39_passphrase: String,
    /// Miniscript descriptor secret string for `sign_psbt_software`.
    pub descriptor_secret: String,
    pub linked_wallet_id: Option<String>,
    pub linked_vault_id: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotWalletSummary {
    pub id: String,
    pub name: String,
    pub network: NetworkName,
    pub fingerprint: String,
    pub origin_path: String,
    pub xpub: String,
    pub linked_wallet_id: Option<String>,
    pub linked_vault_id: Option<String>,
    pub created_at: i64,
}

impl From<&HotWalletRecord> for HotWalletSummary {
    fn from(value: &HotWalletRecord) -> Self {
        Self {
            id: value.id.clone(),
            name: value.name.clone(),
            network: value.network,
            fingerprint: value.fingerprint.clone(),
            origin_path: value.origin_path.clone(),
            xpub: value.xpub.clone(),
            linked_wallet_id: value.linked_wallet_id.clone(),
            linked_vault_id: value.linked_vault_id.clone(),
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct KeystorePayload {
    version: u32,
    wallets: Vec<HotWalletRecord>,
}

/// Unlocked in-memory keystore; persists sealed file on each mutation.
pub struct HotKeystore {
    path: PathBuf,
    password: String,
    payload: KeystorePayload,
}

impl HotKeystore {
    pub fn path_in(data_dir: impl AsRef<Path>) -> PathBuf {
        data_dir.as_ref().join(KEYSTORE_FILENAME)
    }

    pub fn exists(data_dir: impl AsRef<Path>) -> bool {
        Self::path_in(data_dir).is_file()
    }

    /// Create a new empty encrypted keystore (fails if file already exists).
    pub fn create(data_dir: impl AsRef<Path>, password: &str) -> Result<Self, HotKeystoreError> {
        if password.trim().is_empty() {
            return Err(HotKeystoreError::Message(
                "master password must not be empty".into(),
            ));
        }
        let path = Self::path_in(data_dir);
        if path.exists() {
            return Err(HotKeystoreError::Message(
                "hot keystore already exists — unlock instead".into(),
            ));
        }
        let store = Self {
            path,
            password: password.to_string(),
            payload: KeystorePayload {
                version: 1,
                wallets: vec![],
            },
        };
        store.persist()?;
        Ok(store)
    }

    pub fn unlock(data_dir: impl AsRef<Path>, password: &str) -> Result<Self, HotKeystoreError> {
        let path = Self::path_in(data_dir);
        if !path.is_file() {
            return Err(HotKeystoreError::Message(
                "no hot keystore yet — create one with a master password".into(),
            ));
        }
        let bytes = fs::read(&path)?;
        let payload = decrypt_file(&bytes, password)?;
        Ok(Self {
            path,
            password: password.to_string(),
            payload,
        })
    }

    pub fn list(&self) -> Vec<HotWalletSummary> {
        self.payload.wallets.iter().map(HotWalletSummary::from).collect()
    }

    pub fn get(&self, id: &str) -> Result<&HotWalletRecord, HotKeystoreError> {
        self.payload
            .wallets
            .iter()
            .find(|w| w.id == id)
            .ok_or_else(|| HotKeystoreError::NotFound(id.into()))
    }

    pub fn insert(&mut self, record: HotWalletRecord) -> Result<HotWalletSummary, HotKeystoreError> {
        if record.name.trim().is_empty() {
            return Err(HotKeystoreError::Message("name required".into()));
        }
        let summary = HotWalletSummary::from(&record);
        self.payload.wallets.push(record);
        self.persist()?;
        Ok(summary)
    }

    pub fn set_links(
        &mut self,
        id: &str,
        wallet_id: Option<String>,
        vault_id: Option<String>,
    ) -> Result<HotWalletSummary, HotKeystoreError> {
        let rec = self
            .payload
            .wallets
            .iter_mut()
            .find(|w| w.id == id)
            .ok_or_else(|| HotKeystoreError::NotFound(id.into()))?;
        rec.linked_wallet_id = wallet_id;
        rec.linked_vault_id = vault_id;
        let summary = HotWalletSummary::from(&*rec);
        self.persist()?;
        Ok(summary)
    }

    pub fn remove(&mut self, id: &str) -> Result<(), HotKeystoreError> {
        let before = self.payload.wallets.len();
        self.payload.wallets.retain(|w| w.id != id);
        if self.payload.wallets.len() == before {
            return Err(HotKeystoreError::NotFound(id.into()));
        }
        self.persist()
    }

    pub fn descriptor_secret(&self, id: &str) -> Result<String, HotKeystoreError> {
        Ok(self.get(id)?.descriptor_secret.clone())
    }

    fn persist(&self) -> Result<(), HotKeystoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let plain = serde_json::to_vec(&self.payload)?;
        let sealed = encrypt_file(&plain, &self.password)?;
        fs::write(&self.path, sealed)?;
        Ok(())
    }
}

fn argon2_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], HotKeystoreError> {
    let params = Params::new(19_456, 2, 1, Some(KEY_LEN))
        .map_err(|e| HotKeystoreError::Crypto(e.to_string()))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; KEY_LEN];
    argon
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| HotKeystoreError::Crypto(e.to_string()))?;
    Ok(key)
}

fn encrypt_file(plaintext: &[u8], password: &str) -> Result<Vec<u8>, HotKeystoreError> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let key_bytes = argon2_key(password, &salt)?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key_bytes));
    let nonce = XNonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| HotKeystoreError::Crypto(e.to_string()))?;
    let mut out = Vec::with_capacity(MAGIC.len() + SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt_file(bytes: &[u8], password: &str) -> Result<KeystorePayload, HotKeystoreError> {
    if bytes.len() < MAGIC.len() + SALT_LEN + NONCE_LEN + 16 {
        return Err(HotKeystoreError::Corrupt("file too short".into()));
    }
    if &bytes[..MAGIC.len()] != MAGIC {
        return Err(HotKeystoreError::Corrupt("bad magic".into()));
    }
    let salt = &bytes[MAGIC.len()..MAGIC.len() + SALT_LEN];
    let nonce_bytes = &bytes[MAGIC.len() + SALT_LEN..MAGIC.len() + SALT_LEN + NONCE_LEN];
    let ciphertext = &bytes[MAGIC.len() + SALT_LEN + NONCE_LEN..];
    let key_bytes = argon2_key(password, salt)?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key_bytes));
    let nonce = XNonce::from_slice(nonce_bytes);
    let plain = cipher.decrypt(nonce, ciphertext).map_err(|_| HotKeystoreError::WrongPassword)?;
    Ok(serde_json::from_slice(&plain)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::derive::{derive_bip86_account, ImportHotWalletRequest};
    use policy_engine::NetworkName;

    #[test]
    fn roundtrip_create_unlock_import() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = HotKeystore::create(dir.path(), "test-pass").unwrap();
        let (rec, _) = derive_bip86_account(&ImportHotWalletRequest {
            name: "Dev".into(),
            mnemonic: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".into(),
            bip39_passphrase: String::new(),
            network: NetworkName::Regtest,
            account_path: None,
        })
        .unwrap();
        let id = rec.id.clone();
        store.insert(rec).unwrap();
        drop(store);

        let store = HotKeystore::unlock(dir.path(), "test-pass").unwrap();
        assert_eq!(store.list().len(), 1);
        assert!(store.descriptor_secret(&id).unwrap().contains("tprv") || store.descriptor_secret(&id).unwrap().contains("xprv") || store.get(&id).unwrap().descriptor_secret.contains('/'));
        assert!(matches!(
            HotKeystore::unlock(dir.path(), "wrong"),
            Err(HotKeystoreError::WrongPassword)
        ));
    }
}
