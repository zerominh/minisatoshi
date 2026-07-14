use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::StorageError;
use crate::models::{
    AddressRecord, LabelRecord, NewAddress, NewLabel, NewTransaction, NewVault, NewWallet,
    TransactionRecord, VaultRecord, WalletRecord,
};
use crate::schema::{MIGRATION_V1, SCHEMA_VERSION};

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
        self.conn.execute_batch(MIGRATION_V1)?;

        let version: Option<u32> = self
            .conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
            .optional()?;

        if version.is_none() {
            self.conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                params![SCHEMA_VERSION],
            )?;
        }

        Ok(())
    }

    pub fn insert_wallet(&self, wallet: &NewWallet) -> Result<WalletRecord, StorageError> {
        self.conn.execute(
            "INSERT INTO wallets (id, name, network, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![wallet.id, wallet.name, wallet.network, wallet.created_at,],
        )?;

        self.get_wallet(&wallet.id)
    }

    pub fn get_wallet(&self, id: &str) -> Result<WalletRecord, StorageError> {
        self.conn
            .query_row(
                "SELECT id, name, network, created_at, updated_at FROM wallets WHERE id = ?1",
                params![id],
                |row| {
                    Ok(WalletRecord {
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
                    StorageError::WalletNotFound(id.to_string())
                }
                other => StorageError::Database(other),
            })
    }

    pub fn list_wallets(&self) -> Result<Vec<WalletRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, network, created_at, updated_at
             FROM wallets ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(WalletRecord {
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

    pub fn touch_wallet(&self, id: &str, updated_at: i64) -> Result<(), StorageError> {
        let changed = self.conn.execute(
            "UPDATE wallets SET updated_at = ?2 WHERE id = ?1",
            params![id, updated_at],
        )?;
        if changed == 0 {
            return Err(StorageError::WalletNotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn insert_vault(&self, vault: &NewVault) -> Result<VaultRecord, StorageError> {
        self.conn.execute(
            "INSERT INTO vaults (id, wallet_id, name, policy_json, descriptor, script_type, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                vault.id,
                vault.wallet_id,
                vault.name,
                vault.policy_json,
                vault.descriptor,
                vault.script_type,
                vault.created_at,
            ],
        )?;

        self.get_vault(&vault.id)
    }

    pub fn get_vault(&self, id: &str) -> Result<VaultRecord, StorageError> {
        self.conn
            .query_row(
                "SELECT id, wallet_id, name, policy_json, descriptor, script_type, created_at
                 FROM vaults WHERE id = ?1",
                params![id],
                |row| {
                    Ok(VaultRecord {
                        id: row.get(0)?,
                        wallet_id: row.get(1)?,
                        name: row.get(2)?,
                        policy_json: row.get(3)?,
                        descriptor: row.get(4)?,
                        script_type: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => StorageError::VaultNotFound(id.to_string()),
                other => StorageError::Database(other),
            })
    }

    pub fn list_vaults_for_wallet(
        &self,
        wallet_id: &str,
    ) -> Result<Vec<VaultRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, wallet_id, name, policy_json, descriptor, script_type, created_at
             FROM vaults WHERE wallet_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![wallet_id], |row| {
            Ok(VaultRecord {
                id: row.get(0)?,
                wallet_id: row.get(1)?,
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
            "INSERT INTO addresses (id, vault_id, address, index_num, is_change, used, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
            params![
                address.id,
                address.vault_id,
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
                "SELECT id, vault_id, address, index_num, is_change, used, created_at
                 FROM addresses WHERE id = ?1",
                params![id],
                |row| {
                    Ok(AddressRecord {
                        id: row.get(0)?,
                        vault_id: row.get(1)?,
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
            "INSERT INTO transactions (txid, vault_id, block_height, amount, fee, confirmed, raw_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                tx.txid,
                tx.vault_id,
                tx.block_height,
                tx.amount,
                tx.fee,
                tx.confirmed.map(|v| v as i64),
                tx.raw_json,
            ],
        )?;

        self.get_transaction(&tx.txid, &tx.vault_id)
    }

    pub fn get_transaction(
        &self,
        txid: &str,
        vault_id: &str,
    ) -> Result<TransactionRecord, StorageError> {
        self.conn
            .query_row(
                "SELECT txid, vault_id, block_height, amount, fee, confirmed, raw_json
             FROM transactions WHERE txid = ?1 AND vault_id = ?2",
                params![txid, vault_id],
                |row| {
                    Ok(TransactionRecord {
                        txid: row.get(0)?,
                        vault_id: row.get(1)?,
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

    pub fn list_addresses_for_vault(
        &self,
        vault_id: &str,
    ) -> Result<Vec<AddressRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, vault_id, address, index_num, is_change, used, created_at
             FROM addresses WHERE vault_id = ?1 ORDER BY is_change ASC, index_num ASC",
        )?;
        let rows = stmt.query_map(params![vault_id], |row| {
            Ok(AddressRecord {
                id: row.get(0)?,
                vault_id: row.get(1)?,
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
        vault_id: &str,
        is_change: bool,
    ) -> Result<Option<u32>, StorageError> {
        let value: Option<i64> = self
            .conn
            .query_row(
                "SELECT MAX(index_num) FROM addresses WHERE vault_id = ?1 AND is_change = ?2",
                params![vault_id, is_change as i64],
                |row| row.get(0),
            )
            .optional()?
            .flatten();

        Ok(value.map(|index| index as u32))
    }

    /// Delete a vault and related addresses / txs / labels (CASCADE + labels cleanup).
    pub fn delete_vault(&self, id: &str) -> Result<(), StorageError> {
        self.get_vault(id)?;
        self.conn.execute(
            "DELETE FROM labels WHERE target_type = 'vault' AND target_id = ?1",
            params![id],
        )?;
        let changed = self
            .conn
            .execute("DELETE FROM vaults WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(StorageError::VaultNotFound(id.to_string()));
        }
        Ok(())
    }

    /// Delete a wallet and all nested vaults (CASCADE).
    pub fn delete_wallet(&self, id: &str) -> Result<(), StorageError> {
        self.get_wallet(id)?;
        // Clean labels for this wallet and its vaults before CASCADE removes children.
        let vaults = self.list_vaults_for_wallet(id)?;
        for vault in &vaults {
            self.conn.execute(
                "DELETE FROM labels WHERE target_type = 'vault' AND target_id = ?1",
                params![vault.id],
            )?;
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NewWallet;

    #[test]
    fn creates_wallet_and_vault() {
        let db = Database::open_in_memory().unwrap();
        let now = 1_700_000_000_i64;
        let wallet = db
            .insert_wallet(&NewWallet {
                id: "w1".into(),
                name: "Test".into(),
                network: "testnet".into(),
                created_at: now,
            })
            .unwrap();

        assert_eq!(wallet.name, "Test");

        let vault = db
            .insert_vault(&NewVault {
                id: "v1".into(),
                wallet_id: wallet.id.clone(),
                name: "Vault 1".into(),
                policy_json: "{}".into(),
                descriptor: "tr(...)".into(),
                script_type: "taproot".into(),
                created_at: now,
            })
            .unwrap();

        assert_eq!(vault.wallet_id, wallet.id);
        assert_eq!(db.list_vaults_for_wallet(&wallet.id).unwrap().len(), 1);
    }

    #[test]
    fn delete_wallet_cascades_vaults() {
        let db = Database::open_in_memory().unwrap();
        let now = 1_700_000_000_i64;
        let wallet = db
            .insert_wallet(&NewWallet {
                id: "w1".into(),
                name: "Test".into(),
                network: "testnet".into(),
                created_at: now,
            })
            .unwrap();
        db.insert_vault(&NewVault {
            id: "v1".into(),
            wallet_id: wallet.id.clone(),
            name: "Vault 1".into(),
            policy_json: "{}".into(),
            descriptor: "tr(...)".into(),
            script_type: "taproot".into(),
            created_at: now,
        })
        .unwrap();
        db.delete_wallet(&wallet.id).unwrap();
        assert!(db.get_wallet(&wallet.id).is_err());
        assert!(db.get_vault("v1").is_err());
    }

    #[test]
    fn delete_vault_removes_row() {
        let db = Database::open_in_memory().unwrap();
        let now = 1_700_000_000_i64;
        let wallet = db
            .insert_wallet(&NewWallet {
                id: "w1".into(),
                name: "Test".into(),
                network: "testnet".into(),
                created_at: now,
            })
            .unwrap();
        db.insert_vault(&NewVault {
            id: "v1".into(),
            wallet_id: wallet.id.clone(),
            name: "Vault 1".into(),
            policy_json: "{}".into(),
            descriptor: "tr(...)".into(),
            script_type: "taproot".into(),
            created_at: now,
        })
        .unwrap();
        db.delete_vault("v1").unwrap();
        assert!(db.get_vault("v1").is_err());
        assert!(db.get_wallet(&wallet.id).is_ok());
    }
}
