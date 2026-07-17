//! PSBT creation, signing, and export for Minisatoshi.

mod broadcast;
mod combine;
mod create;
mod descriptor;
mod enrich;
mod error;
mod export;
mod finalize;
mod hw_sign;
mod sign;
mod status;
#[cfg(test)]
mod test_keys;
mod types;

pub use broadcast::broadcast_psbt;
pub use combine::combine_psbt;
pub use create::create_psbt;
pub use enrich::populate_global_xpubs;
pub use error::PsbtError;
pub use hw_sign::{hw_sign_made_progress, signature_snapshot, SignatureSnapshot};
pub use export::{export_psbt, import_psbt_base64};
pub use finalize::{extract_transaction, finalize_psbt, transaction_bytes, transaction_hex};
pub use sign::{sign_psbt, Signer, SoftwareSigner};
pub use status::{
    analyze_signing_status, signed_fingerprints, KeySignStatus, KeyStatus, PathStatus,
    SigningStatus,
};
pub use types::{
    CreatePsbtOptions, ExportFormat, FeeRate, Psbt, PsbtRecipient, SignProgress, SpendingUtxo,
    WalletPsbt,
};

#[cfg(test)]
mod integration_tests {
    use std::str::FromStr;

    use miniscript::descriptor::{DescriptorSecretKey, KeyMap};
    use policy_engine::{
        NetworkName, PolicyConfig, PolicyExpression, ScriptTypeName, POLICY_SCHEMA_VERSION,
    };
    use wallet_core::Wallet;

    use crate::test_keys::{key_config_from_tprv, TEST_TPRV_A, TEST_TPRV_B};

    use super::*;

    fn regtest_wallet() -> Wallet {
        let policy = PolicyConfig {
            version: POLICY_SCHEMA_VERSION,
            network: NetworkName::Regtest,
            script_type: ScriptTypeName::Taproot,
            keys: [
                key_config_from_tprv("A", policy_engine::KeyRole::Investor, TEST_TPRV_A),
                key_config_from_tprv("B", policy_engine::KeyRole::Manager, TEST_TPRV_B),
            ]
            .into(),
            policy: PolicyExpression {
                primary: "A && B".into(),
                fallback: None,
                fallbacks: vec![],
            },
        };
        let descriptor = descriptor_engine::compile_descriptor_from_config(&policy).unwrap();
        Wallet {
            id: "v1".into(),
            workspace_id: "w1".into(),
            name: "2of2".into(),
            policy,
            descriptor,
            script_type: ScriptTypeName::Taproot,
            created_at: 0,
        }
    }

    #[test]
    fn two_of_two_sign_combine_finalize() {
        let wallet = regtest_wallet();
        let receive =
            address_engine::new_receive_address(&wallet.policy, &wallet.descriptor, 0).unwrap();
        let recipient =
            address_engine::new_receive_address(&wallet.policy, &wallet.descriptor, 1).unwrap();

        let mut psbt = create_psbt(
            &wallet,
            &[PsbtRecipient {
                address: recipient.address,
                amount_sats: 50_000,
            }],
            FeeRate::new(2),
            &[SpendingUtxo::new(
                blockchain::Utxo {
                    txid: "cc".repeat(32),
                    vout: 0,
                    value_sats: 100_000,
                    address: receive.address,
                    confirmed: true,
                    block_height: None,
                    derivation_index: 0,
                    is_change: false,
                },
                0,
                false,
            )],
            CreatePsbtOptions::default(),
        )
        .unwrap();

        let secp = bitcoin::secp256k1::Secp256k1::new();
        let sk_a = DescriptorSecretKey::from_str(TEST_TPRV_A).unwrap();

        let mut psbt_a = psbt.clone();
        let mut psbt_b = psbt.clone();
        sign_psbt(&mut psbt_a, &SoftwareSigner::from_secret(sk_a)).unwrap();
        sign_psbt(
            &mut psbt_b,
            &SoftwareSigner::from_secret(DescriptorSecretKey::from_str(TEST_TPRV_B).unwrap()),
        )
        .unwrap();

        psbt = combine_psbt(psbt_a, psbt_b).unwrap();

        let mut key_map = KeyMap::new();
        key_map
            .insert(&secp, DescriptorSecretKey::from_str(TEST_TPRV_A).unwrap())
            .unwrap();
        key_map
            .insert(&secp, DescriptorSecretKey::from_str(TEST_TPRV_B).unwrap())
            .unwrap();
        sign_psbt(&mut psbt, &SoftwareSigner::from_key_map(key_map)).unwrap();

        let tx = finalize_psbt(&mut psbt).unwrap();
        assert!(!tx.input.is_empty());
        assert!(!tx.output.is_empty());

        let exported = export_psbt(&psbt, ExportFormat::Base64).unwrap();
        assert!(exported.starts_with(b"cHNidP8"));
    }

