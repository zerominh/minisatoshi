use bitcoin::absolute::LockTime;
use bitcoin::psbt::Psbt;
use bitcoin::transaction::Version;
use bitcoin::{Address, Amount, OutPoint, Sequence, TxIn, TxOut, Txid};
use miniscript::psbt::PsbtExt;
use std::collections::BTreeMap;
use std::str::FromStr;
use wallet_core::Vault;

use crate::descriptor::definite_descriptor_at;
use crate::error::PsbtError;
use crate::types::{CreatePsbtOptions, FeeRate, PsbtRecipient, SpendingUtxo};

pub fn create_psbt(
    vault: &Vault,
    recipients: &[PsbtRecipient],
    fee_rate: FeeRate,
    utxos: &[SpendingUtxo],
    options: CreatePsbtOptions,
) -> Result<Psbt, PsbtError> {
    if utxos.is_empty() {
        return Err(PsbtError::NoInputs);
    }
    if recipients.is_empty() {
        return Err(PsbtError::NoOutputs);
    }

    let network = vault.policy.network.to_bitcoin_network();
    let input_total: u64 = utxos.iter().map(|entry| entry.utxo.value_sats).sum();
    let recipient_total: u64 = recipients.iter().map(|entry| entry.amount_sats).sum();

    let estimated_vbytes = estimate_vbytes(utxos.len(), recipients.len() + 1);
    let fee_sats = fee_rate.sat_per_vb.saturating_mul(estimated_vbytes as u64);
    let change_index = options.change_index.unwrap_or(0);

    let mut outputs = Vec::with_capacity(recipients.len() + 1);
    for recipient in recipients {
        let address = parse_address(&recipient.address, network)?;
        outputs.push(TxOut {
            value: Amount::from_sat(recipient.amount_sats),
            script_pubkey: address.script_pubkey(),
        });
    }

    let change_amount = input_total
        .checked_sub(recipient_total)
        .and_then(|value| value.checked_sub(fee_sats))
        .ok_or(PsbtError::InsufficientFunds {
            needed: recipient_total + fee_sats,
            available: input_total,
        })?;

    if change_amount > 546 {
        let change_address =
            address_engine::new_change_address(&vault.policy, &vault.descriptor, change_index)?;
        let address = parse_address(&change_address.address, network)?;
        outputs.push(TxOut {
            value: Amount::from_sat(change_amount),
            script_pubkey: address.script_pubkey(),
        });
    } else if change_amount != 0 {
        return Err(PsbtError::InsufficientFunds {
            needed: recipient_total + fee_sats + change_amount,
            available: input_total,
        });
    }

    let sequence = options
        .input_sequence
        .map(Sequence::from_consensus)
        .unwrap_or(Sequence::ENABLE_RBF_NO_LOCKTIME);

    let mut unsigned_tx = bitcoin::Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: Vec::with_capacity(utxos.len()),
        output: outputs.clone(),
    };

    for spending in utxos {
        let txid = Txid::from_str(&spending.utxo.txid)
            .map_err(|e| PsbtError::Psbt(format!("invalid txid: {e}")))?;
        unsigned_tx.input.push(TxIn {
            previous_output: OutPoint {
                txid,
                vout: spending.utxo.vout,
            },
            sequence,
            ..Default::default()
        });
    }

    let mut psbt = Psbt {
        unsigned_tx: unsigned_tx.clone(),
        version: 0,
        xpub: BTreeMap::new(),
        proprietary: BTreeMap::new(),
        unknown: BTreeMap::new(),
        inputs: vec![bitcoin::psbt::Input::default(); utxos.len()],
        outputs: vec![bitcoin::psbt::Output::default(); unsigned_tx.output.len()],
    };

    for (index, spending) in utxos.iter().enumerate() {
        let definite_descriptor =
            definite_descriptor_at(&vault.policy, spending.derivation_index, spending.is_change)?;
        let witness_utxo = TxOut {
            value: Amount::from_sat(spending.utxo.value_sats),
            script_pubkey: definite_descriptor.script_pubkey(),
        };

        psbt.inputs[index].witness_utxo = Some(witness_utxo);
        psbt.update_input_with_descriptor(index, &definite_descriptor)
            .map_err(|e| PsbtError::Psbt(e.to_string()))?;
    }

    Ok(psbt)
}

fn parse_address(address: &str, network: bitcoin::Network) -> Result<Address, PsbtError> {
    Address::from_str(address)
        .map_err(|e| PsbtError::InvalidAddress(e.to_string()))?
        .require_network(network)
        .map_err(|e| PsbtError::InvalidAddress(e.to_string()))
}

