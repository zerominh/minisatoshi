use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use address_engine::{new_change_address, new_receive_address, DerivedAddress};
use descriptor_engine::compile_descriptor_from_config;
use policy_engine::{PolicyConfig, ScriptTypeName};
use storage::{Database, NewAddress, NewVault, NewWallet, StorageError};
use uuid::Uuid;

use crate::error::WalletError;
use crate::types::{
    network_from_str, network_to_str, script_type_from_str, script_type_to_str, Vault, VaultSummary,
    Wallet, WalletSummary,
};

/// Persistent wallet database handle.
pub struct WalletStore {
    db: Database,
    path: PathBuf,
}

impl WalletStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WalletError> {
        let path = path.as_ref().to_path_buf();
        let db = Database::open(&path)?;
        Ok(Self { db, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn create_wallet(&self, name: &str, network: policy_engine::NetworkName) -> Result<Wallet, WalletError> {
        if name.trim().is_empty() {
            return Err(WalletError::EmptyWalletName);
        }

        let now = unix_now();
        let record = self.db.insert_wallet(&NewWallet {
            id: Uuid::new_v4().to_string(),
            name: name.trim().to_string(),
            network: network_to_str(network).to_string(),
            created_at: now,
        })?;

        wallet_from_record(record)
    }

    pub fn open_wallet(&self, id: &str) -> Result<Wallet, WalletError> {
        wallet_from_record(self.db.get_wallet(id)?)
    }

    pub fn list_wallets(&self) -> Result<Vec<WalletSummary>, WalletError> {
        let mut summaries = Vec::new();
        for record in self.db.list_wallets()? {
            let vault_count = self.db.list_vaults_for_wallet(&record.id)?.len();
            summaries.push(WalletSummary {
                id: record.id,
                name: record.name,
                network: network_from_str(&record.network).map_err(WalletError::InvalidNetwork)?,
                vault_count,
                created_at: record.created_at,
            });
        }
        Ok(summaries)
    }

    pub fn backup_wallet(&self, _id: &str, destination: &Path) -> Result<(), WalletError> {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(StorageError::from)?;
        }
        std::fs::copy(&self.path, destination).map_err(StorageError::from)?;
        Ok(())
    }

    pub fn restore_wallet(source: &Path, destination: &Path) -> Result<Self, WalletError> {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(StorageError::from)?;
        }
        std::fs::copy(source, destination).map_err(StorageError::from)?;
        Self::open(destination)
    }

    pub fn create_vault(
        &self,
        wallet_id: &str,
        name: &str,
        policy: PolicyConfig,
    ) -> Result<Vault, WalletError> {
        if name.trim().is_empty() {
            return Err(WalletError::EmptyVaultName);
        }

        // Ensure wallet exists.
        self.db.get_wallet(wallet_id)?;

        let descriptor = compile_descriptor_from_config(&policy)?;
        let now = unix_now();
        let record = self.db.insert_vault(&NewVault {
            id: Uuid::new_v4().to_string(),
            wallet_id: wallet_id.to_string(),
            name: name.trim().to_string(),
            policy_json: serde_json::to_string(&policy)?,
            descriptor,
            script_type: script_type_to_str(policy.script_type).to_string(),
            created_at: now,
        })?;

        self.db.touch_wallet(wallet_id, now)?;
        vault_from_record(record)
    }

    pub fn import_descriptor(
        &self,
        wallet_id: &str,
        name: &str,
        descriptor: &str,
    ) -> Result<Vault, WalletError> {
        if name.trim().is_empty() {
            return Err(WalletError::EmptyVaultName);
        }

        let wallet = self.db.get_wallet(wallet_id)?;
        let script_type = validate_imported_descriptor(descriptor)?;
        let network = network_from_str(&wallet.network).map_err(WalletError::InvalidNetwork)?;
        let policy = imported_policy_placeholder(network, script_type, descriptor);

        let now = unix_now();
        let record = self.db.insert_vault(&NewVault {
            id: Uuid::new_v4().to_string(),
            wallet_id: wallet_id.to_string(),
            name: name.trim().to_string(),
            policy_json: serde_json::to_string(&policy)?,
            descriptor: descriptor.to_string(),
            script_type: script_type_to_str(script_type).to_string(),
            created_at: now,
        })?;

        self.db.touch_wallet(wallet_id, now)?;
        vault_from_record(record)
    }

