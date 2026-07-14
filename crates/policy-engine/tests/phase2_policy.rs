//! Phase 2 policy extensions — TDD coverage.
//!
//! Covers: multi-fallback merge, compound `allow`, templates → descriptor.

use policy_engine::{
    build_from_template, compile_leaf_policies, list_templates, validate_config, FallbackPolicy,
    KeyConfig, KeyRole, NetworkName, PolicyConfig, PolicyExpression, ScriptTypeName, TemplateId,
    POLICY_SCHEMA_VERSION,
};
use policy_engine::test_vectors::{TEST_FP, TEST_XPUB_A, TEST_XPUB_B, TEST_XPUB_C};

fn key(id: &str, role: KeyRole, xpub: &str) -> KeyConfig {
    KeyConfig {
        id: id.into(),
        role,
        xpub: xpub.into(),
        fingerprint: if id == "A" {
            "78412e3a".into()
        } else {
            TEST_FP.into()
        },
        origin_path: None,
    }
}

fn abc_keys() -> Vec<KeyConfig> {
    vec![
        key("A", KeyRole::Investor, TEST_XPUB_A),
        key("B", KeyRole::Manager, TEST_XPUB_B),
        key("C", KeyRole::Recovery, TEST_XPUB_C),
    ]
}

#[test]
fn legacy_fallback_json_deserializes_without_fallbacks_field() {
    let json = r#"{
        "version": 1,
        "network": "testnet",
        "script_type": "taproot",
        "keys": [
            {"id":"A","role":"investor","xpub":"tpub","fingerprint":"78412e3a"}
        ],
        "policy": {
            "primary": "A",
            "fallback": {"after":"1d","allow":"A"}
        }
    }"#;
    // Minimal fixture — use real xpubs via rewrite
    let mut config: PolicyConfig = serde_json::from_str(&json.replace(
        "\"xpub\":\"tpub\"",
        &format!("\"xpub\":\"{TEST_XPUB_A}\""),
    ))
    .unwrap();
    config.keys[0].xpub = TEST_XPUB_A.into();
    assert!(config.policy.fallbacks.is_empty());
    assert!(config.policy.fallback.is_some());
    assert_eq!(config.policy.all_fallbacks().len(), 1);
    validate_config(&config).unwrap();
}

#[test]
fn legacy_plus_fallbacks_merge_in_leaf_count() {
    let config = PolicyConfig {
        version: POLICY_SCHEMA_VERSION,
        network: NetworkName::Testnet,
        script_type: ScriptTypeName::Taproot,
        keys: abc_keys(),
        policy: PolicyExpression {
            primary: "A && B".into(),
            fallback: Some(FallbackPolicy {
                after: "1y".into(),
                allow: "A".into(),
            }),
            fallbacks: vec![FallbackPolicy {
                after: "4y".into(),
                allow: "C".into(),
            }],
        },
    };
    let leaves = compile_leaf_policies(&config).unwrap();
    // primary A&&B → 1 leaf + 2 fallbacks
    assert_eq!(leaves.len(), 3);
    assert!(leaves.iter().any(|l| l.contains("older(52560)"))); // 1y
    assert!(leaves.iter().any(|l| l.contains("older(210240)"))); // 4y
}

#[test]
fn compound_allow_expression_compiles_to_and_leaf() {
    let config = PolicyConfig {
        version: POLICY_SCHEMA_VERSION,
        network: NetworkName::Testnet,
        script_type: ScriptTypeName::Taproot,
        keys: abc_keys(),
        policy: PolicyExpression {
            primary: "A && B".into(),
            fallback: None,
            fallbacks: vec![FallbackPolicy {
                after: "1w".into(),
                allow: "A && C".into(),
            }],
        },
    };
    let leaves = compile_leaf_policies(&config).unwrap();
    assert_eq!(leaves.len(), 2);
    assert_eq!(
        leaves[1],
        format!("and(older(1008),and(pk(A),pk(C)))")
    );
}

