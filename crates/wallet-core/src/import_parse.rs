//! Best-effort watch-only payload parsing (descriptor / backup / BSMS / Liana-ish JSON).

use descriptor_engine::ensure_descriptor_checksum;
use policy_engine::{NetworkName, PolicyConfig};
use serde_json::Value;

use crate::backup::WalletBackup;
use crate::error::WalletError;
use crate::types::network_from_str;

/// Where a watch-only import payload came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportSource {
    MinisatoshiBackup,
    BareDescriptor,
    Bsms,
    GenericJson,
}

/// Normalized watch-only import after parsing third-party formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedWatchOnlyImport {
    pub name: Option<String>,
    pub network: Option<NetworkName>,
    pub descriptor: String,
    pub policy: Option<PolicyConfig>,
    pub source: ImportSource,
}

/// Parse paste/file payloads into a checksummed descriptor (+ optional policy).
///
/// Supported (best-effort):
/// - `minisatoshi-vault-v1` JSON
/// - bare `tr(…)#…` / `wsh(…)#…` (checksum computed if missing)
/// - BIP-129 BSMS 1.0 descriptor records
/// - JSON with `descriptor` / `main_descriptor` / `receive_descriptor` (Liana/Nunchuk-ish)
pub fn parse_watch_only_payload(raw: &str) -> Result<ParsedWatchOnlyImport, WalletError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(WalletError::InvalidDescriptor("payload is empty".into()));
    }

    if looks_like_bsms(trimmed) {
        return parse_bsms(trimmed);
    }

    if trimmed.starts_with('{') {
        if let Ok(backup) = WalletBackup::from_json(trimmed) {
            if backup.is_supported_format() {
                let descriptor = ensure_checksummed(&backup.descriptor)?;
                return Ok(ParsedWatchOnlyImport {
                    name: Some(backup.name),
                    network: Some(backup.network),
                    descriptor,
                    policy: backup.policy,
                    source: ImportSource::MinisatoshiBackup,
                });
            }
        }
        return parse_generic_json(trimmed);
    }

    // Multi-line paste that is not BSMS — take first descriptor-looking line.
    if let Some(desc) = extract_descriptor_line(trimmed) {
        return Ok(ParsedWatchOnlyImport {
            name: None,
            network: None,
            descriptor: ensure_checksummed(&desc)?,
            policy: None,
            source: ImportSource::BareDescriptor,
        });
    }

    Err(WalletError::InvalidDescriptor(
        "unrecognized watch-only payload — paste a checksummed descriptor, minisatoshi-vault-v1.json, or BSMS 1.0 file".into(),
    ))
}

/// BIP-129-ish descriptor record export (watch-only; no key records).
pub fn format_bsms(descriptor: &str, first_address: &str) -> String {
    let body = strip_checksum(descriptor.trim());
    format!(
        "BSMS 1.0\n{body}\nNo path restrictions\n{}\n",
        first_address.trim()
    )
}

fn looks_like_bsms(raw: &str) -> bool {
    raw.lines()
        .next()
        .map(|line| line.trim().eq_ignore_ascii_case("BSMS 1.0"))
        .unwrap_or(false)
}

fn parse_bsms(raw: &str) -> Result<ParsedWatchOnlyImport, WalletError> {
    let lines: Vec<&str> = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() < 2 {
        return Err(WalletError::InvalidDescriptor(
            "BSMS file is missing descriptor line".into(),
        ));
    }
    let mut template = lines[1].to_string();
    let restrictions = lines.get(2).copied().unwrap_or("No path restrictions");
    if template.contains("/**") {
        template = expand_bsms_template(&template, restrictions)?;
    }
    Ok(ParsedWatchOnlyImport {
        name: Some("BSMS import".into()),
        network: None,
        descriptor: ensure_checksummed(&template)?,
        policy: None,
        source: ImportSource::Bsms,
    })
}

fn expand_bsms_template(template: &str, restrictions: &str) -> Result<String, WalletError> {
    if restrictions.eq_ignore_ascii_case("No path restrictions") {
        // Prefer Core/Nunchuk multipath when no explicit paths given.
        return Ok(template.replace("/**", "/<0;1>/*"));
    }
    let paths: Vec<&str> = restrictions
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    // Prefer multipath when both receive+change are listed.
    if paths.len() >= 2
        && paths.iter().any(|p| p.starts_with("/0"))
        && paths.iter().any(|p| p.starts_with("/1"))
    {
        return Ok(template.replace("/**", "/<0;1>/*"));
    }
    if let Some(first) = paths.first() {
        // Strip trailing /* from "/0/*" → "/0/*" already; use as suffix after dropping /**
        let suffix = if first.ends_with("/*") {
            first.to_string()
        } else {
            format!("{first}/*")
        };
        return Ok(template.replace("/**", &suffix));
    }
    Err(WalletError::InvalidDescriptor(format!(
        "unsupported BSMS path restrictions: {restrictions}"
    )))
}

