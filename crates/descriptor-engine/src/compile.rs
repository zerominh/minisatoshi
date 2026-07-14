use std::sync::Arc;

use bitcoin::Network;
use miniscript::descriptor::{Descriptor, TapTree};
use miniscript::policy::compiler;
use miniscript::policy::Concrete;
use miniscript::{DescriptorPublicKey, Miniscript, Segwitv0, Tap};
use policy_engine::{
    compile_leaf_policies, compile_miniscript, KeyTranslator, PolicyConfig, ScriptTypeName,
};
use std::str::FromStr;

use crate::error::DescriptorError;

/// BIP-341 standard unspendable NUMS key for script-only Taproot descriptors.
pub const NUMS_UNSPENDABLE_KEY: &str =
    "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

/// Compile a policy configuration into a checksumed output descriptor string.
pub fn compile_descriptor_from_config(config: &PolicyConfig) -> Result<String, DescriptorError> {
    let descriptor = compile_descriptor_from_abstract(config)?;
    Ok(descriptor.to_string())
}

/// Compile a policy configuration into a typed descriptor.
pub fn compile_descriptor_from_abstract(
    config: &PolicyConfig,
) -> Result<Descriptor<DescriptorPublicKey>, DescriptorError> {
    match config.script_type {
        ScriptTypeName::Taproot => compile_taproot(config),
        ScriptTypeName::Wsh => {
            let resolved = compile_miniscript(config).map_err(DescriptorError::Policy)?;
            compile_wsh(&resolved)
        }
    }
}

/// Parse and validate a descriptor string.
pub fn parse_descriptor(
    descriptor: &str,
    _network: Network,
) -> Result<Descriptor<DescriptorPublicKey>, DescriptorError> {
    let desc = descriptor
        .parse::<Descriptor<DescriptorPublicKey>>()
        .map_err(|e| DescriptorError::Parse(e.to_string()))?;
    desc.sanity_check()
        .map_err(|e| DescriptorError::Parse(e.to_string()))?;
    Ok(desc)
}

/// Extract the checksum portion from a descriptor (if present).
pub fn descriptor_checksum(descriptor: &str) -> Option<String> {
    descriptor
        .rsplit_once('#')
        .map(|(_, checksum)| checksum.to_string())
}

fn compile_taproot(
    config: &PolicyConfig,
) -> Result<Descriptor<DescriptorPublicKey>, DescriptorError> {
    let leaf_policies = compile_leaf_policies(config).map_err(DescriptorError::Policy)?;
    let mut compiled_leaves = Vec::with_capacity(leaf_policies.len());

    for leaf in leaf_policies {
        let policy: Concrete<String> = leaf
            .parse::<Concrete<String>>()
            .map_err(|e| DescriptorError::Compile(e.to_string()))?;
        let miniscript = compiler::best_compilation::<String, Tap>(&policy)
            .map_err(|e| DescriptorError::Compile(e.to_string()))?;
        compiled_leaves.push(Arc::new(miniscript));
    }

    let tree = combine_taptree(compiled_leaves);
    let descriptor = Descriptor::new_tr(NUMS_UNSPENDABLE_KEY.to_string(), Some(tree))
        .map_err(|e| DescriptorError::Compile(e.to_string()))?;

    descriptor
        .translate_pk(&mut KeyTranslator { config })
        .map_err(|e| DescriptorError::Compile(format!("{e:?}")))
}

fn combine_taptree(mut leaves: Vec<Arc<Miniscript<String, Tap>>>) -> TapTree<String> {
    let unsatisfiable = Arc::new(
        Miniscript::<String, Tap>::from_str("0").expect("unsatisfiable miniscript should parse"),
    );

    while leaves.len() > 1 && !leaves.len().is_power_of_two() {
        leaves.push(unsatisfiable.clone());
    }

    let mut level = leaves.into_iter().map(TapTree::leaf).collect::<Vec<_>>();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() / 2);
        let mut iter = level.into_iter();
        while let (Some(left), Some(right)) = (iter.next(), iter.next()) {
            next.push(TapTree::combine(left, right).expect("tap tree within depth limit"));
        }
        level = next;
    }

    level.pop().expect("at least one leaf")
}