    pub fn export_descriptor(&self, vault_id: &str) -> Result<String, WalletError> {
        Ok(self.db.get_vault(vault_id)?.descriptor)
    }

    pub fn get_vault(&self, vault_id: &str) -> Result<Vault, WalletError> {
        vault_from_record(self.db.get_vault(vault_id)?)
    }

    pub fn list_vaults(&self, wallet_id: &str) -> Result<Vec<VaultSummary>, WalletError> {
        self.db.get_wallet(wallet_id)?;
        let mut vaults = Vec::new();
        for record in self.db.list_vaults_for_wallet(wallet_id)? {
            vaults.push(VaultSummary {
                id: record.id,
                wallet_id: record.wallet_id,
                name: record.name,
                script_type: script_type_from_str(&record.script_type)
                    .map_err(WalletError::InvalidScriptType)?,
                created_at: record.created_at,
            });
        }
        Ok(vaults)
    }

    pub fn save_address(
        &self,
        vault_id: &str,
        address: &str,
        index: u32,
        is_change: bool,
    ) -> Result<DerivedAddress, WalletError> {
        self.db.get_vault(vault_id)?;
        let now = unix_now();
        self.db.insert_address(&NewAddress {
            id: Uuid::new_v4().to_string(),
            vault_id: vault_id.to_string(),
            address: address.to_string(),
            index,
            is_change,
            created_at: now,
        })?;

        Ok(DerivedAddress {
            address: address.to_string(),
            index,
            is_change,
        })
    }

    pub fn derive_and_save_receive_address(
        &self,
        vault_id: &str,
        index: u32,
    ) -> Result<DerivedAddress, WalletError> {
        let vault = self.get_vault(vault_id)?;
        let derived = new_receive_address(&vault.policy, &vault.descriptor, index)
            .map_err(WalletError::from)?;
        self.save_address(vault_id, &derived.address, derived.index, derived.is_change)
    }

    pub fn derive_and_save_change_address(
        &self,
        vault_id: &str,
        index: u32,
    ) -> Result<DerivedAddress, WalletError> {
        let vault = self.get_vault(vault_id)?;
        let derived = new_change_address(&vault.policy, &vault.descriptor, index)
            .map_err(WalletError::from)?;
        self.save_address(vault_id, &derived.address, derived.index, derived.is_change)
    }

    pub fn next_receive_address(&self, vault_id: &str) -> Result<DerivedAddress, WalletError> {
        let index = self
            .db
            .max_address_index(vault_id, false)?
            .map(|value| value + 1)
            .unwrap_or(0);
        self.derive_and_save_receive_address(vault_id, index)
    }

    pub fn list_addresses(&self, vault_id: &str) -> Result<Vec<DerivedAddress>, WalletError> {
        self.db.get_vault(vault_id)?;
        let records = self.db.list_addresses_for_vault(vault_id)?;
        Ok(records
            .into_iter()
            .map(|record| DerivedAddress {
                address: record.address,
                index: record.index,
                is_change: record.is_change,
            })
            .collect())
    }
}

