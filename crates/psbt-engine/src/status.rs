//! Inspect which policy keys have contributed signatures to a PSBT.

use std::collections::BTreeSet;

use bitcoin::psbt::Psbt;
use policy_engine::{spending_paths, PolicyConfig, SpendingPath};
use serde::{Deserialize, Serialize};

use crate::error::PsbtError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KeySignStatus {
    Signed,
    Missing,
    Unused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyStatus {
    pub id: String,
    pub fingerprint: String,
    pub role: String,
    pub status: KeySignStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathStatus {
    pub path: SpendingPath,
    pub satisfied: bool,
    pub missing_keys: Vec<String>,
    pub present_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningStatus {
    pub summary: String,
    pub keys: Vec<KeyStatus>,
    pub paths: Vec<PathStatus>,
    pub signed_fingerprints: Vec<String>,
    pub signed_input_count: usize,
    pub total_inputs: usize,
    pub active_path_id: Option<String>,
}

/// Collect master fingerprints that contributed at least one signature.
pub fn signed_fingerprints(psbt: &Psbt) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for input in &psbt.inputs {
        for pk in input.partial_sigs.keys() {
            if let Some((fp, _)) = input.bip32_derivation.get(&pk.inner) {
                out.insert(fp_hex(fp));
            }
        }
        for (xonly, _) in input.tap_script_sigs.keys() {
            if let Some((_, (fp, _))) = input.tap_key_origins.get(xonly) {
                out.insert(fp_hex(fp));
            }
        }
        if input.tap_key_sig.is_some() {
            // Key-path spend: if a single origin is present, attribute to it.
            if input.tap_key_origins.len() == 1 {
                if let Some((_, (fp, _))) = input.tap_key_origins.values().next() {
                    out.insert(fp_hex(fp));
                }
            }
        }
    }
    out
}

fn fp_hex(fp: &bitcoin::bip32::Fingerprint) -> String {
    // Fingerprint Display is 8 hex chars.
    fp.to_string().to_ascii_lowercase()
}

fn role_label(role: policy_engine::KeyRole) -> &'static str {
    match role {
        policy_engine::KeyRole::Investor => "investor",
        policy_engine::KeyRole::Manager => "manager",
        policy_engine::KeyRole::Recovery => "recovery",
        policy_engine::KeyRole::Cosigner => "cosigner",
        policy_engine::KeyRole::Other => "other",
    }
}

fn count_signed_inputs(psbt: &Psbt) -> usize {
    psbt.inputs
        .iter()
        .filter(|input| {
            !input.partial_sigs.is_empty()
                || input.tap_key_sig.is_some()
                || !input.tap_script_sigs.is_empty()
        })
        .count()
}

/// Build signing UX status for a vault policy + PSBT.
pub fn analyze_signing_status(
    config: &PolicyConfig,
    psbt: &Psbt,
    active_path_id: Option<&str>,
) -> Result<SigningStatus, PsbtError> {
    let fps = signed_fingerprints(psbt);
    let paths = spending_paths(config).map_err(|e| PsbtError::Psbt(e.to_string()))?;

    let keys: Vec<KeyStatus> = config
        .keys
        .iter()
        .map(|k| {
            let fp = k.fingerprint.trim().to_ascii_lowercase();
            let signed = fps.iter().any(|s| s.eq_ignore_ascii_case(&fp));
            KeyStatus {
                id: k.id.clone(),
                fingerprint: fp,
                role: role_label(k.role).into(),
                status: if signed {
                    KeySignStatus::Signed
                } else {
                    KeySignStatus::Missing
                },
            }
        })
        .collect();

    let path_status: Vec<PathStatus> = paths
        .into_iter()
        .map(|path| {
            let mut present = Vec::new();
            let mut missing = Vec::new();
            for id in &path.required_keys {
                let signed = config
                    .keys
                    .iter()
                    .find(|k| k.id.eq_ignore_ascii_case(id))
                    .map(|k| {
                        fps.iter()
                            .any(|s| s.eq_ignore_ascii_case(k.fingerprint.trim()))
                    })
                    .unwrap_or(false);
                if signed {
                    present.push(id.clone());
                } else {
                    missing.push(id.clone());
                }
            }
            let satisfied = missing.is_empty() && !path.required_keys.is_empty();
            PathStatus {
                path,
                satisfied,
                missing_keys: missing,
                present_keys: present,
            }
        })
        .collect();

    // Mark keys unused if they are not required by the active path (when selected).
    let mut keys = keys;
    if let Some(active) = active_path_id {
        if let Some(ps) = path_status.iter().find(|p| p.path.id == active) {
            for k in &mut keys {
                let required = ps
                    .path
                    .required_keys
                    .iter()
                    .any(|id| id.eq_ignore_ascii_case(&k.id));
                if !required && k.status != KeySignStatus::Signed {
                    k.status = KeySignStatus::Unused;
                }
            }
        }
    }

    let summary = build_summary(&path_status, active_path_id, &keys);

    Ok(SigningStatus {
        summary,
        keys,
        paths: path_status,
        signed_fingerprints: fps.into_iter().collect(),
        signed_input_count: count_signed_inputs(psbt),
        total_inputs: psbt.inputs.len(),
        active_path_id: active_path_id.map(str::to_string),
    })
}

fn build_summary(
    paths: &[PathStatus],
    active_path_id: Option<&str>,
    keys: &[KeyStatus],
) -> String {
    let focus = active_path_id
        .and_then(|id| paths.iter().find(|p| p.path.id == id))
        .or_else(|| paths.iter().find(|p| p.path.kind == policy_engine::SpendingPathKind::Primary))
        .or_else(|| paths.first());

    if let Some(ps) = focus {
        if ps.satisfied {
            return format!(
                "{} — complete ({})",
                ps.path.label,
                ps.present_keys.join("+")
            );
        }
        if ps.present_keys.is_empty() {
            return format!(
                "Need {} · none signed yet",
                ps.path.required_keys.join("+")
            );
        }
        return format!(
            "Need {} · have {} · missing {}",
            ps.path.required_keys.join("+"),
            ps.present_keys.join("+"),
            ps.missing_keys.join("+")
        );
    }

    let signed: Vec<_> = keys
        .iter()
        .filter(|k| k.status == KeySignStatus::Signed)
        .map(|k| k.id.as_str())
        .collect();
    if signed.is_empty() {
        "No signatures yet".into()
    } else {
        format!("Signed: {}", signed.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    use miniscript::descriptor::DescriptorSecretKey;
    use policy_engine::{
        KeyRole, NetworkName, PolicyExpression, ScriptTypeName, POLICY_SCHEMA_VERSION,
    };
    use wallet_core::Wallet;

    use crate::create::create_psbt;
    use crate::sign::{sign_psbt, SoftwareSigner};
    use crate::test_keys::{key_config_from_tprv, TEST_TPRV_A, TEST_TPRV_B};
    use crate::types::{CreatePsbtOptions, FeeRate, PsbtRecipient, SpendingUtxo};

    fn wallet() -> Wallet {
        let policy = policy_engine::PolicyConfig {
            version: POLICY_SCHEMA_VERSION,
            network: NetworkName::Regtest,
            script_type: ScriptTypeName::Taproot,
            keys: [
                key_config_from_tprv("A", KeyRole::Investor, TEST_TPRV_A),
                key_config_from_tprv("B", KeyRole::Manager, TEST_TPRV_B),
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
    fn status_after_one_signature_reports_missing_cosigner() {
        let wallet = wallet();
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

        sign_psbt(
            &mut psbt,
            &SoftwareSigner::from_secret(DescriptorSecretKey::from_str(TEST_TPRV_A).unwrap()),
        )
        .unwrap();

        let status =
            analyze_signing_status(&wallet.policy, &psbt, Some("primary-0")).unwrap();
        assert!(
            status.summary.contains("missing") || status.summary.contains("Need"),
            "{}",
            status.summary
        );
        let a = status.keys.iter().find(|k| k.id == "A").unwrap();
        let b = status.keys.iter().find(|k| k.id == "B").unwrap();
        assert_eq!(a.status, KeySignStatus::Signed);
        assert_eq!(b.status, KeySignStatus::Missing);
        assert!(!status.paths[0].satisfied);
    }
}
