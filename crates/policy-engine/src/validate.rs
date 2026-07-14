use std::collections::HashSet;

use crate::config::{KeyConfig, PolicyConfig, POLICY_SCHEMA_VERSION};
use crate::error::PolicyError;
use crate::parser::{self, Expr};
use crate::timelock::parse_duration;

pub fn validate(config: &PolicyConfig) -> Result<(), PolicyError> {
    if config.version != POLICY_SCHEMA_VERSION {
        return Err(PolicyError::UnsupportedVersion(config.version));
    }

    if config.keys.is_empty() {
        return Err(PolicyError::EmptyKeys);
    }

    let mut seen = HashSet::new();
    for key in &config.keys {
        if !seen.insert(&key.id) {
            return Err(PolicyError::DuplicateKeyId(key.id.clone()));
        }
        validate_fingerprint(key)?;
        validate_xpub(key)?;
    }

    let primary_ast = parser::parse_expression(&config.policy.primary)?;
    validate_expr_keys(&primary_ast, config)?;

    for fallback in config.policy.all_fallbacks() {
        parse_duration(&fallback.after)?;
        if fallback.allow.trim().is_empty() {
            return Err(PolicyError::EmptyFallbackAllow);
        }
        let allow_ast = parser::parse_expression(&fallback.allow).map_err(|e| {
            PolicyError::InvalidExpression(format!("fallback allow: {e}"))
        })?;
        validate_expr_keys(&allow_ast, config)?;
    }

    Ok(())
}

fn validate_fingerprint(key: &KeyConfig) -> Result<(), PolicyError> {
    let fp = key.fingerprint.trim();
    if fp.len() != 8 || !fp.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(PolicyError::InvalidFingerprint {
            key: key.id.clone(),
            reason: "fingerprint must be 8 hex characters".into(),
        });
    }
    Ok(())
}

fn validate_xpub(key: &KeyConfig) -> Result<(), PolicyError> {
    let xpub = key.xpub.trim();
    let valid_prefix = matches!(
        xpub.split_at_checked(4).map(|(p, _)| p),
        Some("xpub" | "tpub" | "ypub" | "upub" | "zpub" | "vpub")
    );
    if !valid_prefix || xpub.len() < 100 {
        return Err(PolicyError::InvalidXpub {
            key: key.id.clone(),
            reason: "expected extended public key (xpub/tpub/...)".into(),
        });
    }
    Ok(())
}

fn validate_expr_keys(expr: &Expr, config: &PolicyConfig) -> Result<(), PolicyError> {
    match expr {
        Expr::Key(id) => {
            if !config.keys.iter().any(|k| k.id == *id) {
                return Err(PolicyError::UnknownKey(id.clone()));
            }
        }
        Expr::And(left, right) | Expr::Or(left, right) => {
            validate_expr_keys(left, config)?;
            validate_expr_keys(right, config)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        FallbackPolicy, KeyConfig, KeyRole, NetworkName, PolicyExpression, ScriptTypeName,
    };
    use crate::test_vectors::{TEST_FP, TEST_XPUB_A, TEST_XPUB_B};

    fn base_config(keys: Vec<KeyConfig>) -> PolicyConfig {
        PolicyConfig {
            version: POLICY_SCHEMA_VERSION,
            network: NetworkName::Testnet,
            script_type: ScriptTypeName::Taproot,
            keys,
            policy: PolicyExpression {
                primary: "A && B".into(),
                fallback: None,
                fallbacks: vec![],
            },
        }
    }

    fn key(id: &str, xpub: &str, fingerprint: &str) -> KeyConfig {
        KeyConfig {
            id: id.into(),
            role: KeyRole::Investor,
            xpub: xpub.into(),
            fingerprint: fingerprint.into(),
            origin_path: None,
        }
    }

    #[test]
    fn rejects_invalid_fingerprint() {
        let config = base_config(vec![
            key("A", TEST_XPUB_A, "zzz"),
            key("B", TEST_XPUB_B, TEST_FP),
        ]);
        assert!(matches!(
            validate(&config),
            Err(PolicyError::InvalidFingerprint { .. })
        ));
    }

    #[test]
    fn rejects_invalid_xpub_prefix() {
        let config = base_config(vec![
            key("A", "not-an-xpub", "78412e3a"),
            key("B", TEST_XPUB_B, TEST_FP),
        ]);
        assert!(matches!(
            validate(&config),
            Err(PolicyError::InvalidXpub { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_key_id() {
        let config = base_config(vec![
            key("A", TEST_XPUB_A, "78412e3a"),
            key("A", TEST_XPUB_B, TEST_FP),
        ]);
        assert_eq!(
            validate(&config),
            Err(PolicyError::DuplicateKeyId("A".into()))
        );
    }

    #[test]
    fn rejects_unsupported_version() {
        let mut config = base_config(vec![
            key("A", TEST_XPUB_A, "78412e3a"),
            key("B", TEST_XPUB_B, TEST_FP),
        ]);
        config.version = 99;
        assert_eq!(validate(&config), Err(PolicyError::UnsupportedVersion(99)));
    }

    #[test]
    fn rejects_unknown_fallback_key() {
        let mut config = base_config(vec![
            key("A", TEST_XPUB_A, "78412e3a"),
            key("B", TEST_XPUB_B, TEST_FP),
        ]);
        config.policy.fallback = Some(FallbackPolicy {
            after: "4y".into(),
            allow: "Z".into(),
        });
        assert!(matches!(
            validate(&config),
            Err(PolicyError::UnknownKey(_))
        ));
    }

    #[test]
    fn accepts_multiple_fallbacks_and_allow_expression() {
        let mut config = base_config(vec![
            key("A", TEST_XPUB_A, "78412e3a"),
            key("B", TEST_XPUB_B, TEST_FP),
        ]);
        config.policy.fallback = None;
        config.policy.fallbacks = vec![
            FallbackPolicy {
                after: "1y".into(),
                allow: "A".into(),
            },
            FallbackPolicy {
                after: "4y".into(),
                allow: "A && B".into(),
            },
        ];
        validate(&config).unwrap();
    }

    #[test]
    fn accepts_valid_abc_config() {
        let config = base_config(vec![
            key("A", TEST_XPUB_A, "78412e3a"),
            key("B", TEST_XPUB_B, TEST_FP),
        ]);
        validate(&config).unwrap();
    }
}
