use address_engine::{new_change_address, new_receive_address, DerivedAddress};
use policy_engine::PolicyConfig;
use wallet_core::{Vault, VaultSummary, WalletStore};

use crate::error::VaultError;
use crate::types::{Balance, TxSummary, VaultWithAddress};

/// High-level vault operations over a wallet store.
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
        Ok(self.store.derive_and_save_receive_address(vault_id, index)?)
    }

    pub fn new_change_address_at(
        &self,
        vault_id: &str,
        index: u32,
    ) -> Result<DerivedAddress, VaultError> {
        Ok(self.store.derive_and_save_change_address(vault_id, index)?)
    }

    pub fn derive_receive_address(
        vault: &Vault,
        index: u32,
    ) -> Result<DerivedAddress, VaultError> {
        Ok(new_receive_address(&vault.policy, &vault.descriptor, index)?)
    }

    pub fn derive_change_address(
        vault: &Vault,
        index: u32,
    ) -> Result<DerivedAddress, VaultError> {
        Ok(new_change_address(&vault.policy, &vault.descriptor, index)?)
    }

    pub fn list_addresses(&self, vault_id: &str) -> Result<Vec<DerivedAddress>, VaultError> {
        Ok(self.store.list_addresses(vault_id)?)
    }

    pub fn vault_balance(&self, vault_id: &str) -> Result<Balance, VaultError> {
        self.get_vault(vault_id)?;
        Ok(Balance::zero())
    }

    pub fn vault_history(&self, vault_id: &str) -> Result<Vec<TxSummary>, VaultError> {
        self.get_vault(vault_id)?;
        Ok(Vec::new())
    }
}
