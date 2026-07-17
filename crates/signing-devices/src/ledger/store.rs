//! Persist Ledger wallet-policy HMAC per `(wallet_id, fingerprint)`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use policy_engine::NetworkName;
use serde::{Deserialize, Serialize};

use crate::error::SignError;
use crate::registration::{bip388_policy_fingerprint, Bip388Policy};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerRegistration {
    pub wallet_id: String,
    pub fingerprint: String,
    pub hmac: String,
    pub registered_at_secs: u64,
    /// SHA-256 of canonical BIP-388 policy JSON (template + keys).
    #[serde(default)]
    pub policy_fingerprint: String,
    /// Wallet network label at registration (`testnet`, `mainnet`, …).
    #[serde(default)]
    pub network: String,
}

pub fn registrations_root(data_dir: &Path) -> PathBuf {
    data_dir.join("ledger_registrations")
}

fn sanitize_wallet_id(wallet_id: &str) -> String {
    wallet_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn network_storage_label(network: NetworkName) -> &'static str {
    match network {
        NetworkName::Mainnet => "mainnet",
        NetworkName::Testnet => "testnet",
        NetworkName::Testnet4 => "testnet4",
        NetworkName::Signet => "signet",
        NetworkName::Regtest => "regtest",
    }
}

pub fn registration_path(data_dir: &Path, wallet_id: &str, fingerprint: &str) -> PathBuf {
    registrations_root(data_dir)
        .join(sanitize_wallet_id(wallet_id))
        .join(format!("{}.json", fingerprint.trim().to_ascii_lowercase()))
}

pub fn load_registration(
    data_dir: &Path,
    wallet_id: &str,
    fingerprint: &str,
) -> Option<LedgerRegistration> {
    let path = registration_path(data_dir, wallet_id, fingerprint);
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_registration(
    data_dir: &Path,
    wallet_id: &str,
    fingerprint: &str,
    hmac: &str,
    bip388: &Bip388Policy,
    network: NetworkName,
) -> Result<LedgerRegistration, SignError> {
    let path = registration_path(data_dir, wallet_id, fingerprint);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| SignError::Ledger(format!("create dir: {e}")))?;
    }
    let reg = LedgerRegistration {
        wallet_id: wallet_id.to_string(),
        fingerprint: fingerprint.trim().to_ascii_lowercase(),
        hmac: hmac.trim().to_ascii_lowercase(),
        registered_at_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        policy_fingerprint: bip388_policy_fingerprint(bip388),
        network: network_storage_label(network).to_string(),
    };
    let json = serde_json::to_string_pretty(&reg)
        .map_err(|e| SignError::Ledger(format!("serialize registration: {e}")))?;
    fs::write(&path, json).map_err(|e| SignError::Ledger(format!("write registration: {e}")))?;
    Ok(reg)
}

/// `None` when registration matches the current wallet policy; otherwise a short reason.
pub fn registration_stale_reason(
    reg: &LedgerRegistration,
    bip388: &Bip388Policy,
    network: NetworkName,
) -> Option<&'static str> {
    if reg.policy_fingerprint.is_empty() {
        return Some("registration from an older app version — re-register on Ledger");
    }
    let current = bip388_policy_fingerprint(bip388);
    if reg.policy_fingerprint != current {
        return Some("wallet descriptor or keys changed since registration");
    }
    let want = network_storage_label(network);
    if !reg.network.is_empty() && reg.network != want {
        return Some("wallet network changed since registration");
    }
    None
}

pub fn is_registered(
    data_dir: &Path,
    wallet_id: &str,
    fingerprint: &str,
    bip388: &Bip388Policy,
    network: NetworkName,
) -> bool {
    load_registration(data_dir, wallet_id, fingerprint)
        .is_some_and(|reg| registration_stale_reason(&reg, bip388, network).is_none())
}

pub fn delete_registration(
    data_dir: &Path,
    wallet_id: &str,
    fingerprint: &str,
) -> Result<(), SignError> {
    let path = registration_path(data_dir, wallet_id, fingerprint);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| SignError::Ledger(format!("delete registration: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registration::Bip388Policy;

    fn sample_bip388() -> Bip388Policy {
        Bip388Policy {
            name: "ABC".into(),
            policy: "tr(@0)".into(),
            keys: vec!["[a98a1256/86'/1'/0']xpubtest".into()],
        }
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let bip = sample_bip388();
        save_registration(
            dir.path(),
            "wallet-1",
            "a98a1256",
            &"ab".repeat(32),
            &bip,
            NetworkName::Testnet,
        )
        .unwrap();
        let reg = load_registration(dir.path(), "wallet-1", "A98A1256").unwrap();
        assert_eq!(reg.fingerprint, "a98a1256");
        assert_eq!(reg.hmac.len(), 64);
        assert!(!reg.policy_fingerprint.is_empty());
        assert_eq!(reg.network, "testnet");
    }

    #[test]
    fn detects_stale_after_policy_change() {
        let dir = tempfile::tempdir().unwrap();
        let mut bip = sample_bip388();
        save_registration(
            dir.path(),
            "w",
            "a98a1256",
            &"ab".repeat(32),
            &bip,
            NetworkName::Testnet,
        )
        .unwrap();
        let reg = load_registration(dir.path(), "w", "a98a1256").unwrap();
        bip.policy.push_str(",@1");
        assert!(registration_stale_reason(&reg, &bip, NetworkName::Testnet).is_some());
    }

    #[test]
    fn legacy_registration_without_fingerprint_is_stale() {
        let reg = LedgerRegistration {
            wallet_id: "w".into(),
            fingerprint: "a98a1256".into(),
            hmac: "aa".repeat(32),
            registered_at_secs: 0,
            policy_fingerprint: String::new(),
            network: "testnet".into(),
        };
        let bip = sample_bip388();
        assert!(registration_stale_reason(&reg, &bip, NetworkName::Testnet).is_some());
    }
}
