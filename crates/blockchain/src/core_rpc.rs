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

#[cfg(test)]
mod tests {
    use policy_engine::{
        abc_preset, test_vectors::TEST_FP, test_vectors::TEST_XPUB_A, test_vectors::TEST_XPUB_B,
        test_vectors::TEST_XPUB_C, KeyConfig, KeyRole, NetworkName,
    };

    use super::*;

    #[test]
    fn core_rpc_sync_returns_unsupported() {
        let keys = [
            KeyConfig {
                id: "A".into(),
                role: KeyRole::Investor,
                xpub: TEST_XPUB_A.into(),
                fingerprint: "78412e3a".into(),
                origin_path: None,
            },
            KeyConfig {
                id: "B".into(),
                role: KeyRole::Manager,
                xpub: TEST_XPUB_B.into(),
                fingerprint: TEST_FP.into(),
                origin_path: None,
            },
            KeyConfig {
                id: "C".into(),
                role: KeyRole::Recovery,
                xpub: TEST_XPUB_C.into(),
                fingerprint: TEST_FP.into(),
                origin_path: None,
            },
        ];
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let backend = CoreRpcBackend::new("http://127.0.0.1:18332");
        let err = backend
            .sync(
                &DescriptorQuery::new(policy, "tr(deadbeef)#checksum"),
                &|_| {},
            )
            .unwrap_err();
        assert!(matches!(err, ChainError::Unsupported(_)));
    }
}
