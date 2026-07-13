use crate::backend::BlockchainBackend;
use crate::error::ChainError;
use crate::query::DescriptorQuery;
use crate::types::{Balance, SyncProgress, SyncResult, TxSummary, Utxo};

/// Bitcoin Core RPC backend placeholder.
///
/// Full descriptor-based scanning via Core requires `importdescriptors` + rescans.
/// Sprint 4 exposes the type and URL validation; sync is deferred to a follow-up.
pub struct CoreRpcBackend {
    url: String,
}

impl CoreRpcBackend {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl BlockchainBackend for CoreRpcBackend {
    fn sync(
        &self,
        _query: &DescriptorQuery,
        _progress: &dyn Fn(SyncProgress),
    ) -> Result<SyncResult, ChainError> {
        Err(ChainError::Unsupported(
            "Bitcoin Core RPC descriptor sync is not implemented yet; use Esplora or Electrum"
                .into(),
        ))
    }

    fn get_balance(&self, _query: &DescriptorQuery) -> Result<Balance, ChainError> {
        Err(ChainError::Unsupported(
            "Bitcoin Core RPC balance query is not implemented yet".into(),
        ))
    }

    fn get_history(&self, _query: &DescriptorQuery) -> Result<Vec<TxSummary>, ChainError> {
        Err(ChainError::Unsupported(
            "Bitcoin Core RPC history query is not implemented yet".into(),
        ))
    }

    fn get_utxos(&self, _query: &DescriptorQuery) -> Result<Vec<Utxo>, ChainError> {
        Err(ChainError::Unsupported(
            "Bitcoin Core RPC UTXO query is not implemented yet".into(),
        ))
    }

    fn broadcast(&self, _tx_hex: &str) -> Result<String, ChainError> {
        Err(ChainError::Unsupported(
            "Bitcoin Core RPC broadcast is not implemented yet".into(),
        ))
    }
}