fn wallet_from_record(record: storage::WalletRecord) -> Result<Wallet, WalletError> {
    Ok(Wallet {
        id: record.id,
        name: record.name,
        network: network_from_str(&record.network).map_err(WalletError::InvalidNetwork)?,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn vault_from_record(record: storage::VaultRecord) -> Result<Vault, WalletError> {
    Ok(Vault {
        id: record.id,
        wallet_id: record.wallet_id,
        name: record.name,
        policy: serde_json::from_str(&record.policy_json)?,
        descriptor: record.descriptor,
        script_type: script_type_from_str(&record.script_type)
            .map_err(WalletError::InvalidScriptType)?,
        created_at: record.created_at,
    })
}

fn validate_imported_descriptor(descriptor: &str) -> Result<ScriptTypeName, WalletError> {
    let desc = descriptor.trim();
    if desc.is_empty() {
        return Err(WalletError::InvalidDescriptor("descriptor is empty".into()));
    }
    if !desc.contains('#') {
        return Err(WalletError::InvalidDescriptor(
            "descriptor must include checksum".into(),
        ));
    }
    detect_script_type(desc)
}

fn detect_script_type(descriptor: &str) -> Result<ScriptTypeName, WalletError> {
    let normalized = descriptor.split('#').next().unwrap_or(descriptor).trim();
    if normalized.starts_with("tr(") {
        Ok(ScriptTypeName::Taproot)
    } else if normalized.starts_with("wsh(") {
        Ok(ScriptTypeName::Wsh)
    } else {
        Err(WalletError::InvalidDescriptor(
            "only taproot (tr) and wsh descriptors are supported".into(),
        ))
    }
}

fn imported_policy_placeholder(
    network: policy_engine::NetworkName,
    script_type: ScriptTypeName,
    _descriptor: &str,
) -> PolicyConfig {
    PolicyConfig {
        version: policy_engine::POLICY_SCHEMA_VERSION,
        network,
        script_type,
        keys: Vec::new(),
        policy: policy_engine::PolicyExpression {
            primary: "imported".into(),
            fallback: None,
        },
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use policy_engine::{
        abc_preset, test_vectors::TEST_FP, test_vectors::TEST_XPUB_A, test_vectors::TEST_XPUB_B,
        test_vectors::TEST_XPUB_C, KeyConfig, KeyRole, NetworkName,
    };

    use super::*;

    fn sample_keys() -> [KeyConfig; 3] {
        [
            KeyConfig {
                id: "A".into(),
                role: KeyRole::Investor,
                xpub: TEST_XPUB_A.into(),
                fingerprint: "78412e3a".into(),
                origin_path: Some("44'/0'/0'".into()),
            },
            KeyConfig {
                id: "B".into(),
                role: KeyRole::Manager,
                xpub: TEST_XPUB_B.into(),
                fingerprint: TEST_FP.into(),
                origin_path: Some("86'/0'/0'".into()),
            },
            KeyConfig {
                id: "C".into(),
                role: KeyRole::Recovery,
                xpub: TEST_XPUB_C.into(),
                fingerprint: TEST_FP.into(),
                origin_path: Some("84'/0'/0'".into()),
            },
        ]
    }

    #[test]
    fn wallet_vault_descriptor_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("wallet.db");
        let store = WalletStore::open(&db_path).unwrap();

        let wallet = store.create_wallet("Family Fund", NetworkName::Testnet).unwrap();
        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );

        let vault = store
            .create_vault(&wallet.id, "ABC Vault", policy)
            .unwrap();

        let exported = store.export_descriptor(&vault.id).unwrap();
        assert!(exported.starts_with("tr("));
        assert_eq!(exported, vault.descriptor);

        let reopened = store.open_wallet(&wallet.id).unwrap();
        assert_eq!(reopened.name, "Family Fund");

        let listed = store.list_wallets().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].vault_count, 1);
    }

    #[test]
    fn backup_and_restore_wallet() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("wallet.db");
        let backup_path = dir.path().join("backup.db");
        let restored_path = dir.path().join("restored.db");

        let store = WalletStore::open(&db_path).unwrap();
        let wallet = store.create_wallet("Backup Test", NetworkName::Signet).unwrap();
        store.backup_wallet(&wallet.id, &backup_path).unwrap();

        let restored = WalletStore::restore_wallet(&backup_path, &restored_path).unwrap();
        let loaded = restored.open_wallet(&wallet.id).unwrap();
        assert_eq!(loaded.name, "Backup Test");
    }

    #[test]
    fn import_and_export_descriptor() {
        let dir = tempfile::tempdir().unwrap();
        let store = WalletStore::open(dir.path().join("wallet.db")).unwrap();
        let wallet = store.create_wallet("Import", NetworkName::Testnet).unwrap();

        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let vault = store.create_vault(&wallet.id, "Source", policy).unwrap();
        let descriptor = store.export_descriptor(&vault.id).unwrap();

        let imported = store
            .import_descriptor(&wallet.id, "Imported Vault", &descriptor)
            .unwrap();
        let exported = store.export_descriptor(&imported.id).unwrap();
        assert_eq!(exported, descriptor);
    }
}
