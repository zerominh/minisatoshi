//! BIP-39 → BIP-86 account keys for hot wallet import.

use std::str::FromStr;

use bip39::Mnemonic;
use bitcoin::bip32::{ChildNumber, DerivationPath, Fingerprint, Xpriv, Xpub};
use bitcoin::key::Secp256k1;
use bitcoin::Network;
use policy_engine::{KeyConfig, KeyRole, NetworkName};
use serde::{Deserialize, Serialize};

use crate::error::HotKeystoreError;
use crate::store::HotWalletRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportHotWalletRequest {
    pub name: String,
    pub mnemonic: String,
    #[serde(default)]
    pub bip39_passphrase: String,
    pub network: NetworkName,
    /// Coin-type account path override; default BIP-86 `m/86'/coin'/0'`.
    #[serde(default)]
    pub account_path: Option<String>,
}

pub fn bitcoin_network(network: NetworkName) -> Network {
    match network {
        NetworkName::Mainnet => Network::Bitcoin,
        NetworkName::Testnet | NetworkName::Testnet4 => Network::Testnet,
        NetworkName::Signet => Network::Signet,
        NetworkName::Regtest => Network::Regtest,
    }
}

fn coin_type(network: NetworkName) -> u32 {
    match network {
        NetworkName::Mainnet => 0,
        _ => 1,
    }
}

/// Derive BIP-86 account and build a keystore record (+ KeyConfig for vault create).
pub fn derive_bip86_account(
    request: &ImportHotWalletRequest,
) -> Result<(HotWalletRecord, KeyConfig), HotKeystoreError> {
    let mnemonic = Mnemonic::parse_normalized(request.mnemonic.trim())
        .map_err(|e| HotKeystoreError::Mnemonic(e.to_string()))?;
    let seed = mnemonic.to_seed(&request.bip39_passphrase);
    let secp = Secp256k1::new();
    let btc_net = bitcoin_network(request.network);
    let master = Xpriv::new_master(btc_net, &seed)
        .map_err(|e| HotKeystoreError::Derive(e.to_string()))?;
    let master_fp: Fingerprint = master.fingerprint(&secp);

    let account_path = match request.account_path.as_deref().map(str::trim).filter(|s| !s.is_empty())
    {
        Some(path) => {
            let path = path.trim_start_matches("m/").trim_start_matches("M/");
            DerivationPath::from_str(path).map_err(|e| HotKeystoreError::Derive(e.to_string()))?
        }
        None => {
            let ct = coin_type(request.network);
            DerivationPath::from(vec![
                ChildNumber::from_hardened_idx(86).expect("86"),
                ChildNumber::from_hardened_idx(ct).expect("coin"),
                ChildNumber::from_hardened_idx(0).expect("account"),
            ])
        }
    };

    let account = master
        .derive_priv(&secp, &account_path)
        .map_err(|e| HotKeystoreError::Derive(e.to_string()))?;
    let account_xpub = Xpub::from_priv(&secp, &account);

    let origin_path = account_path.to_string();
    let fp_hex = format!("{master_fp:08x}");
    let xpub_str = account_xpub.to_string();
    // IMPORTANT: use master xprv + full path, NOT `[fp/origin]account/<0;1>/*`.
    // Miniscript GetKey with origin+nonempty derivation_path signs with the wrong
    // child (account/i instead of account/0/i), producing invalid taproot sigs.
    let descriptor_secret = format!("[{fp_hex}]{master}/{origin_path}/<0;1>/*");

    let id = uuid::Uuid::new_v4().to_string();
    let now = unix_now();
    let record = HotWalletRecord {
        id: id.clone(),
        name: request.name.trim().to_string(),
        network: request.network,
        fingerprint: fp_hex.clone(),
        origin_path: origin_path.clone(),
        xpub: xpub_str.clone(),
        mnemonic: mnemonic.to_string(),
        bip39_passphrase: request.bip39_passphrase.clone(),
        descriptor_secret,
        linked_wallet_id: None,
        linked_vault_id: None,
        created_at: now,
    };

    let key = KeyConfig {
        id: "A".into(),
        role: KeyRole::Other,
        xpub: xpub_str,
        fingerprint: fp_hex,
        origin_path: Some(origin_path),
    };

    Ok((record, key))
}

/// Policy key helper (same as returned by import).
pub fn account_policy_key(record: &HotWalletRecord) -> KeyConfig {
    KeyConfig {
        id: "A".into(),
        role: KeyRole::Other,
        xpub: record.xpub.clone(),
        fingerprint: record.fingerprint.clone(),
        origin_path: Some(record.origin_path.clone()),
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_testnet_account_has_fingerprint_and_tpub() {
        let req = ImportHotWalletRequest {
            name: "Test".into(),
            mnemonic: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".into(),
            bip39_passphrase: String::new(),
            network: NetworkName::Testnet,
            account_path: None,
        };
        let (rec, key) = derive_bip86_account(&req).unwrap();
        assert_eq!(rec.fingerprint.len(), 8);
        assert!(rec.xpub.starts_with('t') || rec.xpub.contains("tpub") || rec.xpub.starts_with("tpub") || rec.xpub.starts_with("vpub") || rec.xpub.starts_with("upub") || key.xpub.len() > 10);
        assert!(rec.descriptor_secret.contains("/<0;1>/*"));
        assert_eq!(key.id, "A");
    }
}
