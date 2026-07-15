use address_engine::{new_change_address, new_receive_address, DerivedAddress};
use blockchain::{
    Balance, BlockchainBackend, DescriptorQuery, SyncProgress, SyncResult, TxSummary,
};
use policy_engine::PolicyConfig;
use wallet_core::{Wallet, WalletStore, WalletSummary};

use crate::error::VaultError;
use crate::types::WalletWithAddress;
pub struct WalletService<'a> {
    store: &'a WalletStore,
}

impl<'a> WalletService<'a> {
    pub fn new(store: &'a WalletStore) -> Self {
        Self { store }
    }

    pub fn create_wallet(
        &self,
        workspace_id: &str,
        name: &str,
        policy: PolicyConfig,
    ) -> Result<Wallet, VaultError> {
        Ok(self.store.create_wallet(workspace_id, name, policy)?)
    }

    pub fn import_descriptor(
        &self,
        workspace_id: &str,
        name: &str,
        descriptor: &str,
        policy: Option<PolicyConfig>,
    ) -> Result<Wallet, VaultError> {
        Ok(self
            .store
            .import_descriptor(workspace_id, name, descriptor, policy)?)
    }

    pub fn import_wallet_backup(
        &self,
        workspace_id: &str,
        backup: &wallet_core::WalletBackup,
        name_override: Option<&str>,
    ) -> Result<Wallet, VaultError> {
        Ok(self
            .store
            .import_wallet_backup(workspace_id, backup, name_override)?)
    }

    pub fn import_watch_only_payload(
        &self,
        workspace_id: &str,
        payload: &str,
        name_override: Option<&str>,
    ) -> Result<Wallet, VaultError> {
        Ok(self
            .store
            .import_watch_only_payload(workspace_id, payload, name_override)?)
    }

    pub fn export_wallet_backup(
        &self,
        wallet_id: &str,
    ) -> Result<wallet_core::WalletBackup, VaultError> {
        Ok(self.store.export_wallet_backup(wallet_id)?)
    }

    pub fn export_bsms(&self, wallet_id: &str) -> Result<String, VaultError> {
        Ok(self.store.export_bsms(wallet_id)?)
    }

    pub fn create_wallet_with_receive_address(
        &self,
        workspace_id: &str,
        name: &str,
        policy: PolicyConfig,
    ) -> Result<WalletWithAddress, VaultError> {
        let wallet = self.create_wallet(workspace_id, name, policy)?;
        let receive_address = self.new_receive_address(&wallet.id)?;
        Ok(WalletWithAddress {
            wallet,
            receive_address,
        })
    }

    pub fn list_wallets(&self, workspace_id: &str) -> Result<Vec<WalletSummary>, VaultError> {
        Ok(self.store.list_wallets(workspace_id)?)
    }

    pub fn delete_wallet(&self, wallet_id: &str) -> Result<(), VaultError> {
        Ok(self.store.delete_wallet(wallet_id)?)
    }

    pub fn rename_wallet(&self, wallet_id: &str, name: &str) -> Result<Wallet, VaultError> {
        Ok(self.store.rename_wallet(wallet_id, name)?)
    }

    pub fn delete_workspace(&self, workspace_id: &str) -> Result<(), VaultError> {
        Ok(self.store.delete_workspace(workspace_id)?)
    }

    pub fn get_wallet(&self, wallet_id: &str) -> Result<Wallet, VaultError> {
        Ok(self.store.get_wallet(wallet_id)?)
    }

    pub fn new_receive_address(&self, wallet_id: &str) -> Result<DerivedAddress, VaultError> {
        Ok(self.store.next_receive_address(wallet_id)?)
    }

    pub fn new_receive_address_at(
        &self,
        wallet_id: &str,
        index: u32,
    ) -> Result<DerivedAddress, VaultError> {
        Ok(self
            .store
            .derive_and_save_receive_address(wallet_id, index)?)
    }

    pub fn new_change_address_at(
        &self,
        wallet_id: &str,
        index: u32,
    ) -> Result<DerivedAddress, VaultError> {
        Ok(self.store.derive_and_save_change_address(wallet_id, index)?)
    }

    pub fn derive_receive_address(
        wallet: &Wallet,
        index: u32,
    ) -> Result<DerivedAddress, VaultError> {
        Ok(new_receive_address(
            &wallet.policy,
            &wallet.descriptor,
            index,
        )?)
    }

    pub fn derive_change_address(wallet: &Wallet, index: u32) -> Result<DerivedAddress, VaultError> {
        Ok(new_change_address(&wallet.policy, &wallet.descriptor, index)?)
    }

    pub fn list_addresses(&self, wallet_id: &str) -> Result<Vec<DerivedAddress>, VaultError> {
        Ok(self.store.list_addresses(wallet_id)?)
    }

    pub fn descriptor_query(&self, wallet_id: &str) -> Result<DescriptorQuery, VaultError> {
        let wallet = self.get_wallet(wallet_id)?;
        Ok(DescriptorQuery::new(wallet.policy, wallet.descriptor))
    }

    pub fn sync_wallet(
        &self,
        wallet_id: &str,
        backend: &dyn BlockchainBackend,
        progress: &dyn Fn(SyncProgress),
    ) -> Result<SyncResult, VaultError> {
        let query = self.descriptor_query(wallet_id)?;
        Ok(backend.sync(&query, progress)?)
    }

    pub fn wallet_balance(
        &self,
        wallet_id: &str,
        backend: &dyn BlockchainBackend,
    ) -> Result<Balance, VaultError> {
        let query = self.descriptor_query(wallet_id)?;
        Ok(backend.get_balance(&query)?)
    }

    pub fn wallet_history(
        &self,
        wallet_id: &str,
        backend: &dyn BlockchainBackend,
    ) -> Result<Vec<TxSummary>, VaultError> {
        let query = self.descriptor_query(wallet_id)?;
        Ok(backend.get_history(&query)?)
    }
}
