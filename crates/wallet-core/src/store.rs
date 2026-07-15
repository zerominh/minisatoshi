use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use address_engine::{new_change_address, new_receive_address, DerivedAddress};
use descriptor_engine::{compile_descriptor_from_config, ensure_descriptor_checksum};
use policy_engine::{NetworkName, PolicyConfig, ScriptTypeName};
use storage::{Database, NewAddress, NewWallet, NewWorkspace, StorageError};
use uuid::Uuid;

use crate::backup::WalletBackup;
use crate::error::WalletError;
use crate::types::{
    network_from_str, network_to_str, script_type_from_str, script_type_to_str, Wallet,
    WalletSummary, Workspace, WorkspaceSummary,
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

    pub fn create_workspace(
        &self,
        name: &str,
        network: policy_engine::NetworkName,
    ) -> Result<Workspace, WalletError> {
        if name.trim().is_empty() {
            return Err(WalletError::EmptyWorkspaceName);
        }

        let now = unix_now();
        let record = self.db.insert_workspace(&NewWorkspace {
            id: Uuid::new_v4().to_string(),
            name: name.trim().to_string(),
            network: network_to_str(network).to_string(),
            created_at: now,
        })?;

        workspace_from_record(record)
    }

    pub fn open_workspace(&self, id: &str) -> Result<Workspace, WalletError> {
        workspace_from_record(self.db.get_workspace(id)?)
    }

    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceSummary>, WalletError> {
        let mut summaries = Vec::new();
        for record in self.db.list_workspaces()? {
            let wallet_count = self.db.list_wallets_for_workspace(&record.id)?.len();
            summaries.push(WorkspaceSummary {
                id: record.id,
                name: record.name,
                network: network_from_str(&record.network).map_err(WalletError::InvalidNetwork)?,
                wallet_count,
                created_at: record.created_at,
            });
        }
        Ok(summaries)
    }

    pub fn delete_workspace(&self, workspace_id: &str) -> Result<(), WalletError> {
        self.db.delete_workspace(workspace_id)?;
        Ok(())
    }

    pub fn delete_wallet(&self, wallet_id: &str) -> Result<(), WalletError> {
        self.db.delete_wallet(wallet_id)?;
        Ok(())
    }

    pub fn rename_workspace(
        &self,
        workspace_id: &str,
        name: &str,
    ) -> Result<Workspace, WalletError> {
        if name.trim().is_empty() {
            return Err(WalletError::EmptyWorkspaceName);
        }
        let now = unix_now();
        workspace_from_record(self.db.rename_workspace(workspace_id, name.trim(), now)?)
    }

    pub fn rename_wallet(&self, wallet_id: &str, name: &str) -> Result<Wallet, WalletError> {
        if name.trim().is_empty() {
            return Err(WalletError::EmptyWalletName);
        }
        wallet_from_record(self.db.rename_wallet(wallet_id, name.trim())?)
    }

    pub fn backup_workspace(&self, _id: &str, destination: &Path) -> Result<(), WalletError> {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(StorageError::from)?;
        }
        std::fs::copy(&self.path, destination).map_err(StorageError::from)?;
        Ok(())
    }

    pub fn restore_workspace(source: &Path, destination: &Path) -> Result<Self, WalletError> {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(StorageError::from)?;
        }
        std::fs::copy(source, destination).map_err(StorageError::from)?;
        Self::open(destination)
    }

    pub fn create_wallet(
        &self,
        workspace_id: &str,
        name: &str,
        policy: PolicyConfig,
    ) -> Result<Wallet, WalletError> {
        if name.trim().is_empty() {
            return Err(WalletError::EmptyWalletName);
        }

        // Ensure workspace exists.
        self.db.get_workspace(workspace_id)?;

        let descriptor = compile_descriptor_from_config(&policy)?;
        let now = unix_now();
        let record = self.db.insert_wallet(&NewWallet {
            id: Uuid::new_v4().to_string(),
            workspace_id: workspace_id.to_string(),
            name: name.trim().to_string(),
            policy_json: serde_json::to_string(&policy)?,
            descriptor,
            script_type: script_type_to_str(policy.script_type).to_string(),
            created_at: now,
        })?;

        self.db.touch_workspace(workspace_id, now)?;
        wallet_from_record(record)
    }

    pub fn import_descriptor(
        &self,
        workspace_id: &str,
        name: &str,
        descriptor: &str,
        policy: Option<PolicyConfig>,
    ) -> Result<Wallet, WalletError> {
        if name.trim().is_empty() {
            return Err(WalletError::EmptyWalletName);
        }

        let workspace = self.db.get_workspace(workspace_id)?;
        let workspace_network =
            network_from_str(&workspace.network).map_err(WalletError::InvalidNetwork)?;
        let (normalized, script_type) = validate_imported_descriptor(descriptor)?;

        let policy = match policy {
            Some(mut policy) => {
                if policy.network != workspace_network {
                    return Err(WalletError::NetworkMismatch {
                        workspace: network_to_str(workspace_network).into(),
                        provided: network_to_str(policy.network).into(),
                    });
                }
                if policy.script_type != script_type {
                    policy.script_type = script_type;
                }
                // Prefer compiled descriptor from policy when it matches what was pasted.
                if let Ok(compiled) = compile_descriptor_from_config(&policy) {
                    if strip_checksum(&compiled) != strip_checksum(&normalized)
                        && compiled != normalized
                    {
                        // Keep user descriptor as source of truth; still store policy metadata.
                    }
                }
                policy
            }
            None => imported_policy_placeholder(workspace_network, script_type),
        };

        let now = unix_now();
        let record = self.db.insert_wallet(&NewWallet {
            id: Uuid::new_v4().to_string(),
            workspace_id: workspace_id.to_string(),
            name: name.trim().to_string(),
            policy_json: serde_json::to_string(&policy)?,
            descriptor: normalized,
            script_type: script_type_to_str(script_type).to_string(),
            created_at: now,
        })?;

        self.db.touch_workspace(workspace_id, now)?;
        wallet_from_record(record)
    }

    pub fn export_descriptor(&self, wallet_id: &str) -> Result<String, WalletError> {
        Ok(self.db.get_wallet(wallet_id)?.descriptor)
    }

    /// Export a portable `minisatoshi-wallet-v1` backup (descriptor + optional policy).
    pub fn export_wallet_backup(&self, wallet_id: &str) -> Result<WalletBackup, WalletError> {
        let wallet = self.get_wallet(wallet_id)?;
        let has_real_policy = wallet.policy.policy.primary != "imported"
            || !wallet.policy.keys.is_empty();
        Ok(WalletBackup::new(
            wallet.name,
            wallet.policy.network,
            wallet.descriptor,
            wallet.script_type,
            if has_real_policy {
                Some(wallet.policy)
            } else {
                None
            },
            wallet.created_at,
        ))
    }

    /// Import from a `minisatoshi-wallet-v1` (or legacy `minisatoshi-vault-v1`) JSON backup package.
    pub fn import_wallet_backup(
        &self,
        workspace_id: &str,
        backup: &WalletBackup,
        name_override: Option<&str>,
    ) -> Result<Wallet, WalletError> {
        if !backup.is_supported_format() {
            return Err(WalletError::UnsupportedBackupFormat(
                backup.format_version.clone(),
            ));
        }
        let workspace = self.db.get_workspace(workspace_id)?;
        let workspace_network =
            network_from_str(&workspace.network).map_err(WalletError::InvalidNetwork)?;
        if backup.network != workspace_network {
            return Err(WalletError::NetworkMismatch {
                workspace: network_to_str(workspace_network).into(),
                provided: network_to_str(backup.network).into(),
            });
        }
        let name = name_override
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(backup.name.trim());
        self.import_descriptor(workspace_id, name, &backup.descriptor, backup.policy.clone())
    }

    /// Import watch-only from backup / bare descriptor / BSMS / Liana-ish JSON.
    pub fn import_watch_only_payload(
        &self,
        workspace_id: &str,
        payload: &str,
        name_override: Option<&str>,
    ) -> Result<Wallet, WalletError> {
        let parsed = crate::import_parse::parse_watch_only_payload(payload)?;
        let workspace = self.db.get_workspace(workspace_id)?;
        let workspace_network =
            network_from_str(&workspace.network).map_err(WalletError::InvalidNetwork)?;
        if let Some(network) = parsed.network {
            if network != workspace_network {
                return Err(WalletError::NetworkMismatch {
                    workspace: network_to_str(workspace_network).into(),
                    provided: network_to_str(network).into(),
                });
            }
        }
        let name = name_override
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .or(parsed.name.as_deref().map(str::trim).filter(|s| !s.is_empty()))
            .unwrap_or("Imported wallet");
        self.import_descriptor(workspace_id, name, &parsed.descriptor, parsed.policy)
    }

    /// BIP-129-ish BSMS descriptor record for watch-only sharing.
    pub fn export_bsms(&self, wallet_id: &str) -> Result<String, WalletError> {
        let wallet = self.get_wallet(wallet_id)?;
        let first = self
            .list_addresses(wallet_id)?
            .into_iter()
            .find(|a| !a.is_change && a.index == 0)
            .map(Ok)
            .unwrap_or_else(|| {
                new_receive_address(&wallet.policy, &wallet.descriptor, 0).map_err(WalletError::from)
            })?;
        Ok(crate::import_parse::format_bsms(
            &wallet.descriptor,
            &first.address,
        ))
    }

    pub fn get_wallet(&self, wallet_id: &str) -> Result<Wallet, WalletError> {
        wallet_from_record(self.db.get_wallet(wallet_id)?)
    }

    pub fn list_wallets(&self, workspace_id: &str) -> Result<Vec<WalletSummary>, WalletError> {
        self.db.get_workspace(workspace_id)?;
        let mut wallets = Vec::new();
        for record in self.db.list_wallets_for_workspace(workspace_id)? {
            wallets.push(WalletSummary {
                id: record.id,
                workspace_id: record.workspace_id,
                name: record.name,
                script_type: script_type_from_str(&record.script_type)
                    .map_err(WalletError::InvalidScriptType)?,
                created_at: record.created_at,
            });
        }
        Ok(wallets)
    }

    pub fn save_address(
        &self,
        wallet_id: &str,
        address: &str,
        index: u32,
        is_change: bool,
    ) -> Result<DerivedAddress, WalletError> {
        self.db.get_wallet(wallet_id)?;
        let now = unix_now();
        self.db.insert_address(&NewAddress {
            id: Uuid::new_v4().to_string(),
            wallet_id: wallet_id.to_string(),
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
        wallet_id: &str,
        index: u32,
    ) -> Result<DerivedAddress, WalletError> {
        let wallet = self.get_wallet(wallet_id)?;
        let derived = new_receive_address(&wallet.policy, &wallet.descriptor, index)
            .map_err(WalletError::from)?;
        self.save_address(wallet_id, &derived.address, derived.index, derived.is_change)
    }

    pub fn derive_and_save_change_address(
        &self,
        wallet_id: &str,
        index: u32,
    ) -> Result<DerivedAddress, WalletError> {
        let wallet = self.get_wallet(wallet_id)?;
        let derived = new_change_address(&wallet.policy, &wallet.descriptor, index)
            .map_err(WalletError::from)?;
        self.save_address(wallet_id, &derived.address, derived.index, derived.is_change)
    }

    pub fn next_receive_address(&self, wallet_id: &str) -> Result<DerivedAddress, WalletError> {
        let index = self
            .db
            .max_address_index(wallet_id, false)?
            .map(|value| value + 1)
            .unwrap_or(0);
        self.derive_and_save_receive_address(wallet_id, index)
    }

    pub fn list_addresses(&self, wallet_id: &str) -> Result<Vec<DerivedAddress>, WalletError> {
        self.db.get_wallet(wallet_id)?;
        let records = self.db.list_addresses_for_wallet(wallet_id)?;
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

fn workspace_from_record(record: storage::WorkspaceRecord) -> Result<Workspace, WalletError> {
    Ok(Workspace {
        id: record.id,
        name: record.name,
        network: network_from_str(&record.network).map_err(WalletError::InvalidNetwork)?,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn wallet_from_record(record: storage::WalletRecord) -> Result<Wallet, WalletError> {
    Ok(Wallet {
        id: record.id,
        workspace_id: record.workspace_id,
        name: record.name,
        policy: serde_json::from_str(&record.policy_json)?,
        descriptor: record.descriptor,
        script_type: script_type_from_str(&record.script_type)
            .map_err(WalletError::InvalidScriptType)?,
        created_at: record.created_at,
    })
}

fn validate_imported_descriptor(
    descriptor: &str,
) -> Result<(String, ScriptTypeName), WalletError> {
    let desc = descriptor.trim();
    if desc.is_empty() {
        return Err(WalletError::InvalidDescriptor("descriptor is empty".into()));
    }
    let normalized = ensure_descriptor_checksum(desc).map_err(|e| {
        WalletError::InvalidDescriptor(format!("checksum or parse failed: {e}"))
    })?;
    let script_type = detect_script_type(&normalized)?;
    Ok((normalized, script_type))
}

fn strip_checksum(descriptor: &str) -> &str {
    descriptor
        .rsplit_once('#')
        .map(|(body, _)| body)
        .unwrap_or(descriptor)
        .trim()
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
    network: NetworkName,
    script_type: ScriptTypeName,
) -> PolicyConfig {
    PolicyConfig {
        version: policy_engine::POLICY_SCHEMA_VERSION,
        network,
        script_type,
        keys: Vec::new(),
        policy: policy_engine::PolicyExpression {
            primary: "imported".into(),
            fallback: None,
            fallbacks: vec![],
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
        test_vectors::TEST_XPUB_C, KeyConfig, KeyRole, NetworkName, ScriptTypeName,
    };

    use crate::backup::WalletBackup;
    use crate::error::WalletError;

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
    fn workspace_wallet_descriptor_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("wallet.db");
        let store = WalletStore::open(&db_path).unwrap();

        let workspace = store
            .create_workspace("Family Fund", NetworkName::Testnet)
            .unwrap();
        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );

        let wallet = store
            .create_wallet(&workspace.id, "ABC Wallet", policy)
            .unwrap();

        let exported = store.export_descriptor(&wallet.id).unwrap();
        assert!(exported.starts_with("tr("));
        assert_eq!(exported, wallet.descriptor);

        let reopened = store.open_workspace(&workspace.id).unwrap();
        assert_eq!(reopened.name, "Family Fund");

        let listed = store.list_workspaces().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].wallet_count, 1);
    }

    #[test]
    fn backup_and_restore_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("wallet.db");
        let backup_path = dir.path().join("backup.db");
        let restored_path = dir.path().join("restored.db");

        let store = WalletStore::open(&db_path).unwrap();
        let workspace = store
            .create_workspace("Backup Test", NetworkName::Signet)
            .unwrap();
        store
            .backup_workspace(&workspace.id, &backup_path)
            .unwrap();

        let restored = WalletStore::restore_workspace(&backup_path, &restored_path).unwrap();
        let loaded = restored.open_workspace(&workspace.id).unwrap();
        assert_eq!(loaded.name, "Backup Test");
    }

    #[test]
    fn import_and_export_descriptor() {
        let dir = tempfile::tempdir().unwrap();
        let store = WalletStore::open(dir.path().join("wallet.db")).unwrap();
        let workspace = store
            .create_workspace("Import", NetworkName::Testnet)
            .unwrap();

        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let wallet = store
            .create_wallet(&workspace.id, "Source", policy)
            .unwrap();
        let descriptor = store.export_descriptor(&wallet.id).unwrap();

        let imported = store
            .import_descriptor(&workspace.id, "Imported Wallet", &descriptor, None)
            .unwrap();
        let exported = store.export_descriptor(&imported.id).unwrap();
        assert_eq!(exported, descriptor);
    }

    #[test]
    fn reject_bad_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let store = WalletStore::open(dir.path().join("wallet.db")).unwrap();
        let workspace = store
            .create_workspace("Import", NetworkName::Testnet)
            .unwrap();
        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let wallet = store
            .create_wallet(&workspace.id, "Source", policy)
            .unwrap();
        let mut bad = wallet.descriptor.clone();
        // Flip last checksum char.
        bad.pop();
        bad.push('x');
        let err = store
            .import_descriptor(&workspace.id, "Bad", &bad, None)
            .unwrap_err();
        assert!(
            matches!(err, WalletError::InvalidDescriptor(_)),
            "{err}"
        );
    }

    #[test]
    fn reject_network_mismatch_on_backup() {
        let dir = tempfile::tempdir().unwrap();
        let store = WalletStore::open(dir.path().join("wallet.db")).unwrap();
        let workspace = store
            .create_workspace("Testnet W", NetworkName::Testnet)
            .unwrap();
        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Mainnet,
        );
        // Craft backup claiming mainnet while descriptor is still valid parse-wise for import path.
        let desc = include_str!("../../../tests/vectors/policy_abc_mainnet_descriptor.txt").trim();
        let backup = WalletBackup::new(
            "Mainnet wallet",
            NetworkName::Mainnet,
            desc,
            ScriptTypeName::Taproot,
            Some(policy),
            0,
        );
        let err = store
            .import_wallet_backup(&workspace.id, &backup, None)
            .unwrap_err();
        assert!(
            matches!(err, WalletError::NetworkMismatch { .. }),
            "{err}"
        );
    }

    #[test]
    fn backup_roundtrip_same_receive_address_index_0() {
        let dir = tempfile::tempdir().unwrap();
        let store_a = WalletStore::open(dir.path().join("a.db")).unwrap();
        let workspace_a = store_a
            .create_workspace("A", NetworkName::Testnet)
            .unwrap();
        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let wallet = store_a
            .create_wallet(&workspace_a.id, "ABC", policy)
            .unwrap();
        let addr0 = store_a
            .derive_and_save_receive_address(&wallet.id, 0)
            .unwrap();
        let backup = store_a.export_wallet_backup(&wallet.id).unwrap();
        assert_eq!(backup.format_version, crate::backup::WALLET_BACKUP_FORMAT);
        assert!(backup.policy.is_some());

        let store_b = WalletStore::open(dir.path().join("b.db")).unwrap();
        let workspace_b = store_b
            .create_workspace("B", NetworkName::Testnet)
            .unwrap();
        let imported = store_b
            .import_wallet_backup(&workspace_b.id, &backup, Some("Restored"))
            .unwrap();
        let addr0_b = store_b
            .derive_and_save_receive_address(&imported.id, 0)
            .unwrap();
        assert_eq!(addr0.address, addr0_b.address);
    }
}
