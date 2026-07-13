use address_engine::{new_change_address, new_receive_address};
use policy_engine::PolicyConfig;

use crate::error::ChainError;
use crate::types::SyncProgress;

pub const DEFAULT_GAP_LIMIT: u32 = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedAddress {
    pub address: String,
    pub index: u32,
    pub is_change: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanPlan {
    pub receive: Vec<ScannedAddress>,
    pub change: Vec<ScannedAddress>,
}

/// Build the address list to scan using BIP44-style gap limit discovery.
pub fn build_scan_plan(
    policy: &PolicyConfig,
    descriptor: &str,
    gap_limit: u32,
    has_activity: impl Fn(&str) -> Result<bool, ChainError>,
    progress: &dyn Fn(SyncProgress),
) -> Result<ScanPlan, ChainError> {
    let receive = discover_chain(policy, descriptor, false, gap_limit, &has_activity, progress)?;
    let change = discover_chain(policy, descriptor, true, gap_limit, &has_activity, progress)?;
    Ok(ScanPlan { receive, change })
}

fn discover_chain(
    policy: &PolicyConfig,
    descriptor: &str,
    is_change: bool,
    gap_limit: u32,
    has_activity: &impl Fn(&str) -> Result<bool, ChainError>,
    progress: &dyn Fn(SyncProgress),
) -> Result<Vec<ScannedAddress>, ChainError> {
    let mut discovered = Vec::new();
    let mut trailing_empty = 0_u32;
    let mut index = 0_u32;

    while trailing_empty < gap_limit {
        let derived = if is_change {
            new_change_address(policy, descriptor, index)?
        } else {
            new_receive_address(policy, descriptor, index)?
        };

        let active = has_activity(&derived.address)?;
        if active {
            discovered.push(ScannedAddress {
                address: derived.address.clone(),
                index,
                is_change,
            });
            trailing_empty = 0;
        } else {
            trailing_empty += 1;
        }

        progress(SyncProgress {
            scanned_addresses: index + 1,
            active_addresses: discovered.len() as u32,
            message: if is_change {
                format!("scanning change index {index}")
            } else {
                format!("scanning receive index {index}")
            },
        });

        index += 1;
    }

    Ok(discovered)
}

#[cfg(test)]
mod tests {
    use descriptor_engine::compile_descriptor_from_config;
    use policy_engine::{
        abc_preset, test_vectors::TEST_FP, test_vectors::TEST_XPUB_A, test_vectors::TEST_XPUB_B,
        test_vectors::TEST_XPUB_C, KeyConfig, KeyRole, NetworkName,
    };

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
    fn gap_limit_stops_after_empty_run() {
        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let descriptor = compile_descriptor_from_config(&policy).unwrap();

        let receive_0 = new_receive_address(&policy, &descriptor, 0).unwrap().address;
        let active = |address: &str| Ok(address == receive_0);

        let plan = build_scan_plan(&policy, &descriptor, 3, active, &|_| {}).unwrap();
        assert_eq!(plan.receive.len(), 1);
        assert_eq!(plan.change.len(), 0);
    }
}
