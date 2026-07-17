//! Extra PSBT metadata for hardware signers (HWI / BIP-388).

use std::str::FromStr;

use bitcoin::bip32::{DerivationPath, Fingerprint, Xpub};
use bitcoin::psbt::Psbt;
use miniscript::psbt::PsbtExt;
use policy_engine::PolicyConfig;

use crate::descriptor::definite_descriptor_at;
use crate::error::PsbtError;

/// Insert vault key xpubs into the PSBT global map (BIP-174), helping HWI match keys.
pub fn populate_global_xpubs(psbt: &mut Psbt, policy: &PolicyConfig) -> Result<(), PsbtError> {
    for key in &policy.keys {
        let xpub = Xpub::from_str(key.xpub.trim()).map_err(|e| {
            PsbtError::Psbt(format!("invalid xpub for key '{}': {e}", key.id))
        })?;
        let fingerprint = parse_fingerprint_hex(&key.fingerprint)?;
        let derivation = key
            .origin_path
            .as_deref()
            .map(parse_origin_path)
            .transpose()?
            .unwrap_or_default();
        psbt.xpub.insert(xpub, (fingerprint, derivation));
    }
    Ok(())
}

/// Tag a change output with its definite descriptor so signers can recognize change.
pub fn tag_change_output(
    psbt: &mut Psbt,
    policy: &PolicyConfig,
    output_index: usize,
    change_index: u32,
) -> Result<(), PsbtError> {
    if output_index >= psbt.outputs.len() {
        return Ok(());
    }
    let desc = definite_descriptor_at(policy, change_index, true)?;
    psbt.update_output_with_descriptor(output_index, &desc)
        .map_err(|e| PsbtError::Psbt(e.to_string()))
}

fn parse_fingerprint_hex(raw: &str) -> Result<Fingerprint, PsbtError> {
    let hex = raw.trim();
    Fingerprint::from_hex(hex).map_err(|e| PsbtError::Psbt(format!("invalid fingerprint '{hex}': {e}")))
}

fn parse_origin_path(raw: &str) -> Result<DerivationPath, PsbtError> {
    let trimmed = raw.trim().trim_start_matches("m/").trim_start_matches("M/");
    DerivationPath::from_str(trimmed).map_err(|e| PsbtError::Psbt(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use policy_engine::{
        test_vectors::{TEST_FP, TEST_XPUB_A, TEST_XPUB_B},
        KeyConfig, KeyRole, NetworkName, PolicyConfig, PolicyExpression, ScriptTypeName,
        POLICY_SCHEMA_VERSION,
    };

    #[test]
    fn global_xpubs_populated() {
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
                    origin_path: Some("86'/1'/0'".into()),
                },
                KeyConfig {
                    id: "B".into(),
                    role: KeyRole::Manager,
                    xpub: TEST_XPUB_B.into(),
                    fingerprint: TEST_FP.into(),
                    origin_path: Some("86'/1'/0'".into()),
                },
            ]
            .into(),
            policy: PolicyExpression {
                primary: "A && B".into(),
                fallback: None,
                fallbacks: vec![],
            },
        };
        let mut psbt = Psbt {
            unsigned_tx: bitcoin::Transaction {
                version: bitcoin::transaction::Version::TWO,
                lock_time: bitcoin::absolute::LockTime::ZERO,
                input: vec![],
                output: vec![],
            },
            version: 0,
            xpub: Default::default(),
            proprietary: Default::default(),
            unknown: Default::default(),
            inputs: vec![],
            outputs: vec![],
        };
        populate_global_xpubs(&mut psbt, &policy).unwrap();
        assert_eq!(psbt.xpub.len(), 2);
    }
}
