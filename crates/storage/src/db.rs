use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::StorageError;
use crate::models::{
    AddressRecord, LabelRecord, NewAddress, NewLabel, NewTransaction, NewWallet, NewWorkspace,
    TransactionRecord, WalletRecord, WorkspaceRecord,
};
use crate::schema::{MIGRATION_V2, SCHEMA_V2, SCHEMA_VERSION};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), StorageError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);",
        )?;

        let version: Option<u32> = self
            .conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
            .optional()?;

        match version {
            None => {
                // Brand-new database: apply the final v2 schema directly.
                self.conn.execute_batch(SCHEMA_V2)?;
                self.conn.execute(
                    "INSERT INTO schema_version (version) VALUES (?1)",
                    params![SCHEMA_VERSION],
                )?;
            }
            Some(1) => {
                // Existing v1 database: rename wallets/vaults into workspaces/wallets.
                self.conn.execute_batch(MIGRATION_V2)?;
                self.conn.execute(
                    "UPDATE schema_version SET version = ?1",
                    params![SCHEMA_VERSION],
                )?;
            }
            Some(v) if v >= SCHEMA_VERSION => {
                // Already up to date; nothing to do.
            }
            Some(other) => {
                return Err(StorageError::Database(rusqlite::Error::InvalidParameterName(
                    format!("unsupported schema version: {other}"),
                )));
            }
        }

        Ok(())
    }

    pub fn insert_workspace(
        &self,
        workspace: &NewWorkspace,
    ) -> Result<WorkspaceRecord, StorageError> {
        self.conn.execute(
            "INSERT INTO workspaces (id, name, network, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![
                workspace.id,
                workspace.name,
                workspace.network,
                workspace.created_at,
            ],
        )?;

        self.get_workspace(&workspace.id)
    }

    pub fn get_workspace(&self, id: &str) -> Result<WorkspaceRecord, StorageError> {
        self.conn
            .query_row(
                "SELECT id, name, network, created_at, updated_at FROM workspaces WHERE id = ?1",
                params![id],
                |row| {
                    Ok(WorkspaceRecord {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        network: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    StorageError::WorkspaceNotFound(id.to_string())
                }
                other => StorageError::Database(other),
            })
    }

    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, network, created_at, updated_at
             FROM workspaces ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(WorkspaceRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                network: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn touch_workspace(&self, id: &str, updated_at: i64) -> Result<(), StorageError> {
        let changed = self.conn.execute(
            "UPDATE workspaces SET updated_at = ?2 WHERE id = ?1",
            params![id, updated_at],
        )?;
        if changed == 0 {
            return Err(StorageError::WorkspaceNotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn rename_workspace(
        &self,
        id: &str,
        name: &str,
        updated_at: i64,
    ) -> Result<WorkspaceRecord, StorageError> {
        let changed = self.conn.execute(
            "UPDATE workspaces SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name, updated_at],
        )?;
        if changed == 0 {
            return Err(StorageError::WorkspaceNotFound(id.to_string()));
        }
        self.get_workspace(id)
    }

    pub fn rename_wallet(&self, id: &str, name: &str) -> Result<WalletRecord, StorageError> {
        let changed = self
            .conn
            .execute("UPDATE wallets SET name = ?2 WHERE id = ?1", params![id, name])?;
        if changed == 0 {
            return Err(StorageError::WalletNotFound(id.to_string()));
        }
        self.get_wallet(id)
    }

    pub fn insert_wallet(&self, wallet: &NewWallet) -> Result<WalletRecord, StorageError> {
        self.conn.execute(
            "INSERT INTO wallets (id, workspace_id, name, policy_json, descriptor, script_type, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                wallet.id,
                wallet.workspace_id,
                wallet.name,
                wallet.policy_json,
                wallet.descriptor,
                wallet.script_type,
                wallet.created_at,
            ],
        )?;

        self.get_wallet(&wallet.id)
    }

    pub fn get_wallet(&self, id: &str) -> Result<WalletRecord, StorageError> {
        self.conn
            .query_row(
                "SELECT id, workspace_id, name, policy_json, descriptor, script_type, created_at
                 FROM wallets WHERE id = ?1",
                params![id],
                |row| {
                    Ok(WalletRecord {
                        id: row.get(0)?,
                        workspace_id: row.get(1)?,
                        name: row.get(2)?,
                        policy_json: row.get(3)?,
                        descriptor: row.get(4)?,
                        script_type: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => StorageError::WalletNotFound(id.to_string()),
                other => StorageError::Database(other),
            })
    }

    pub fn list_wallets_for_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WalletRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, workspace_id, name, policy_json, descriptor, script_type, created_at
             FROM wallets WHERE workspace_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![workspace_id], |row| {
            Ok(WalletRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                policy_json: row.get(3)?,
                descriptor: row.get(4)?,
                script_type: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn insert_address(&self, address: &NewAddress) -> Result<AddressRecord, StorageError> {
        self.conn.execute(
            "INSERT INTO addresses (id, wallet_id, address, index_num, is_change, used, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
            params![
                address.id,
                address.wallet_id,
                address.address,
                address.index,
                address.is_change as i64,
                address.created_at,
            ],
        )?;

        self.get_address(&address.id)
    }

    pub fn get_address(&self, id: &str) -> Result<AddressRecord, StorageError> {
        self.conn
            .query_row(
                "SELECT id, wallet_id, address, index_num, is_change, used, created_at
                 FROM addresses WHERE id = ?1",
                params![id],
                |row| {
                    Ok(AddressRecord {
                        id: row.get(0)?,
                        wallet_id: row.get(1)?,
                        address: row.get(2)?,
                        index: row.get::<_, i64>(3)? as u32,
                        is_change: row.get::<_, i64>(4)? != 0,
                        used: row.get::<_, i64>(5)? != 0,
                        created_at: row.get(6)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    StorageError::AddressNotFound(id.to_string())
                }
                other => StorageError::Database(other),
            })
    }

    pub fn insert_transaction(
        &self,
        tx: &NewTransaction,
    ) -> Result<TransactionRecord, StorageError> {
        self.conn.execute(
            "INSERT INTO transactions (txid, wallet_id, block_height, amount, fee, confirmed, raw_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                tx.txid,
                tx.wallet_id,
                tx.block_height,
                tx.amount,
                tx.fee,
                tx.confirmed.map(|v| v as i64),
                tx.raw_json,
            ],
        )?;

        self.get_transaction(&tx.txid, &tx.wallet_id)
    }

    pub fn get_transaction(
        &self,
        txid: &str,
        wallet_id: &str,
    ) -> Result<TransactionRecord, StorageError> {
        self.conn
            .query_row(
                "SELECT txid, wallet_id, block_height, amount, fee, confirmed, raw_json
             FROM transactions WHERE txid = ?1 AND wallet_id = ?2",
                params![txid, wallet_id],
                |row| {
                    Ok(TransactionRecord {
                        txid: row.get(0)?,
                        wallet_id: row.get(1)?,
                        block_height: row.get(2)?,
                        amount: row.get(3)?,
                        fee: row.get(4)?,
                        confirmed: row.get::<_, Option<i64>>(5)?.map(|value| value != 0),
                        raw_json: row.get(6)?,
                    })
                },
            )
            .map_err(StorageError::from)
    }

    pub fn insert_label(&self, label: &NewLabel) -> Result<LabelRecord, StorageError> {
        self.conn.execute(
            "INSERT INTO labels (id, target_type, target_id, label)
             VALUES (?1, ?2, ?3, ?4)",
            params![label.id, label.target_type, label.target_id, label.label],
        )?;

        Ok(LabelRecord {
            id: label.id.clone(),
            target_type: label.target_type.clone(),
            target_id: label.target_id.clone(),
            label: label.label.clone(),
        })
    }

    pub fn list_addresses_for_wallet(
        &self,
        wallet_id: &str,
    ) -> Result<Vec<AddressRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, wallet_id, address, index_num, is_change, used, created_at
             FROM addresses WHERE wallet_id = ?1 ORDER BY is_change ASC, index_num ASC",
        )?;
        let rows = stmt.query_map(params![wallet_id], |row| {
            Ok(AddressRecord {
                id: row.get(0)?,
                wallet_id: row.get(1)?,
                address: row.get(2)?,
                index: row.get::<_, i64>(3)? as u32,
                is_change: row.get::<_, i64>(4)? != 0,
                used: row.get::<_, i64>(5)? != 0,
                created_at: row.get(6)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn max_address_index(
        &self,
        wallet_id: &str,
        is_change: bool,
    ) -> Result<Option<u32>, StorageError> {
        let value: Option<i64> = self
            .conn
            .query_row(
                "SELECT MAX(index_num) FROM addresses WHERE wallet_id = ?1 AND is_change = ?2",
                params![wallet_id, is_change as i64],
                |row| row.get(0),
            )
            .optional()?
            .flatten();

        Ok(value.map(|index| index as u32))
    }

    /// Delete a wallet and related addresses / txs / labels (CASCADE + labels cleanup).
    pub fn delete_wallet(&self, id: &str) -> Result<(), StorageError> {
        self.get_wallet(id)?;
        self.conn.execute(
            "DELETE FROM labels WHERE target_type = 'wallet' AND target_id = ?1",
            params![id],
        )?;
        let changed = self
            .conn
            .execute("DELETE FROM wallets WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(StorageError::WalletNotFound(id.to_string()));
        }
        Ok(())
    }

    /// Delete a workspace and all nested wallets (CASCADE).
    pub fn delete_workspace(&self, id: &str) -> Result<(), StorageError> {
        self.get_workspace(id)?;
        // Clean labels for this workspace and its wallets before CASCADE removes children.
        let wallets = self.list_wallets_for_workspace(id)?;
        for wallet in &wallets {
            self.conn.execute(
                "DELETE FROM labels WHERE target_type = 'wallet' AND target_id = ?1",
                params![wallet.id],
            )?;
        }
        self.conn.execute(
            "DELETE FROM labels WHERE target_type = 'workspace' AND target_id = ?1",
            params![id],
        )?;
        let changed = self
            .conn
            .execute("DELETE FROM workspaces WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(StorageError::WorkspaceNotFound(id.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NewWorkspace;

    #[test]
    fn creates_workspace_and_wallet() {
        let db = Database::open_in_memory().unwrap();
        let now = 1_700_000_000_i64;
        let workspace = db
            .insert_workspace(&NewWorkspace {
                id: "w1".into(),
                name: "Test".into(),
                network: "testnet".into(),
                created_at: now,
            })
            .unwrap();

        assert_eq!(workspace.name, "Test");

        let wallet = db
            .insert_wallet(&NewWallet {
                id: "v1".into(),
                workspace_id: workspace.id.clone(),
                name: "Wallet 1".into(),
                policy_json: "{}".into(),
                descriptor: "tr(...)".into(),
                script_type: "taproot".into(),
                created_at: now,
            })
            .unwrap();

        assert_eq!(wallet.workspace_id, workspace.id);
        assert_eq!(
            db.list_wallets_for_workspace(&workspace.id).unwrap().len(),
            1
        );
    }

    #[test]
    fn delete_workspace_cascades_wallets() {
        let db = Database::open_in_memory().unwrap();
        let now = 1_700_000_000_i64;
        let workspace = db
            .insert_workspace(&NewWorkspace {
                id: "w1".into(),
                name: "Test".into(),
                network: "testnet".into(),
                created_at: now,
            })
            .unwrap();
        db.insert_wallet(&NewWallet {
            id: "v1".into(),
            workspace_id: workspace.id.clone(),
            name: "Wallet 1".into(),
            policy_json: "{}".into(),
            descriptor: "tr(...)".into(),
            script_type: "taproot".into(),
            created_at: now,
        })
        .unwrap();
        db.delete_workspace(&workspace.id).unwrap();
        assert!(db.get_workspace(&workspace.id).is_err());
        assert!(db.get_wallet("v1").is_err());
    }

    #[test]
    fn delete_wallet_removes_row() {
        let db = Database::open_in_memory().unwrap();
        let now = 1_700_000_000_i64;
        let workspace = db
            .insert_workspace(&NewWorkspace {
                id: "w1".into(),
                name: "Test".into(),
                network: "testnet".into(),
                created_at: now,
            })
            .unwrap();
        db.insert_wallet(&NewWallet {
            id: "v1".into(),
            workspace_id: workspace.id.clone(),
            name: "Wallet 1".into(),
            policy_json: "{}".into(),
            descriptor: "tr(...)".into(),
            script_type: "taproot".into(),
            created_at: now,
        })
        .unwrap();
        db.delete_wallet("v1").unwrap();
        assert!(db.get_wallet("v1").is_err());
        assert!(db.get_workspace(&workspace.id).is_ok());
    }

    #[test]
    fn migrates_v1_database_to_v2_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        conn.execute_batch(crate::schema::legacy_v1::MIGRATION_V1)
            .unwrap();
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (1)",
            [],
        )
        .unwrap();

        let now = 1_700_000_000_i64;
        conn.execute(
            "INSERT INTO wallets (id, name, network, created_at, updated_at) VALUES ('w1', 'Old', 'testnet', ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO vaults (id, wallet_id, name, policy_json, descriptor, script_type, created_at)
             VALUES ('v1', 'w1', 'Old Vault', '{}', 'tr(...)', 'taproot', ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO addresses (id, vault_id, address, index_num, is_change, used, created_at)
             VALUES ('a1', 'v1', 'tb1qtest', 0, 0, 0, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO transactions (txid, vault_id, block_height, amount, fee, confirmed, raw_json)
             VALUES ('tx1', 'v1', NULL, NULL, NULL, NULL, NULL)",
            [],
        )
        .unwrap();

        let db = Database { conn };
        db.migrate().unwrap();

        let workspace = db.get_workspace("w1").unwrap();
        assert_eq!(workspace.name, "Old");
        let wallet = db.get_wallet("v1").unwrap();
        assert_eq!(wallet.workspace_id, "w1");
        assert_eq!(db.list_addresses_for_wallet("v1").unwrap().len(), 1);
        assert_eq!(db.get_transaction("tx1", "v1").unwrap().txid, "tx1");
    }
}