fn estimate_vbytes(input_count: usize, output_count: usize) -> usize {
    10 + input_count * 58 + output_count * 43
}

#[cfg(test)]
mod tests {
    use policy_engine::{
        test_vectors::TEST_FP, test_vectors::TEST_XPUB_A, test_vectors::TEST_XPUB_B, KeyConfig,
        KeyRole, NetworkName, PolicyConfig, PolicyExpression, ScriptTypeName,
        POLICY_SCHEMA_VERSION,
    };
    use wallet_core::Vault;

    use super::*;
    use crate::types::SpendingUtxo;

    fn two_of_two_vault() -> Vault {
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
    fn creates_unsigned_psbt_with_change() {
        let vault = two_of_two_vault();
        let receive =
            address_engine::new_receive_address(&vault.policy, &vault.descriptor, 0).unwrap();
        let recipient =
            address_engine::new_receive_address(&vault.policy, &vault.descriptor, 1).unwrap();

        let psbt = create_psbt(
            &vault,
            &[PsbtRecipient {
                address: recipient.address,
                amount_sats: 40_000,
            }],
            FeeRate::new(2),
            &[SpendingUtxo::new(
                blockchain::Utxo {
                    txid: "11".repeat(32),
                    vout: 0,
                    value_sats: 100_000,
                    address: receive.address,
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
        .unwrap();

        assert_eq!(psbt.inputs.len(), 1);
        assert_eq!(psbt.unsigned_tx.output.len(), 2);
        assert!(psbt.inputs[0].witness_utxo.is_some());
    }

    #[test]
    fn timelock_sets_input_sequence() {
        let vault = two_of_two_vault();
        let receive =
            address_engine::new_receive_address(&vault.policy, &vault.descriptor, 0).unwrap();
        let recipient =
            address_engine::new_receive_address(&vault.policy, &vault.descriptor, 1).unwrap();
        let sequence = 52560 * 4;

        let psbt = create_psbt(
            &vault,
            &[PsbtRecipient {
                address: recipient.address,
                amount_sats: 40_000,
            }],
            FeeRate::new(2),
            &[SpendingUtxo::new(
                blockchain::Utxo {
                    txid: "22".repeat(32),
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
            CreatePsbtOptions {
                input_sequence: Some(sequence),
                change_index: None,
            },
        )
        .unwrap();

        assert_eq!(
            psbt.unsigned_tx.input[0].sequence.to_consensus_u32(),
            sequence
        );
    }

    #[test]
    fn rejects_insufficient_funds() {
        let vault = two_of_two_vault();
        let receive =
            address_engine::new_receive_address(&vault.policy, &vault.descriptor, 0).unwrap();
        let recipient =
            address_engine::new_receive_address(&vault.policy, &vault.descriptor, 1).unwrap();

        let err = create_psbt(
            &vault,
            &[PsbtRecipient {
                address: recipient.address,
                amount_sats: 90_000,
            }],
            FeeRate::new(10),
            &[SpendingUtxo::new(
                blockchain::Utxo {
                    txid: "33".repeat(32),
                    vout: 0,
                    value_sats: 50_000,
                    address: receive.address,
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
        .unwrap_err();

        assert!(matches!(
            err,
            PsbtError::InsufficientFunds {
                available: 50_000,
                ..
            }
        ));
    }

    #[test]
    fn rejects_dust_change_as_insufficient() {
        let vault = two_of_two_vault();
        let receive =
            address_engine::new_receive_address(&vault.policy, &vault.descriptor, 0).unwrap();
        let recipient =
            address_engine::new_receive_address(&vault.policy, &vault.descriptor, 1).unwrap();

        // Craft fee so leftover change is between 1 and 546 sats.
        // estimate_vbytes(1 in, 2 out) = 10 + 58 + 86 = 154; fee_rate 324 → fee 49896
        // input 50000 - recipient 50 - fee 49896 = 54 (dust)
        let err = create_psbt(
            &vault,
            &[PsbtRecipient {
                address: recipient.address,
                amount_sats: 50,
            }],
            FeeRate::new(324),
            &[SpendingUtxo::new(
                blockchain::Utxo {
                    txid: "44".repeat(32),
                    vout: 0,
                    value_sats: 50_000,
                    address: receive.address,
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
        .unwrap_err();

        assert!(matches!(err, PsbtError::InsufficientFunds { .. }));
    }
}