#[test]
fn compound_allow_produces_taproot_descriptor() {
    let config = PolicyConfig {
        version: POLICY_SCHEMA_VERSION,
        network: NetworkName::Testnet,
        script_type: ScriptTypeName::Taproot,
        keys: abc_keys(),
        policy: PolicyExpression {
            primary: "A && B".into(),
            fallback: None,
            fallbacks: vec![FallbackPolicy {
                after: "1d".into(),
                allow: "A && C".into(),
            }],
        },
    };
    let descriptor = descriptor_engine::compile_descriptor_from_config(&config).unwrap();
    assert!(descriptor.starts_with("tr("));
    assert!(descriptor.contains('#'));
}

#[test]
fn all_listed_templates_build_and_compile() {
    assert_eq!(list_templates().len(), 6);

    for info in list_templates() {
        if info.id == TemplateId::Custom {
            let err = build_from_template(
                TemplateId::Custom,
                abc_keys(),
                NetworkName::Testnet,
                None,
                vec![],
            )
            .unwrap_err();
            assert!(err.to_string().contains("custom"));
            continue;
        }

        let keys = match info.id {
            TemplateId::Inheritance | TemplateId::DeadMansSwitch => vec![
                key("A", KeyRole::Investor, TEST_XPUB_A),
                key("B", KeyRole::Manager, TEST_XPUB_B),
            ],
            TemplateId::MultiManager => vec![
                key("A", KeyRole::Investor, TEST_XPUB_A),
                key("B", KeyRole::Manager, TEST_XPUB_B),
                key("C", KeyRole::Manager, TEST_XPUB_C),
            ],
            _ => abc_keys(),
        };

        let config =
            build_from_template(info.id, keys, NetworkName::Testnet, None, vec![]).unwrap();
        validate_config(&config).unwrap();
        let descriptor = descriptor_engine::compile_descriptor_from_config(&config).unwrap();
        assert!(
            descriptor.starts_with("tr("),
            "template {:?} did not compile to tr()",
            info.id
        );
    }
}

#[test]
fn template_id_aliases_parse() {
    assert_eq!(TemplateId::parse("2of3"), Some(TemplateId::TwoOfThree));
    assert_eq!(TemplateId::parse("dms"), Some(TemplateId::DeadMansSwitch));
    assert_eq!(TemplateId::parse("nope"), None);
}

#[test]
fn custom_template_with_override_compiles() {
    let config = build_from_template(
        TemplateId::Custom,
        abc_keys(),
        NetworkName::Testnet,
        Some("(A && B) || (B && C)".into()),
        vec![FallbackPolicy {
            after: "2w".into(),
            allow: "A".into(),
        }],
    )
    .unwrap();
    assert_eq!(config.policy.primary, "(A && B) || (B && C)");
    assert_eq!(config.policy.fallbacks.len(), 1);
    validate_config(&config).unwrap();
    descriptor_engine::compile_descriptor_from_config(&config).unwrap();
}

#[test]
fn rejects_empty_fallback_allow() {
    let config = PolicyConfig {
        version: POLICY_SCHEMA_VERSION,
        network: NetworkName::Testnet,
        script_type: ScriptTypeName::Taproot,
        keys: abc_keys(),
        policy: PolicyExpression {
            primary: "A && B".into(),
            fallback: None,
            fallbacks: vec![FallbackPolicy {
                after: "1y".into(),
                allow: "   ".into(),
            }],
        },
    };
    assert_eq!(
        validate_config(&config),
        Err(policy_engine::PolicyError::EmptyFallbackAllow)
    );
}

#[test]
fn multi_manager_requires_manager_key() {
    let err = build_from_template(
        TemplateId::MultiManager,
        vec![key("A", KeyRole::Investor, TEST_XPUB_A)],
        NetworkName::Testnet,
        None,
        vec![],
    )
    .unwrap_err();
    assert!(err.to_string().contains("manager"));
}
