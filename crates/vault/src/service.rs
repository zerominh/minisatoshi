use address_engine::{new_change_address, new_receive_address, DerivedAddress};
use blockchain::{
    Balance, BlockchainBackend, DescriptorQuery, SyncProgress, SyncResult, TxSummary,
};
use policy_engine::PolicyConfig;
use wallet_core::{Vault, VaultSummary, WalletStore};

use crate::error::VaultError;
use crate::types::VaultWithAddress;
pub struct VaultService<'a> {
    store: &'a WalletStore,
}

impl<'a> VaultService<'a> {
    pub fn new(store: &'a WalletStore) -> Self {
        Self { store }
    }

    pub fn create_vault(
        &self,
        wallet_id: &str,
        name: &str,
        policy: PolicyConfig,
    ) -> Result<Vault, VaultError> {
        Ok(self.store.create_vault(wallet_id, name, policy)?)
    }

    pub fn import_descriptor(
        &self,
        wallet_id: &str,
        name: &str,
        descriptor: &str,
        policy: Option<PolicyConfig>,
    ) -> Result<Vault, VaultError> {
        Ok(self
            .store
            .import_descriptor(wallet_id, name, descriptor, policy)?)
    }

    pub fn import_vault_backup(
        &self,
        wallet_id: &str,
        backup: &wallet_core::VaultBackup,
        name_override: Option<&str>,
    ) -> Result<Vault, VaultError> {
        Ok(self
            .store
            .import_vault_backup(wallet_id, backup, name_override)?)
    }

    pub fn import_watch_only_payload(
        &self,
        wallet_id: &str,
        payload: &str,
        name_override: Option<&str>,
    ) -> Result<Vault, VaultError> {
        Ok(self
            .store
            .import_watch_only_payload(wallet_id, payload, name_override)?)
    }

    pub fn export_vault_backup(
        &self,
        vault_id: &str,
    ) -> Result<wallet_core::VaultBackup, VaultError> {
        Ok(self.store.export_vault_backup(vault_id)?)
    }

    pub fn export_bsms(&self, vault_id: &str) -> Result<String, VaultError> {
        Ok(self.store.export_bsms(vault_id)?)
    }

    pub fn create_vault_with_receive_address(
        &self,
        wallet_id: &str,
        name: &str,
        policy: PolicyConfig,
    ) -> Result<VaultWithAddress, VaultError> {
        let vault = self.create_vault(wallet_id, name, policy)?;
        let receive_address = self.new_receive_address(&vault.id)?;
        Ok(VaultWithAddress {
            vault,
            receive_address,
        })
    }

    pub fn list_vaults(&self, wallet_id: &str) -> Result<Vec<VaultSummary>, VaultError> {
        Ok(self.store.list_vaults(wallet_id)?)
    }

    pub fn get_vault(&self, vault_id: &str) -> Result<Vault, VaultError> {
        Ok(self.store.get_vault(vault_id)?)
    }

    pub fn new_receive_address(&self, vault_id: &str) -> Result<DerivedAddress, VaultError> {
        Ok(self.store.next_receive_address(vault_id)?)
    }

    pub fn new_receive_address_at(
        &self,
        vault_id: &str,
        index: u32,
    ) -> Result<DerivedAddress, VaultError> {
        Ok(self
            .store
            .derive_and_save_receive_address(vault_id, index)?)
    }

    pub fn new_change_address_at(
        &self,
        vault_id: &str,
        index: u32,
    ) -> Result<DerivedAddress, VaultError> {
        Ok(self.store.derive_and_save_change_address(vault_id, index)?)
    }

    pub fn derive_receive_address(vault: &Vault, index: u32) -> Result<DerivedAddress, VaultError> {
        Ok(new_receive_address(
            &vault.policy,
            &vault.descriptor,
            index,
        )?)
    }

    pub fn derive_change_address(vault: &Vault, index: u32) -> Result<DerivedAddress, VaultError> {
        Ok(new_change_address(&vault.policy, &vault.descriptor, index)?)
    }

    pub fn list_addresses(&self, vault_id: &str) -> Result<Vec<DerivedAddress>, VaultError> {
        Ok(self.store.list_addresses(vault_id)?)
    }

    pub fn descriptor_query(&self, vault_id: &str) -> Result<DescriptorQuery, VaultError> {
        let vault = self.get_vault(vault_id)?;
        Ok(DescriptorQuery::new(vault.policy, vault.descriptor))
    }

    pub fn sync_vault(
        &self,
        vault_id: &str,
        backend: &dyn BlockchainBackend,
        progress: &dyn Fn(SyncProgress),
    ) -> Result<SyncResult, VaultError> {
        let query = self.descriptor_query(vault_id)?;
        Ok(backend.sync(&query, progress)?)
    }

    pub fn vault_balance(
        &self,
        vault_id: &str,
        backend: &dyn BlockchainBackend,
    ) -> Result<Balance, VaultError> {
        let query = self.descriptor_query(vault_id)?;
        Ok(backend.get_balance(&query)?)
    }

    pub fn vault_history(
        &self,
        vault_id: &str,
        backend: &dyn BlockchainBackend,
    ) -> Result<Vec<TxSummary>, VaultError> {
        let query = self.descriptor_query(vault_id)?;
        Ok(backend.get_history(&query)?)
    }
}