fn parse_generic_json(raw: &str) -> Result<ParsedWatchOnlyImport, WalletError> {
    let value: Value = serde_json::from_str(raw)?;
    let obj = value
        .as_object()
        .ok_or_else(|| WalletError::InvalidDescriptor("JSON root must be an object".into()))?;

    let descriptor = [
        "descriptor",
        "main_descriptor",
        "mainDescriptor",
        "receive_descriptor",
        "receiveDescriptor",
        "desc",
    ]
    .iter()
    .find_map(|key| {
        obj.get(*key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
    })
    .ok_or_else(|| {
        WalletError::InvalidDescriptor(
            "JSON has no descriptor field (tried descriptor / main_descriptor / receive_descriptor)"
                .into(),
        )
    })?;

    let name = ["name", "wallet_name", "walletName", "label"]
        .iter()
        .find_map(|key| {
            obj.get(*key)
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        });

    let network = ["network", "chain"]
        .iter()
        .find_map(|key| {
            obj.get(*key)
                .and_then(|v| v.as_str())
                .and_then(|s| network_from_str(s).ok())
        });

    let policy = obj
        .get("policy")
        .cloned()
        .and_then(|v| serde_json::from_value::<PolicyConfig>(v).ok());

    Ok(ParsedWatchOnlyImport {
        name,
        network,
        descriptor: ensure_checksummed(descriptor)?,
        policy,
        source: ImportSource::GenericJson,
    })
}

fn extract_descriptor_line(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let line = line.trim();
        if line.starts_with("tr(") || line.starts_with("wsh(") || line.starts_with("wpkh(") {
            return Some(line.to_string());
        }
    }
    let trimmed = raw.trim();
    if trimmed.starts_with("tr(") || trimmed.starts_with("wsh(") {
        return Some(trimmed.to_string());
    }
    None
}

fn ensure_checksummed(descriptor: &str) -> Result<String, WalletError> {
    ensure_descriptor_checksum(descriptor.trim()).map_err(|e| {
        WalletError::InvalidDescriptor(format!("checksum or descriptor invalid: {e}"))
    })
}

fn strip_checksum(descriptor: &str) -> &str {
    descriptor
        .rsplit_once('#')
        .map(|(body, _)| body)
        .unwrap_or(descriptor)
        .trim()
}

#[cfg(test)]
mod tests {
    use super::*;
    use policy_engine::{
        abc_preset, test_vectors::TEST_FP, test_vectors::TEST_XPUB_A, test_vectors::TEST_XPUB_B,
        test_vectors::TEST_XPUB_C, KeyConfig, KeyRole, NetworkName,
    };
    use descriptor_engine::compile_descriptor_from_config;

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
                fingerprint: "aaaaaaaa".into(),
                origin_path: Some("86'/0'/1'".into()),
            },
        ]
    }

    #[test]
    fn parse_bare_descriptor_adds_checksum_when_missing() {
        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let with_sum = compile_descriptor_from_config(&policy).unwrap();
        let (body, _) = with_sum.rsplit_once('#').unwrap();
        let parsed = parse_watch_only_payload(body).unwrap();
        assert_eq!(parsed.source, ImportSource::BareDescriptor);
        assert!(parsed.descriptor.contains('#'));
        assert_eq!(parsed.descriptor, with_sum);
    }

    #[test]
    fn parse_bsms_expands_multipath() {
        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let desc = compile_descriptor_from_config(&policy).unwrap();
        let body = desc.rsplit_once('#').unwrap().0;
        // Synthesize a template form if body already uses /<0;1>/* — just wrap as BSMS
        let bsms = format!("BSMS 1.0\n{body}\nNo path restrictions\ntb1qtest\n");
        let parsed = parse_watch_only_payload(&bsms).unwrap();
        assert_eq!(parsed.source, ImportSource::Bsms);
        assert!(parsed.descriptor.starts_with("tr(") || parsed.descriptor.starts_with("wsh("));
        assert!(parsed.descriptor.contains('#'));
    }

    #[test]
    fn parse_bsms_template_with_restrictions() {
        let template = "wsh(sortedmulti(2,xpubAAA/**,xpubBBB/**))";
        let expanded = expand_bsms_template(template, "/0/*,/1/*").unwrap();
        assert!(expanded.contains("/<0;1>/*"));
        assert!(!expanded.contains("/**"));
    }

    #[test]
    fn parse_liana_ish_json() {
        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let desc = compile_descriptor_from_config(&policy).unwrap();
        let json = format!(
            r#"{{"name":"Liana vault","network":"testnet","descriptor":"{desc}"}}"#
        );
        let parsed = parse_watch_only_payload(&json).unwrap();
        assert_eq!(parsed.source, ImportSource::GenericJson);
        assert_eq!(parsed.name.as_deref(), Some("Liana vault"));
        assert_eq!(parsed.network, Some(NetworkName::Testnet));
        assert_eq!(parsed.descriptor, desc);
    }

    #[test]
    fn format_bsms_omits_checksum_on_body() {
        let out = format_bsms("tr(x)#abcdef12", "tb1qabc");
        assert!(out.starts_with("BSMS 1.0\n"));
        assert!(out.contains("\ntr(x)\n"));
        assert!(!out.contains("tr(x)#"));
        assert!(out.contains("tb1qabc"));
    }
}
