//! Named policy templates for Phase 2 vault creation.

use crate::config::{
    FallbackPolicy, KeyConfig, NetworkName, PolicyConfig, PolicyExpression, ScriptTypeName,
    POLICY_SCHEMA_VERSION,
};
use crate::error::PolicyError;

/// Identifier for a built-in policy template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateId {
    /// Classic ABC: `(A && B) || (A && C)` + investor inheritance fallback.
    Abc,
    /// Any 2-of-3: `(A && B) || (A && C) || (B && C)`.
    TwoOfThree,
    /// Single investor + manager, inheritance after timelock for A.
    Inheritance,
    /// Primary A alone; after silence, recovery key B (dead man's switch style).
    DeadMansSwitch,
    /// Investor + N managers: `(A && B) || (A && C) || …`
    MultiManager,
    /// Custom — caller supplies the expression.
    Custom,
}

impl TemplateId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Abc => "abc",
            Self::TwoOfThree => "two_of_three",
            Self::Inheritance => "inheritance",
            Self::DeadMansSwitch => "dead_mans_switch",
            Self::MultiManager => "multi_manager",
            Self::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "abc" => Some(Self::Abc),
            "two_of_three" | "2of3" => Some(Self::TwoOfThree),
            "inheritance" => Some(Self::Inheritance),
            "dead_mans_switch" | "dms" => Some(Self::DeadMansSwitch),
            "multi_manager" => Some(Self::MultiManager),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Abc => "ABC vault (investor + manager / recovery)",
            Self::TwoOfThree => "2-of-3 multisig",
            Self::Inheritance => "Inheritance (A+B now, A alone after delay)",
            Self::DeadMansSwitch => "Dead man's switch (A now, B after delay)",
            Self::MultiManager => "Multi-manager (A + any manager)",
            Self::Custom => "Custom expression",
        }
    }
}

/// Metadata for UI listing (no keys yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateInfo {
    pub id: TemplateId,
    pub label: &'static str,
    pub description: &'static str,
    pub min_keys: usize,
    pub default_primary: &'static str,
}

/// All templates shown in the vault wizard.
pub fn list_templates() -> Vec<TemplateInfo> {
    vec![
        TemplateInfo {
            id: TemplateId::Abc,
            label: TemplateId::Abc.label(),
            description: "Primary (A∧B)∨(A∧C); after timelock, A alone.",
            min_keys: 3,
            default_primary: "(A && B) || (A && C)",
        },
        TemplateInfo {
            id: TemplateId::TwoOfThree,
            label: TemplateId::TwoOfThree.label(),
            description: "Any two of A, B, C. Optional inheritance fallback.",
            min_keys: 3,
            default_primary: "(A && B) || (A && C) || (B && C)",
        },
        TemplateInfo {
            id: TemplateId::Inheritance,
            label: TemplateId::Inheritance.label(),
            description: "A and B spend now; after delay, A alone (heir path).",
            min_keys: 2,
            default_primary: "A && B",
        },
        TemplateInfo {
            id: TemplateId::DeadMansSwitch,
            label: TemplateId::DeadMansSwitch.label(),
            description: "A spends freely; if inactive past delay, B can sweep.",
            min_keys: 2,
            default_primary: "A",
        },
        TemplateInfo {
            id: TemplateId::MultiManager,
            label: TemplateId::MultiManager.label(),
            description: "Investor A with any one of several managers.",
            min_keys: 3,
            default_primary: "(A && B) || (A && C)",
        },
        TemplateInfo {
            id: TemplateId::Custom,
            label: TemplateId::Custom.label(),
            description: "Write your own && / || expression and recovery paths.",
            min_keys: 1,
            default_primary: "A && B",
        },
    ]
}

