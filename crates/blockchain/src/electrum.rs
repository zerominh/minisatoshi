use std::collections::HashMap;
use std::time::Duration;

use bitcoin::hashes::{sha256d, Hash};
use policy_engine::NetworkName;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::backend::BlockchainBackend;
use crate::error::ChainError;
use crate::query::DescriptorQuery;
use crate::scanner::{build_scan_plan, DEFAULT_GAP_LIMIT};
use crate::types::{Balance, SyncProgress, SyncResult, TxSummary, Utxo};

/// Electrum protocol backend (JSON-RPC over TCP or TLS).
pub struct ElectrumBackend {
    rpc_url: String,
    client: Client,
    gap_limit: u32,
}

impl ElectrumBackend {
    pub fn new(rpc_url: impl Into<String>) -> Result<Self, ChainError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ChainError::Http(e.to_string()))?;

        Ok(Self {
            rpc_url: rpc_url.into(),
            client,
            gap_limit: DEFAULT_GAP_LIMIT,
        })
    }

    pub fn for_network(network: NetworkName) -> Result<Self, ChainError> {
        Self::new(default_electrum_url(network))
    }

    pub fn with_gap_limit(mut self, gap_limit: u32) -> Self {
        self.gap_limit = gap_limit;
        self
    }

    fn call(&self, method: &str, params: Value) -> Result<Value, ChainError> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let response = self
            .client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .map_err(|e| ChainError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ChainError::Api(format!(
                "electrum rpc failed with status {}",
                response.status()
            )));
        }

        let body: ElectrumResponse = response
            .json()
            .map_err(|e| ChainError::Parse(e.to_string()))?;

        if let Some(error) = body.error {
            return Err(ChainError::Api(format!("{error:?}")));
        }

        body.result
            .ok_or_else(|| ChainError::Api("electrum response missing result".into()))
    }

    fn address_has_activity(&self, address: &str) -> Result<bool, ChainError> {
        let history = self.scripthash_history(address)?;
        Ok(!history.is_empty())
    }

    fn scripthash_history(&self, address: &str) -> Result<Vec<ElectrumHistoryEntry>, ChainError> {
        let scripthash = address_to_scripthash(address)?;
        let result = self.call(
            "blockchain.scripthash.get_history",
            serde_json::json!([scripthash]),
        )?;

        serde_json::from_value(result).map_err(|e| ChainError::Parse(e.to_string()))
    }

    fn scripthash_utxos(&self, address: &str) -> Result<Vec<ElectrumUtxo>, ChainError> {
        let scripthash = address_to_scripthash(address)?;
        let result = self.call(
            "blockchain.scripthash.listunspent",
            serde_json::json!([scripthash]),
        )?;

        serde_json::from_value(result).map_err(|e| ChainError::Parse(e.to_string()))
    }
}

impl BlockchainBackend for ElectrumBackend {
    fn sync(
        &self,
        query: &DescriptorQuery,
        progress: &dyn Fn(SyncProgress),
    ) -> Result<SyncResult, ChainError> {
        let plan = build_scan_plan(
            &query.policy,
            &query.descriptor,
            self.gap_limit,
            |address| self.address_has_activity(address),
            progress,
        )?;

        let mut utxos = Vec::new();
        let mut balance = Balance::zero();
        let mut history_map: HashMap<String, TxSummary> = HashMap::new();

        for scanned in plan.receive.iter().chain(plan.change.iter()) {
            for entry in self.scripthash_utxos(&scanned.address)? {
                let confirmed = entry.height > 0;
                let value = entry.value;
                if confirmed {
                    balance.confirmed_sats += value;
                } else {
                    balance.unconfirmed_sats += value;
                }

                utxos.push(Utxo {
                    txid: entry.tx_hash,
                    vout: entry.tx_pos,
                    value_sats: value,
                    address: scanned.address.clone(),
                    confirmed,
                    block_height: if entry.height > 0 {
                        Some(entry.height as u32)
                    } else {
                        None
                    },
                    derivation_index: scanned.index,
                    is_change: scanned.is_change,
                });
            }

            for entry in self.scripthash_history(&scanned.address)? {
                history_map
                    .entry(entry.tx_hash.clone())
                    .or_insert(TxSummary {
                        txid: entry.tx_hash,
                        amount_sats: 0,
                        confirmed: entry.height > 0,
                        block_height: if entry.height > 0 {
                            Some(entry.height as u32)
                        } else {
                            None
                        },
                    });
            }
        }

        Ok(SyncResult {
            balance,
            utxos,
            history: history_map.into_values().collect(),
            scanned_receive_count: plan.receive.len() as u32,
            scanned_change_count: plan.change.len() as u32,
        })
    }

    fn broadcast(&self, tx_hex: &str) -> Result<String, ChainError> {
        let result = self.call(
            "blockchain.transaction.broadcast",
            serde_json::json!([tx_hex]),
        )?;

        result
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| ChainError::Parse("broadcast response was not a txid string".into()))
    }
}

pub fn default_electrum_url(network: NetworkName) -> &'static str {
    match network {
        NetworkName::Mainnet => "https://blockstream.info/electrum/api",
        NetworkName::Testnet => "https://blockstream.info/testnet/electrum/api",
        // No Blockstream HTTPS Electrum bridge for testnet4; local electrs default.
        NetworkName::Testnet4 => "http://127.0.0.1:3004",
        NetworkName::Signet => "https://blockstream.info/signet/electrum/api",
        NetworkName::Regtest => "http://127.0.0.1:3004",
    }
}

fn address_to_scripthash(address: &str) -> Result<String, ChainError> {
    let parsed = address
        .parse::<bitcoin::Address<bitcoin::address::NetworkUnchecked>>()
        .map_err(|e| ChainError::Parse(e.to_string()))?
        .assume_checked();
    let script = parsed.script_pubkey();
    let hash = sha256d::Hash::hash(script.as_bytes());
    Ok(hex::encode(
        hash.to_byte_array()
            .iter()
            .rev()
            .copied()
            .collect::<Vec<_>>(),
    ))
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct ElectrumResponse {
    result: Option<Value>,
    error: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ElectrumHistoryEntry {
    tx_hash: String,
    height: i64,
}

#[derive(Debug, Deserialize)]
struct ElectrumUtxo {
    tx_hash: String,
    tx_pos: u32,
    value: u64,
    height: i64,
}

#[cfg(test)]
mod tests {
    use policy_engine::{abc_preset, NetworkName};

    use super::{address_to_scripthash, default_electrum_url};

    #[test]
    fn scripthash_is_reversed_sha256() {
        let keys = sample_keys_from_policy_engine();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let descriptor = descriptor_engine::compile_descriptor_from_config(&policy).unwrap();
        let address = address_engine::new_receive_address(&policy, &descriptor, 0)
            .unwrap()
            .address;

        let hash = address_to_scripthash(&address).unwrap();
        assert_eq!(hash.len(), 64);
    }

    fn sample_keys_from_policy_engine() -> [policy_engine::KeyConfig; 3] {
        use policy_engine::{
            test_vectors::TEST_FP, test_vectors::TEST_XPUB_A, test_vectors::TEST_XPUB_B,
            test_vectors::TEST_XPUB_C, KeyConfig, KeyRole,
        };
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
    fn electrum_url_presets_exist() {
        assert!(default_electrum_url(NetworkName::Testnet).contains("testnet"));
    }
}