fn compile_wsh(
    policy: &Concrete<DescriptorPublicKey>,
) -> Result<Descriptor<DescriptorPublicKey>, DescriptorError> {
    let miniscript: Miniscript<DescriptorPublicKey, Segwitv0> = policy
        .compile()
        .map_err(|e| DescriptorError::Compile(e.to_string()))?;
    Descriptor::new_wsh(miniscript).map_err(|e| DescriptorError::Compile(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use policy_engine::{
        abc_preset, blocks_per_year, test_vectors::TEST_FP, test_vectors::TEST_XPUB_A,
        test_vectors::TEST_XPUB_B, test_vectors::TEST_XPUB_C, FallbackPolicy, KeyConfig, KeyRole,
        NetworkName, PolicyConfig, PolicyExpression, ScriptTypeName,
    };

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
    fn abc_preset_compiles_to_taproot_descriptor() {
        let keys = sample_keys();
        let config = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let descriptor = compile_descriptor_from_config(&config).unwrap();

        assert!(descriptor.starts_with("tr("));
        assert!(descriptor.contains('#'));
        assert!(descriptor.contains(NUMS_UNSPENDABLE_KEY));
    }

    #[test]
    fn abc_preset_matches_golden_vector_files() {
        let keys = sample_keys();
        let config = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let descriptor = compile_descriptor_from_config(&config).unwrap();
        let expected =
            include_str!("../../../tests/vectors/policy_abc_expected_descriptor.txt").trim();
        assert_eq!(descriptor, expected);

        let json: PolicyConfig = serde_json::from_str(include_str!(
            "../../../tests/vectors/policy_abc_testnet.json"
        ))
        .unwrap();
        let from_json = compile_descriptor_from_config(&json).unwrap();
        assert_eq!(from_json, expected);
    }

    #[test]
    fn descriptor_roundtrip_parse() {
        let keys = sample_keys();
        let config = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let descriptor = compile_descriptor_from_abstract(&config).unwrap();
        descriptor.sanity_check().unwrap();
        assert!(matches!(descriptor, Descriptor::Tr(_)));
    }

    #[test]
    fn two_of_three_compiles() {
        let keys = sample_keys();
        let config = PolicyConfig {
            version: 1,
            network: NetworkName::Testnet,
            script_type: ScriptTypeName::Taproot,
            keys: keys.into(),
            policy: PolicyExpression {
                primary: "(A && B) || (A && C) || (B && C)".into(),
                fallback: None,
            },
        };
        let descriptor = compile_descriptor_from_config(&config).unwrap();
        assert!(descriptor.starts_with("tr("));
    }

    #[test]
    fn timelock_fallback_in_descriptor() {
        let keys = sample_keys();
        let config = PolicyConfig {
            version: 1,
            network: NetworkName::Testnet,
            script_type: ScriptTypeName::Taproot,
            keys: keys.into(),
            policy: PolicyExpression {
                primary: "A && B".into(),
                fallback: Some(FallbackPolicy {
                    after: "4y".into(),
                    allow: "A".into(),
                }),
            },
        };
        let leaves = policy_engine::compile_leaf_policies(&config).unwrap();
        assert_eq!(leaves.len(), 2);
        assert!(leaves[1].contains(&format!("older({})", 4 * blocks_per_year())));
        compile_descriptor_from_config(&config).unwrap();
    }

    #[test]
    fn mainnet_abc_matches_golden_vector() {
        let json: PolicyConfig = serde_json::from_str(include_str!(
            "../../../tests/vectors/policy_abc_mainnet.json"
        ))
        .unwrap();
        assert_eq!(json.network, NetworkName::Mainnet);
        let descriptor = compile_descriptor_from_config(&json).unwrap();
        let expected =
            include_str!("../../../tests/vectors/policy_abc_mainnet_descriptor.txt").trim();
        assert_eq!(descriptor, expected);
    }

    #[test]
    fn wsh_script_type_compiles() {
        let keys = sample_keys();
        let config = PolicyConfig {
            version: 1,
            network: NetworkName::Testnet,
            script_type: ScriptTypeName::Wsh,
            keys: keys.into(),
            policy: PolicyExpression {
                primary: "A && B".into(),
                fallback: None,
            },
        };
        let descriptor = compile_descriptor_from_config(&config).unwrap();
        assert!(descriptor.starts_with("wsh("));
        assert!(descriptor.contains('#'));
    }
}
