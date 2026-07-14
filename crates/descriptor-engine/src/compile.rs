use std::sync::Arc;

use bitcoin::Network;
use miniscript::descriptor::{Descriptor, TapTree};
use miniscript::policy::compiler;
use miniscript::policy::Concrete;
use miniscript::{DescriptorPublicKey, Miniscript, Segwitv0, Tap};
use policy_engine::{
    compile_leaf_policies, compile_miniscript, descriptor_key_expression, KeyTranslator,
    PolicyConfig, ScriptTypeName,
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

/// Verify BIP-380 descriptor checksum without fully parsing the script tree.
///
/// Miniscript Taproot trees from our compiler may not round-trip through
/// `Descriptor::parse` even when the checksum is correct — import validates
/// checksum here and relies on address derivation at use time.
pub fn verify_descriptor_checksum(descriptor: &str) -> Result<(), DescriptorError> {
    miniscript::descriptor::checksum::verify_checksum(descriptor.trim())
        .map(|_| ())
        .map_err(|e| DescriptorError::Parse(format!("invalid checksum: {e}")))
}

/// Append a BIP-380 checksum when missing; verify if already present.
pub fn ensure_descriptor_checksum(descriptor: &str) -> Result<String, DescriptorError> {
    let desc = descriptor.trim();
    if desc.is_empty() {
        return Err(DescriptorError::Parse("descriptor is empty".into()));
    }
    if desc.contains('#') {
        verify_descriptor_checksum(desc)?;
        return Ok(desc.to_string());
    }
    let mut eng = miniscript::descriptor::checksum::Engine::new();
    eng.input(desc)
        .map_err(|e| DescriptorError::Parse(format!("checksum engine: {e}")))?;
    Ok(format!("{desc}#{}", eng.checksum()))
}

fn compile_taproot(
    config: &PolicyConfig,
) -> Result<Descriptor<DescriptorPublicKey>, DescriptorError> {
    let leaf_policies = compile_leaf_policies(config).map_err(DescriptorError::Policy)?;

    // Single-key Taproot (hot wallets / BIP-86): key-path spend `tr(xpub/…)` —
    // NOT `tr(NUMS,{pk(A)})` which yields different addresses and breaks Sparrow parity.
    if let Some(key_id) = single_key_policy_id(&leaf_policies) {
        return compile_bip86_keypath(config, key_id);
    }

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

/// `["pk(A)"]` → Some("A"); multi-key / and / or → None.
fn single_key_policy_id(leaves: &[String]) -> Option<&str> {
    if leaves.len() != 1 {
        return None;
    }
    let leaf = leaves[0].trim();
    let inner = leaf.strip_prefix("pk(")?.strip_suffix(')')?;
    if inner.is_empty() || inner.contains(['(', ')', ',']) {
        return None;
    }
    Some(inner)
}

fn compile_bip86_keypath(
    config: &PolicyConfig,
    key_id: &str,
) -> Result<Descriptor<DescriptorPublicKey>, DescriptorError> {
    let key = config
        .keys
        .iter()
        .find(|k| k.id == key_id)
        .ok_or_else(|| DescriptorError::Compile(format!("unknown key '{key_id}'")))?;
    let expr = descriptor_key_expression(key).map_err(DescriptorError::Policy)?;
    let dpk: DescriptorPublicKey = expr
        .parse()
        .map_err(|e| DescriptorError::Compile(format!("invalid BIP-86 key '{key_id}': {e}")))?;
    Descriptor::new_tr(dpk, None).map_err(|e| DescriptorError::Compile(e.to_string()))
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
    fn singlesig_taproot_uses_bip86_keypath_not_nums() {
        let config = PolicyConfig {
            version: 1,
            network: NetworkName::Testnet,
            script_type: ScriptTypeName::Taproot,
            keys: vec![KeyConfig {
                id: "A".into(),
                role: KeyRole::Other,
                xpub: TEST_XPUB_A.into(),
                fingerprint: "78412e3a".into(),
                origin_path: Some("86'/1'/0'".into()),
            }],
            policy: PolicyExpression {
                primary: "A".into(),
                fallback: None,
                fallbacks: vec![],
            },
        };
        let descriptor = compile_descriptor_from_config(&config).unwrap();
        assert!(descriptor.starts_with("tr("));
        assert!(descriptor.contains("[78412e3a/86'/1'/0']"));
        assert!(descriptor.contains("/<0;1>/*"));
        assert!(!descriptor.contains(NUMS_UNSPENDABLE_KEY));
        assert!(!descriptor.contains('{'));
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
                fallbacks: vec![],
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
                fallbacks: vec![],
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
                fallbacks: vec![],
            },
        };
        let descriptor = compile_descriptor_from_config(&config).unwrap();
        assert!(descriptor.starts_with("wsh("));
        assert!(descriptor.contains('#'));
    }

    #[test]
    fn origin_path_m_prefix_from_hardware_wallet_compiles() {
        let config = PolicyConfig {
            version: 1,
            network: NetworkName::Testnet,
            script_type: ScriptTypeName::Taproot,
            keys: vec![
                KeyConfig {
                    id: "A".into(),
                    role: KeyRole::Investor,
                    xpub: "tpubDDo3KgGaeh7ZqXfwnzK5SFu6o7gDjZT2bixwCJnuFouD7c1CYsavRAEJ9pJiUXCtF6jinBUDcoHnVcnboiyWXAkvQGVfgjTVv88zrgZT2aW".into(),
                    fingerprint: "c0a7b76c".into(),
                    origin_path: Some("m/86'/1'/0'".into()),
                },
                KeyConfig {
                    id: "B".into(),
                    role: KeyRole::Manager,
                    xpub: "tpubDCks4R9bRfs83gbXgo2KnrJiLFAc2i3epsgD2JEXfE42FGy96SbhrDib4CmfGrgeQRQbxwXUNzNoQkwQmVoSCq9a1mggS6FfcfDs7rpF6j8".into(),
                    fingerprint: "60ca2e86".into(),
                    origin_path: Some("m/86'/1'/0'".into()),
                },
                KeyConfig {
                    id: "C".into(),
                    role: KeyRole::Recovery,
                    xpub: "tpubDCkqutoxx7uej1nzg8f8qmxYntcGG24Uq1VuSLTgCccwQBk5nTSrrnpyFiJTy4LarJWM21a7cqGgN7APPNXbJBQqDrvqjoTk5VmUahjZvSg".into(),
                    fingerprint: "efba9b93".into(),
                    origin_path: Some("m/86'/1'/0'".into()),
                },
            ],
            policy: PolicyExpression {
                primary: "(A && B) || (A && C)".into(),
                fallback: Some(FallbackPolicy {
                    after: "1d".into(),
                    allow: "A".into(),
                }),
                fallbacks: vec![],
            },
        };
        let descriptor = compile_descriptor_from_config(&config).unwrap();
        assert!(descriptor.starts_with("tr("));
        assert!(descriptor.contains("[60ca2e86/86'/1'/0']"));
        assert!(!descriptor.contains("/m/"));
    }
}
