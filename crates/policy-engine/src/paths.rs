//! Enumerate human-readable spending paths from a policy (for UX).

use serde::{Deserialize, Serialize};

use crate::config::PolicyConfig;
use crate::error::PolicyError;
use crate::timelock::parse_duration;

/// One alternative way to satisfy the vault (primary branch or timelocked fallback).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpendingPath {
    pub id: String,
    pub label: String,
    /// Key ids required on this path (e.g. `["A","B"]`).
    pub required_keys: Vec<String>,
    /// Relative locktime in blocks when this path uses `older(N)`.
    pub timelock_blocks: Option<u32>,
    /// Suggested BIP68 `nSequence` (block-based CSV) for this path.
    pub suggested_sequence: Option<u32>,
    pub kind: SpendingPathKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpendingPathKind {
    Primary,
    Fallback,
}

/// List primary OR-branches and timelocked fallbacks for Send UX.
pub fn spending_paths(config: &PolicyConfig) -> Result<Vec<SpendingPath>, PolicyError> {
    let mut paths = Vec::new();
    let primary = config.policy.primary.trim();
    if !primary.is_empty() {
        let branches = split_top_level_or(primary);
        for (i, branch) in branches.iter().enumerate() {
            let keys = keys_in_expr(branch, config);
            if keys.is_empty() {
                continue;
            }
            let label = format!("Primary · {}", keys.join(" + "));
            paths.push(SpendingPath {
                id: format!("primary-{i}"),
                label,
                required_keys: keys,
                timelock_blocks: None,
                suggested_sequence: None,
                kind: SpendingPathKind::Primary,
            });
        }
    }

    for (i, fb) in config.policy.all_fallbacks().into_iter().enumerate() {
        let blocks = parse_duration(&fb.after)?;
        let keys = keys_in_expr(&fb.allow, config);
        let key_label = if keys.is_empty() {
            fb.allow.clone()
        } else {
            keys.join(" + ")
        };
        paths.push(SpendingPath {
            id: format!("fallback-{i}"),
            label: format!("After {} · {key_label}", fb.after),
            required_keys: keys,
            timelock_blocks: Some(blocks),
            suggested_sequence: Some(blocks),
            kind: SpendingPathKind::Fallback,
        });
    }

    Ok(paths)
}

/// BIP68 relative lock-time value for a block-based CSV (`older(N)`).
pub fn bip68_relative_blocks(blocks: u32) -> u32 {
    // Keep only 16-bit block count; clear disable / type flags (block-based).
    blocks & 0xffff
}

fn split_top_level_or(expr: &str) -> Vec<String> {
    let trimmed = strip_outer_parens(expr.trim());
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let bytes = trimmed.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b'|' if depth == 0 && i + 1 < bytes.len() && bytes[i + 1] == b'|' => {
                parts.push(trimmed[start..i].trim().to_string());
                i += 2;
                start = i;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(trimmed[start..].trim().to_string());
    parts.retain(|p| !p.is_empty());
    if parts.is_empty() {
        vec![trimmed.to_string()]
    } else {
        parts
    }
}

fn strip_outer_parens(expr: &str) -> &str {
    let mut s = expr.trim();
    while s.starts_with('(') && s.ends_with(')') && balanced_wrap(s) {
        s = s[1..s.len() - 1].trim();
    }
    s
}

fn balanced_wrap(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'(' || bytes[bytes.len() - 1] != b')' {
        return false;
    }
    let mut depth = 0i32;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return i == bytes.len() - 1;
                }
            }
            _ => {}
        }
    }
    false
}

fn keys_in_expr(expr: &str, config: &PolicyConfig) -> Vec<String> {
    let upper = expr.to_ascii_uppercase();
    let mut found = Vec::new();
    for key in &config.keys {
        let id = key.id.to_ascii_uppercase();
        // Token boundary: not part of a longer alphanumeric id.
        if contains_key_token(&upper, &id) && !found.iter().any(|k: &String| k.eq_ignore_ascii_case(&key.id))
        {
            found.push(key.id.clone());
        }
    }
    found
}

fn contains_key_token(haystack: &str, id: &str) -> bool {
    let h = haystack.as_bytes();
    let needle = id.as_bytes();
    if needle.is_empty() {
        return false;
    }
    let mut i = 0usize;
    while i + needle.len() <= h.len() {
        if h[i..i + needle.len()].eq_ignore_ascii_case(needle) {
            let before_ok = i == 0 || !h[i - 1].is_ascii_alphanumeric();
            let after = i + needle.len();
            let after_ok = after >= h.len() || !h[after].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        FallbackPolicy, KeyConfig, KeyRole, NetworkName, PolicyExpression, ScriptTypeName,
        POLICY_SCHEMA_VERSION,
    };

    fn abc_config() -> PolicyConfig {
        PolicyConfig {
            version: POLICY_SCHEMA_VERSION,
            network: NetworkName::Testnet,
            script_type: ScriptTypeName::Taproot,
            keys: vec![
                KeyConfig {
                    id: "A".into(),
                    role: KeyRole::Investor,
                    xpub: "x".into(),
                    fingerprint: "aaaaaaaa".into(),
                    origin_path: None,
                },
                KeyConfig {
                    id: "B".into(),
                    role: KeyRole::Manager,
                    xpub: "y".into(),
                    fingerprint: "bbbbbbbb".into(),
                    origin_path: None,
                },
                KeyConfig {
                    id: "C".into(),
                    role: KeyRole::Recovery,
                    xpub: "z".into(),
                    fingerprint: "cccccccc".into(),
                    origin_path: None,
                },
            ],
            policy: PolicyExpression {
                primary: "(A && B) || (A && C)".into(),
                fallback: Some(FallbackPolicy {
                    after: "4y".into(),
                    allow: "A".into(),
                }),
                fallbacks: vec![],
            },
        }
    }

    #[test]
    fn splits_abc_primary_and_fallback() {
        let paths = spending_paths(&abc_config()).unwrap();
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0].required_keys, vec!["A", "B"]);
        assert_eq!(paths[1].required_keys, vec!["A", "C"]);
        assert_eq!(paths[2].kind, SpendingPathKind::Fallback);
        assert_eq!(paths[2].timelock_blocks, Some(4 * 52_560));
        assert_eq!(paths[2].suggested_sequence, Some(4 * 52_560));
    }

    #[test]
    fn bip68_masks_to_16_bits() {
        assert_eq!(bip68_relative_blocks(210_240), 210_240 & 0xffff);
    }
}