    /// Hot-wallet style secrets: `[fp/86'/coin'/0']account_xprv/<0;1>/*` — mirrors hot-keystore.
    fn hot_style_secret(mnemonic: &str, network: NetworkName) -> (policy_engine::KeyConfig, String) {
        use bitcoin::bip32::{ChildNumber, DerivationPath, Fingerprint, Xpriv, Xpub};
        use bitcoin::key::Secp256k1;
        use bip39::Mnemonic;

        let mnemonic = Mnemonic::parse_normalized(mnemonic).unwrap();
        let seed = mnemonic.to_seed("");
        let secp = Secp256k1::new();
        let btc_net = network.to_bitcoin_network();
        let master = Xpriv::new_master(btc_net, &seed).unwrap();
        let master_fp: Fingerprint = master.fingerprint(&secp);
        let ct = if network == NetworkName::Mainnet { 0 } else { 1 };
        let account_path = DerivationPath::from(vec![
            ChildNumber::from_hardened_idx(86).unwrap(),
            ChildNumber::from_hardened_idx(ct).unwrap(),
            ChildNumber::from_hardened_idx(0).unwrap(),
        ]);
        let account = master.derive_priv(&secp, &account_path).unwrap();
        let account_xpub = Xpub::from_priv(&secp, &account);
        let origin_path = account_path.to_string();
        let fp_hex = format!("{master_fp:08x}");
        let secret = format!("[{fp_hex}]{master}/{origin_path}/<0;1>/*");
        let key = policy_engine::KeyConfig {
            id: "X".into(),
            role: policy_engine::KeyRole::Other,
            xpub: account_xpub.to_string(),
            fingerprint: fp_hex,
            origin_path: Some(origin_path),
        };
        (key, secret)
    }

    #[test]
    fn abc_hot_style_multipath_sign_finalize() {
        use bitcoin::hashes::Hash;
        use bitcoin::secp256k1::{Message, Secp256k1};
        use bitcoin::sighash::{Prevouts, SighashCache};
        use policy_engine::{FallbackPolicy, KeyRole};

        let (mut key_a, secret_a) = hot_style_secret(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            NetworkName::Regtest,
        );
        key_a.id = "A".into();
        key_a.role = KeyRole::Investor;
        let (mut key_b, secret_b) = hot_style_secret(
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
            NetworkName::Regtest,
        );
        key_b.id = "B".into();
        key_b.role = KeyRole::Manager;
        let (mut key_c, _secret_c) = hot_style_secret(
            "legal winner thank year wave sausage worth useful legal winner thank yellow",
            NetworkName::Regtest,
        );
        key_c.id = "C".into();
        key_c.role = KeyRole::Recovery;

        let policy = PolicyConfig {
            version: POLICY_SCHEMA_VERSION,
            network: NetworkName::Regtest,
            script_type: ScriptTypeName::Taproot,
            keys: vec![key_a, key_b, key_c],
            policy: PolicyExpression {
                primary: "(A && B) || (A && C)".into(),
                fallback: Some(FallbackPolicy {
                    after: "1y".into(),
                    allow: "A".into(),
                }),
                fallbacks: vec![],
            },
        };
        let descriptor = descriptor_engine::compile_descriptor_from_config(&policy).unwrap();
        let wallet = Wallet {
            id: "v1".into(),
            workspace_id: "w1".into(),
            name: "abc".into(),
            policy,
            descriptor,
            script_type: ScriptTypeName::Taproot,
            created_at: 0,
        };
        let receive =
            address_engine::new_receive_address(&wallet.policy, &wallet.descriptor, 2).unwrap();
        let recipient =
            address_engine::new_receive_address(&wallet.policy, &wallet.descriptor, 1).unwrap();
        let mut psbt = create_psbt(
            &wallet,
            &[PsbtRecipient {
                address: recipient.address,
                amount_sats: 50_000,
            }],
            FeeRate::new(2),
            &[SpendingUtxo::new(
                blockchain::Utxo {
                    txid: "aa".repeat(32),
                    vout: 1,
                    value_sats: 125_353,
                    address: receive.address,
                    confirmed: true,
                    block_height: None,
                    derivation_index: 2,
                    is_change: false,
                },
                2,
                false,
            )],
            CreatePsbtOptions::default(),
        )
        .unwrap();

        sign_psbt(
            &mut psbt,
            &SoftwareSigner::from_secret(DescriptorSecretKey::from_str(&secret_a).unwrap()),
        )
        .unwrap();
        sign_psbt(
            &mut psbt,
            &SoftwareSigner::from_secret(DescriptorSecretKey::from_str(&secret_b).unwrap()),
        )
        .unwrap();

        let secp = Secp256k1::verification_only();
        let utxo = psbt.inputs[0].witness_utxo.as_ref().unwrap().clone();
        let mut cache = SighashCache::new(&psbt.unsigned_tx);
        assert!(
            !psbt.inputs[0].tap_script_sigs.is_empty(),
            "expected tap script signatures"
        );
        for ((pk, leaf), sig) in &psbt.inputs[0].tap_script_sigs {
            let msg = cache
                .taproot_script_spend_signature_hash(
                    0,
                    &Prevouts::All(std::slice::from_ref(&utxo)),
                    *leaf,
                    sig.sighash_type,
                )
                .unwrap();
            secp.verify_schnorr(
                &sig.signature,
                &Message::from_digest(msg.to_byte_array()),
                pk,
            )
            .unwrap_or_else(|e| panic!("invalid hot-style sig for {pk} leaf {leaf}: {e}"));
        }

        finalize_psbt(&mut psbt).expect("finalize abc hot-style");
    }

