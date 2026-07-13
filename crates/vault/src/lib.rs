//! Vault orchestration for Minisatoshi.

mod error;
mod service;
mod types;

pub use error::VaultError;
pub use service::VaultService;
pub use types::{Balance, TxSummary, VaultWithAddress};

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

        let balance = service.vault_balance(&result.vault.id).unwrap();
        assert_eq!(balance.confirmed_sats, 0);
        assert!(service.vault_history(&result.vault.id).unwrap().is_empty());
    }
}