/// Build `(A && B) || (A && C) || …` for keys starting at index 1 (managers).
pub fn multi_manager_primary(manager_ids: &[String]) -> Result<String, PolicyError> {
    if manager_ids.is_empty() {
        return Err(PolicyError::InvalidExpression(
            "multi-manager template needs at least one manager key".into(),
        ));
    }
    let parts: Vec<String> = manager_ids
        .iter()
        .map(|id| format!("(A && {id})"))
        .collect();
    Ok(parts.join(" || "))
}

/// Assemble a [`PolicyConfig`] from a template, keys, and optional recovery paths.
pub fn build_from_template(
    template: TemplateId,
    keys: Vec<KeyConfig>,
    network: NetworkName,
    primary_override: Option<String>,
    fallbacks: Vec<FallbackPolicy>,
) -> Result<PolicyConfig, PolicyError> {
    if keys.is_empty() {
        return Err(PolicyError::EmptyKeys);
    }

    let primary = match primary_override {
        Some(p) if !p.trim().is_empty() => p,
        _ => match template {
            TemplateId::Abc => "(A && B) || (A && C)".into(),
            TemplateId::TwoOfThree => "(A && B) || (A && C) || (B && C)".into(),
            TemplateId::Inheritance => "A && B".into(),
            TemplateId::DeadMansSwitch => "A".into(),
            TemplateId::MultiManager => {
                let manager_ids: Vec<String> = keys
                    .iter()
                    .skip(1)
                    .map(|k| k.id.clone())
                    .collect();
                multi_manager_primary(&manager_ids)?
            }
            TemplateId::Custom => {
                return Err(PolicyError::InvalidExpression(
                    "custom template requires a primary expression".into(),
                ));
            }
        },
    };

    let fallbacks = if fallbacks.is_empty() {
        match template {
            TemplateId::Abc | TemplateId::Inheritance => vec![FallbackPolicy {
                after: "4y".into(),
                allow: "A".into(),
            }],
            TemplateId::DeadMansSwitch => {
                let recovery = keys
                    .iter()
                    .find(|k| k.id != "A")
                    .map(|k| k.id.as_str())
                    .unwrap_or("B");
                vec![FallbackPolicy {
                    after: "1y".into(),
                    allow: recovery.into(),
                }]
            }
            _ => Vec::new(),
        }
    } else {
        fallbacks
    };

    Ok(PolicyConfig {
        version: POLICY_SCHEMA_VERSION,
        network,
        script_type: ScriptTypeName::Taproot,
        keys,
        policy: PolicyExpression {
            primary,
            fallback: None,
            fallbacks,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KeyRole;
    use crate::test_vectors::{TEST_FP, TEST_XPUB_A, TEST_XPUB_B, TEST_XPUB_C};
    use crate::validate_config;

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

    #[test]
    fn dead_mans_switch_builds_valid_config() {
        let config = build_from_template(
            TemplateId::DeadMansSwitch,
            vec![
                key("A", KeyRole::Investor, TEST_XPUB_A),
                key("B", KeyRole::Recovery, TEST_XPUB_B),
            ],
            NetworkName::Testnet,
            None,
            vec![],
        )
        .unwrap();
        assert_eq!(config.policy.primary, "A");
        assert_eq!(config.policy.fallbacks.len(), 1);
        assert_eq!(config.policy.fallbacks[0].allow, "B");
        validate_config(&config).unwrap();
    }

    #[test]
    fn multi_manager_primary_or_chain() {
        let expr = multi_manager_primary(&["B".into(), "C".into(), "D".into()]).unwrap();
        assert_eq!(expr, "(A && B) || (A && C) || (A && D)");
    }

    #[test]
    fn two_of_three_validates() {
        let config = build_from_template(
            TemplateId::TwoOfThree,
            vec![
                key("A", KeyRole::Investor, TEST_XPUB_A),
                key("B", KeyRole::Manager, TEST_XPUB_B),
                key("C", KeyRole::Recovery, TEST_XPUB_C),
            ],
            NetworkName::Testnet,
            None,
            vec![],
        )
        .unwrap();
        validate_config(&config).unwrap();
    }
}
