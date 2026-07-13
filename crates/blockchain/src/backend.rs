use crate::error::ChainError;
use crate::query::DescriptorQuery;
use crate::types::{Balance, SyncProgress, SyncResult, TxSummary, Utxo};

pub trait BlockchainBackend: Send + Sync {
    fn sync(
        &self,
        query: &DescriptorQuery,
        progress: &dyn Fn(SyncProgress),
    ) -> Result<SyncResult, ChainError>;

    fn get_balance(&self, query: &DescriptorQuery) -> Result<Balance, ChainError> {
        Ok(self.sync(query, &|_| {})?.balance)
    }

    fn get_history(&self, query: &DescriptorQuery) -> Result<Vec<TxSummary>, ChainError> {
        Ok(self.sync(query, &|_| {})?.history)
    }

    fn get_utxos(&self, query: &DescriptorQuery) -> Result<Vec<Utxo>, ChainError> {
        Ok(self.sync(query, &|_| {})?.utxos)
    }

    fn broadcast(&self, tx_hex: &str) -> Result<String, ChainError>;
}