    /// BIP-86 hot singlesig: receive address matches BIP vector; create → sign (key-path) → finalize.
    #[test]
    fn bip86_hot_singlesig_receive_send_finalize() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let (mut key, secret) = hot_style_secret(mnemonic, NetworkName::Mainnet);
        key.id = "A".into();

        let policy = PolicyConfig {
            version: POLICY_SCHEMA_VERSION,
            network: NetworkName::Mainnet,
            script_type: ScriptTypeName::Taproot,
            keys: vec![key],
            policy: PolicyExpression {
                primary: "A".into(),
                fallback: None,
                fallbacks: vec![],
            },
        };
        let descriptor = descriptor_engine::compile_descriptor_from_config(&policy).unwrap();
        assert!(
            !descriptor.contains(descriptor_engine::NUMS_UNSPENDABLE_KEY),
            "BIP-86 must be key-path tr(xpub), not NUMS script tree"
        );
        assert!(!descriptor.contains('{'), "no script tree for singlesig");

        let wallet = Wallet {
            id: "v-hot".into(),
            workspace_id: "w1".into(),
            name: "bip86".into(),
            policy,
            descriptor,
            script_type: ScriptTypeName::Taproot,
            created_at: 0,
        };

        let receive0 =
            address_engine::new_receive_address(&wallet.policy, &wallet.descriptor, 0).unwrap();
        assert_eq!(
            receive0.address,
            "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr",
            "BIP-86 receive m/86'/0'/0'/0/0"
        );
        let receive1 =
            address_engine::new_receive_address(&wallet.policy, &wallet.descriptor, 1).unwrap();
        assert_eq!(
            receive1.address,
            "bc1p4qhjn9zdvkux4e44uhx8tc55attvtyu358kutcqkudyccelu0was9fqzwh",
            "BIP-86 receive m/86'/0'/0'/0/1"
        );
        let change0 =
            address_engine::new_change_address(&wallet.policy, &wallet.descriptor, 0).unwrap();
        assert_eq!(
            change0.address,
            "bc1p3qkhfews2uk44qtvauqyr2ttdsw7svhkl9nkm9s9c3x4ax5h60wqwruhk7",
            "BIP-86 change m/86'/0'/0'/1/0"
        );

        // Spend from receive0 → receive1 (simulates Send), sign with hot secret, finalize.
        let mut psbt = create_psbt(
            &wallet,
            &[PsbtRecipient {
                address: receive1.address.clone(),
                amount_sats: 50_000,
            }],
            FeeRate::new(1),
            &[SpendingUtxo::new(
                blockchain::Utxo {
                    txid: "11".repeat(32),
                    vout: 0,
                    value_sats: 100_000,
                    address: receive0.address.clone(),
                    confirmed: true,
                    block_height: Some(1),
                    derivation_index: 0,
                    is_change: false,
                },
                0,
                false,
            )],
            CreatePsbtOptions::default(),
        )
        .expect("create singlesig PSBT");

        let progress = sign_psbt(
            &mut psbt,
            &SoftwareSigner::from_secret(DescriptorSecretKey::from_str(&secret).unwrap()),
        )
        .expect("sign BIP-86 key-path");
        assert_eq!(progress.signed_inputs, 1);
        assert!(
            psbt.inputs[0].tap_key_sig.is_some(),
            "BIP-86 spend must produce tap_key_sig (key-path)"
        );

        let tx = finalize_psbt(&mut psbt).expect("finalize BIP-86 send");
        assert_eq!(tx.input.len(), 1);
        assert!(!tx.output.is_empty());
        assert!(
            tx.input[0].witness.len() >= 1,
            "finalized taproot key-path witness"
        );
    }
}
