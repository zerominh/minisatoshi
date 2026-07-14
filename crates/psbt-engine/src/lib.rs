//! PSBT creation, signing, and export for Minisatoshi.

mod broadcast;
mod combine;
mod create;
mod descriptor;
mod error;
mod export;
mod finalize;
mod sign;
#[cfg(test)]
mod test_keys;
mod types;

pub use broadcast::broadcast_psbt;
pub use combine::combine_psbt;
pub use create::create_psbt;
pub use error::PsbtError;
pub use export::{export_psbt, import_psbt_base64};
pub use finalize::{extract_transaction, finalize_psbt, transaction_bytes, transaction_hex};
pub use sign::{sign_psbt, Signer, SoftwareSigner};
pub use types::{
    CreatePsbtOptions, ExportFormat, FeeRate, Psbt, PsbtRecipient, SignProgress, SpendingUtxo,
    VaultPsbt,
};

#[cfg(test)]
mod integration_tests {
    use std::str::FromStr;

    use miniscript::descriptor::{DescriptorSecretKey, KeyMap};
    use policy_engine::{
        NetworkName, PolicyConfig, PolicyExpression, ScriptTypeName, POLICY_SCHEMA_VERSION,
    };
    use wallet_core::Vault;

    use crate::test_keys::{key_config_from_tprv, TEST_TPRV_A, TEST_TPRV_B};

    use super::*;

    fn regtest_vault() -> Vault {
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
            },
        };
        let descriptor = descriptor_engine::compile_descriptor_from_config(&policy).unwrap();
        Vault {
            id: "v1".into(),
            wallet_id: "w1".into(),
            name: "2of2".into(),
            policy,
            descriptor,
            script_type: ScriptTypeName::Taproot,
            created_at: 0,
        }
    }

    #[test]
    fn two_of_two_sign_combine_finalize() {
        let vault = regtest_vault();
        let receive =
            address_engine::new_receive_address(&vault.policy, &vault.descriptor, 0).unwrap();
        let recipient =
            address_engine::new_receive_address(&vault.policy, &vault.descriptor, 1).unwrap();

        let mut psbt = create_psbt(
            &vault,
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
}
