//! Vault orchestration for Minisatoshi.

mod error;
mod service;
mod types;

pub use error::VaultError;
pub use service::VaultService;
pub use types::VaultWithAddress;

pub use blockchain::{
    export_watch_only_wallet, default_server_presets, BackendKind, Balance, BlockchainBackend,
    ChainError, DescriptorQuery, EsploraBackend, ServerPreset, SparrowWalletExport, SyncProgress,
    SyncResult, TxSummary, Utxo,
};

#[cfg(test)]
mod tests {
    use policy_engine::{
        abc_preset, test_vectors::TEST_FP, test_vectors::TEST_XPUB_A, test_vectors::TEST_XPUB_B,
        test_vectors::TEST_XPUB_C, KeyConfig, KeyRole, NetworkName,
    };
    use wallet_core::WalletStore;

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
    fn policy_to_first_taproot_receive_address() {
        let dir = tempfile::tempdir().unwrap();
        let store = WalletStore::open(dir.path().join("wallet.db")).unwrap();
        let service = VaultService::new(&store);

        let wallet = store.create_wallet("Family", NetworkName::Testnet).unwrap();
        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );

        let result = service
            .create_vault_with_receive_address(&wallet.id, "ABC", policy)
            .unwrap();

        assert!(result.receive_address.address.starts_with("tb1"));
        assert_eq!(result.receive_address.index, 0);
        assert!(!result.receive_address.is_change);

        let listed = service.list_addresses(&result.vault.id).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].address, result.receive_address.address);

        let export = export_watch_only_wallet(&result.vault).unwrap();
        assert!(export.descriptor.contains('#'));

        let presets = default_server_presets(NetworkName::Testnet);
        assert!(!presets.is_empty());
    }

    #[test]
    fn vault_sync_via_esplora_mock() {
        use httpmock::prelude::*;
        use blockchain::EsploraBackend;

        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path_matches(r"^/address/[^/]+$");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"chain_stats":{"tx_count":0},"mempool_stats":{"tx_count":0}}"#);
        });
        server.mock(|when, then| {
            when.method(GET).path_matches(r"^/address/[^/]+/utxo$");
            then.status(200)
                .header("content-type", "application/json")
                .body("[]");
        });
        server.mock(|when, then| {
            when.method(GET).path_matches(r"^/address/[^/]+/txs$");
            then.status(200)
                .header("content-type", "application/json")
                .body("[]");
        });

        let dir = tempfile::tempdir().unwrap();
        let store = WalletStore::open(dir.path().join("wallet.db")).unwrap();
        let service = VaultService::new(&store);
        let wallet = store.create_wallet("Sync", NetworkName::Testnet).unwrap();
        let policy = abc_preset(
            sample_keys()[0].clone(),
            sample_keys()[1].clone(),
            sample_keys()[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let vault = service
            .create_vault(&wallet.id, "ABC", policy)
            .unwrap();

        let backend = EsploraBackend::new(server.base_url())
            .unwrap()
            .with_gap_limit(2);
        let balance = service.vault_balance(&vault.id, &backend).unwrap();
        assert_eq!(balance.confirmed_sats, 0);
        assert!(service.vault_history(&vault.id, &backend).unwrap().is_empty());
    }
}
