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

    if let Some(fallback) = &config.policy.fallback {
        parse_duration(&fallback.after)?;
        if !config.keys.iter().any(|k| k.id == fallback.allow) {
            return Err(PolicyError::UnknownFallbackKey {
                key: fallback.allow.clone(),
            });
        }
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
