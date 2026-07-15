use base64::Engine;
use bitcoin::psbt::Psbt;

use crate::error::PsbtError;
use crate::types::ExportFormat;

pub fn export_psbt(psbt: &Psbt, format: ExportFormat) -> Result<Vec<u8>, PsbtError> {
    match format {
        ExportFormat::Base64 => Ok(base64::engine::general_purpose::STANDARD
            .encode(psbt.serialize())
            .into_bytes()),
        ExportFormat::File => Ok(psbt.serialize()),
    }
}

pub fn import_psbt_base64(data: &[u8]) -> Result<Psbt, PsbtError> {
    let text = std::str::from_utf8(data).map_err(|e| PsbtError::Psbt(e.to_string()))?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(text.trim())
        .map_err(|e| PsbtError::Psbt(e.to_string()))?;
    Psbt::deserialize(&bytes).map_err(|e| PsbtError::Psbt(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create::create_psbt;
    use crate::types::{CreatePsbtOptions, FeeRate, PsbtRecipient, SpendingUtxo};
    use policy_engine::{
        test_vectors::TEST_FP, test_vectors::TEST_XPUB_A, test_vectors::TEST_XPUB_B, KeyConfig,
        KeyRole, NetworkName, PolicyConfig, PolicyExpression, ScriptTypeName,
        POLICY_SCHEMA_VERSION,
    };
    use wallet_core::Wallet;

    fn sample_wallet() -> Wallet {
        let policy = PolicyConfig {
            version: POLICY_SCHEMA_VERSION,
            network: NetworkName::Regtest,
            script_type: ScriptTypeName::Taproot,
            keys: [
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
            name: "export".into(),
            policy,
            descriptor,
            script_type: ScriptTypeName::Taproot,
            created_at: 0,
        }
    }

    #[test]
    fn base64_export_roundtrip() {
        let wallet = sample_wallet();
        let receive =
            address_engine::new_receive_address(&wallet.policy, &wallet.descriptor, 0).unwrap();
        let recipient =
            address_engine::new_receive_address(&wallet.policy, &wallet.descriptor, 1).unwrap();

        let psbt = create_psbt(
            &wallet,
            &[PsbtRecipient {
                address: recipient.address,
                amount_sats: 20_000,
            }],
            FeeRate::new(2),
            &[SpendingUtxo::new(
                blockchain::Utxo {
                    txid: "aa".repeat(32),
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

        let exported = export_psbt(&psbt, ExportFormat::Base64).unwrap();
        let parsed = import_psbt_base64(&exported).unwrap();
        assert_eq!(
            parsed.unsigned_tx.compute_txid(),
            psbt.unsigned_tx.compute_txid()
        );
    }
}
