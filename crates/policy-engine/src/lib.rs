//! Policy configuration types for Minisatoshi vaults.

mod compile;
mod config;
mod error;
mod parser;
mod timelock;
mod translate;
#[cfg(test)]
mod translate_tests;
mod validate;

#[doc(hidden)]
pub mod test_vectors;

pub use compile::{
    compile_abstract_miniscript, compile_abstract_policy_string, compile_leaf_policies,
    compile_miniscript, compile_policy_string,
};
pub use config::{
    FallbackPolicy, KeyConfig, KeyRole, NetworkName, PolicyConfig, PolicyExpression,
    ScriptTypeName, POLICY_SCHEMA_VERSION,
};
pub use error::PolicyError;
pub use parser::parse_expression;
pub use timelock::{
    blocks_per_day, blocks_per_week, blocks_per_year, parse_duration, DurationUnit, BLOCKS_PER_DAY,
    BLOCKS_PER_WEEK, BLOCKS_PER_YEAR,
};
pub use translate::{descriptor_key_expression, translate_policy_keys, KeyTranslator};

use validate::validate;

/// Validate a policy configuration.
pub fn validate_config(config: &PolicyConfig) -> Result<(), PolicyError> {
    validate(config)
}

/// Build the ABC investor/manager/recovery preset (4-year timelock).
pub fn abc_preset(
    investor: KeyConfig,
    manager: KeyConfig,
    recovery: KeyConfig,
    inherit_after_years: u32,
    network: NetworkName,
) -> PolicyConfig {
    PolicyConfig {
        version: 1,
        network,
        script_type: ScriptTypeName::Taproot,
        keys: vec![investor, manager, recovery],
        policy: PolicyExpression {
            primary: "(A && B) || (A && C)".to_string(),
            fallback: Some(FallbackPolicy {
                after: format!("{inherit_after_years}y"),
                allow: "A".to_string(),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KeyRole;
    use crate::test_vectors::{TEST_FP, TEST_XPUB_A, TEST_XPUB_B, TEST_XPUB_C};

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
    fn abc_preset_produces_valid_policy_string() {
        let keys = sample_keys();
        let config = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let policy = compile_policy_string(&config).unwrap();
        assert!(policy.contains("or("));
        assert!(policy.contains("and("));
        assert!(policy.contains(&format!("older({})", 4 * blocks_per_year())));
    }

    #[test]
    fn compile_miniscript_succeeds_for_abc_preset() {
        let keys = sample_keys();
        let config = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let ms = compile_miniscript(&config).unwrap();
        assert!(!format!("{ms}").is_empty());
    }

    #[test]
    fn rejects_unknown_key_in_expression() {
        let keys = sample_keys();
        let config = PolicyConfig {
            version: 1,
            network: NetworkName::Testnet,
            script_type: ScriptTypeName::Taproot,
            keys: keys.into(),
            policy: PolicyExpression {
                primary: "A && Z".into(),
                fallback: None,
            },
        };
        let err = compile_policy_string(&config).unwrap_err();
        assert!(matches!(err, PolicyError::UnknownKey(_)));
    }

    #[test]
    fn rejects_invalid_expression() {
        let err = parse_expression("A &&").unwrap_err();
        assert!(matches!(err, PolicyError::InvalidExpression(_)));
    }

    #[test]
    fn two_of_three_preset_compiles() {
        let config = PolicyConfig {
            version: 1,
            network: NetworkName::Testnet,
            script_type: ScriptTypeName::Taproot,
            keys: sample_keys().into(),
            policy: PolicyExpression {
                primary: "(A && B) || (A && C) || (B && C)".into(),
                fallback: None,
            },
        };
        compile_miniscript(&config).unwrap();
    }
}
