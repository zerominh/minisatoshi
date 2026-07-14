//! Full vault lifecycle: wallet → vault → address → PSBT → Sparrow export.

use blockchain::Utxo;
use policy_engine::{
    abc_preset, test_vectors::TEST_FP, test_vectors::TEST_XPUB_A, test_vectors::TEST_XPUB_B,
    test_vectors::TEST_XPUB_C, KeyConfig, KeyRole, NetworkName,
};
use psbt_engine::{
    create_psbt, export_psbt, CreatePsbtOptions, ExportFormat, FeeRate, PsbtRecipient, SpendingUtxo,
};
use vault::{export_watch_only_wallet, VaultService};
use wallet_core::WalletStore;

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
fn vault_lifecycle_create_address_psbt_export() {
    let dir = tempfile::tempdir().unwrap();
    let store = WalletStore::open(dir.path().join("wallet.db")).unwrap();
    let service = VaultService::new(&store);

    let wallet = store
        .create_wallet("Lifecycle", NetworkName::Testnet)
        .unwrap();
    let keys = sample_keys();
    let policy = abc_preset(
        keys[0].clone(),
        keys[1].clone(),
        keys[2].clone(),
        4,
        NetworkName::Testnet,
    );

    let created = service
        .create_vault_with_receive_address(&wallet.id, "ABC", policy)
        .unwrap();
    assert!(created.receive_address.address.starts_with("tb1"));
    assert!(created.vault.descriptor.starts_with("tr("));
    assert!(created.vault.descriptor.contains('#'));

    let receive = created.receive_address.clone();
    let change = service.new_change_address_at(&created.vault.id, 0).unwrap();
    assert!(change.is_change);

    let payment = service
        .new_receive_address_at(&created.vault.id, 1)
        .unwrap();

    let psbt = create_psbt(
        &created.vault,
        &[PsbtRecipient {
            address: payment.address.clone(),
            amount_sats: 50_000,
        }],
        FeeRate::new(2),
        &[SpendingUtxo::new(
            Utxo {
                txid: "aa".repeat(32),
                vout: 0,
                value_sats: 100_000,
                address: receive.address.clone(),
                confirmed: true,
                block_height: Some(1),
                derivation_index: 0,
                is_change: false,
            },
            0,
            false,
        )],
        CreatePsbtOptions {
            input_sequence: None,
            change_index: Some(0),
        },
    )
    .unwrap();

    assert_eq!(psbt.inputs.len(), 1);
    assert!(!psbt.outputs.is_empty());

    let base64 = String::from_utf8(export_psbt(&psbt, ExportFormat::Base64).unwrap()).unwrap();
    assert!(base64.starts_with("cHNidP8"));

    let file_bytes = export_psbt(&psbt, ExportFormat::File).unwrap();
    assert!(!file_bytes.is_empty());
    std::fs::write(dir.path().join("unsigned.psbt"), &file_bytes).unwrap();

    let sparrow = export_watch_only_wallet(&created.vault).unwrap();
    assert_eq!(sparrow.network, NetworkName::Testnet);
    assert!(sparrow.descriptor.contains('#'));
}
