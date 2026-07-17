//! Ledger Bitcoin app readiness checks (app name, version, policy constraints).

use policy_engine::NetworkName;
use serde::{Deserialize, Serialize};

/// Minimum Bitcoin app version for Taproot Miniscript wallet policies (ABC script-path).
pub const MIN_BITCOIN_APP_TAPROOT_MINISCRIPT: (u32, u32, u32) = (2, 2, 1);
/// Bitcoin app ≥ 2.4.6 rejects `older(n)` when `n` is not a valid BIP68 block count.
pub const BITCOIN_APP_STRICT_OLDER_CHECK: (u32, u32, u32) = (2, 4, 6);
/// BIP68 block-based relative locktime uses only the low 16 bits (max 65535 blocks).
pub const BIP68_MAX_BLOCK_RELATIVE: u32 = 65535;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerReadiness {
    pub device_connected: bool,
    pub app_name: Option<String>,
    pub app_version: Option<String>,
    pub expected_app_name: String,
    pub warnings: Vec<String>,
    pub ready: bool,
}

pub fn expected_bitcoin_app_name(network: NetworkName) -> &'static str {
    match network {
        NetworkName::Mainnet => "Bitcoin",
        NetworkName::Testnet
        | NetworkName::Testnet4
        | NetworkName::Signet
        | NetworkName::Regtest => "Bitcoin Test",
    }
}

pub fn parse_app_version(raw: &str) -> Option<(u32, u32, u32)> {
    let mut parts = raw.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    Some((major, minor, patch))
}

fn version_lt(a: (u32, u32, u32), b: (u32, u32, u32)) -> bool {
    a < b
}

fn version_gte(a: (u32, u32, u32), b: (u32, u32, u32)) -> bool {
    !version_lt(a, b)
}

/// Collect `older(n)` values in a wallet policy template that exceed BIP68 block limits.
pub fn invalid_older_blocks_in_policy(policy: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let mut rest = policy;
    while let Some(idx) = rest.find("older(") {
        let after = &rest[idx + "older(".len()..];
        let Some(end) = after.find(')') else {
            break;
        };
        if let Ok(n) = after[..end].trim().parse::<u32>() {
            if n > BIP68_MAX_BLOCK_RELATIVE {
                out.push(n);
            }
        }
        rest = &after[end..];
    }
    out
}

pub fn evaluate_ledger_readiness(
    network: NetworkName,
    device: Option<(&str, &str)>,
    policy: Option<&str>,
) -> LedgerReadiness {
    let expected = expected_bitcoin_app_name(network).to_string();
    let mut warnings = Vec::new();
    let mut blocking = false;

    if let Some(policy) = policy {
        for n in invalid_older_blocks_in_policy(policy) {
            warnings.push(format!(
                "Policy uses older({n}) — block timelocks above {BIP68_MAX_BLOCK_RELATIVE} are rejected by Bitcoin app ≥ {}.{}.{}.",
                BITCOIN_APP_STRICT_OLDER_CHECK.0,
                BITCOIN_APP_STRICT_OLDER_CHECK.1,
                BITCOIN_APP_STRICT_OLDER_CHECK.2,
            ));
            if let Some((_, ver_raw)) = device {
                if let Some(ver) = parse_app_version(ver_raw) {
                    if version_gte(ver, BITCOIN_APP_STRICT_OLDER_CHECK) {
                        blocking = true;
                    }
                }
            }
        }
    }

    let (device_connected, app_name, app_version) = match device {
        Some((name, version)) => (true, Some(name.to_string()), Some(version.to_string())),
        None => {
            warnings.push(
                "Ledger not detected — unlock the device, open the correct Bitcoin app, and reconnect USB."
                    .into(),
            );
            blocking = true;
            (false, None, None)
        }
    };

    if let Some(name) = app_name.as_deref() {
        if name == "Bitcoin Legacy" || name == "Bitcoin Test Legacy" {
            warnings.push(
                "Legacy Bitcoin app is open — install and open the new Bitcoin app (wallet policies are unsupported)."
                    .into(),
            );
            blocking = true;
        } else if name != expected.as_str() {
            warnings.push(format!(
                "Wrong app open: \"{name}\" — open \"{expected}\" for this wallet network."
            ));
            blocking = true;
        }

        if let Some(ver_raw) = app_version.as_deref() {
            if let Some(ver) = parse_app_version(ver_raw) {
                if version_lt(ver, MIN_BITCOIN_APP_TAPROOT_MINISCRIPT) {
                    warnings.push(format!(
                        "Bitcoin app {ver_raw} is too old for Taproot Miniscript — update to ≥ {}.{}.{} in Ledger Live.",
                        MIN_BITCOIN_APP_TAPROOT_MINISCRIPT.0,
                        MIN_BITCOIN_APP_TAPROOT_MINISCRIPT.1,
                        MIN_BITCOIN_APP_TAPROOT_MINISCRIPT.2,
                    ));
                    blocking = true;
                } else if version_lt(ver, (2, 2, 2)) {
                    warnings.push(
                        "Bitcoin app < 2.2.2 — NUMS internal keys may not show as \"dummy\" during registration (update recommended)."
                            .into(),
                    );
                }
            } else {
                warnings.push(format!(
                    "Could not parse Bitcoin app version \"{ver_raw}\" — ensure firmware is up to date."
                ));
            }
        }
    }

    LedgerReadiness {
        device_connected,
        app_name,
        app_version,
        expected_app_name: expected,
        warnings,
        ready: !blocking,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expects_bitcoin_test_on_testnet() {
        assert_eq!(
            expected_bitcoin_app_name(NetworkName::Testnet),
            "Bitcoin Test"
        );
    }

    #[test]
    fn flags_wrong_app_and_old_version() {
        let r = evaluate_ledger_readiness(
            NetworkName::Testnet,
            Some(("Bitcoin", "2.1.0")),
            None,
        );
        assert!(!r.ready);
        assert!(r.warnings.iter().any(|w| w.contains("Wrong app")));
        assert!(r.warnings.iter().any(|w| w.contains("too old")));
    }

    #[test]
    fn warns_older_210240_on_new_app_blocks_register() {
        let policy = "tr(@0/**,{and_v(v:pk(@1/**),older(210240))})";
        let r = evaluate_ledger_readiness(
            NetworkName::Testnet,
            Some(("Bitcoin Test", "2.4.6")),
            Some(policy),
        );
        assert!(!r.ready);
        assert!(r.warnings.iter().any(|w| w.contains("older(210240)")));
    }

    #[test]
    fn warns_older_210240_on_old_app_still_ready() {
        let policy = "tr(@0/**,{and_v(v:pk(@1/**),older(210240))})";
        let r = evaluate_ledger_readiness(
            NetworkName::Testnet,
            Some(("Bitcoin Test", "2.4.1")),
            Some(policy),
        );
        assert!(r.ready);
        assert!(r.warnings.iter().any(|w| w.contains("older(210240)")));
    }

    #[test]
    fn ready_when_app_and_version_ok() {
        let r = evaluate_ledger_readiness(
            NetworkName::Testnet,
            Some(("Bitcoin Test", "2.4.1")),
            Some("tr(@0/**)"),
        );
        assert!(r.ready);
        assert!(r.warnings.is_empty());
    }
}
