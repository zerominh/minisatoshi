use std::collections::HashMap;
use std::time::Duration;

use policy_engine::NetworkName;
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::backend::BlockchainBackend;
use crate::error::ChainError;
use crate::query::DescriptorQuery;
use crate::scanner::{build_scan_plan, DEFAULT_GAP_LIMIT};
use crate::types::{Balance, SyncProgress, SyncResult, TxSummary, Utxo};

/// Esplora HTTP API backend (Blockstream, mempool.space, self-hosted Electrs).
pub struct EsploraBackend {
    base_url: String,
    client: Client,
    gap_limit: u32,
}

impl EsploraBackend {
    pub fn new(base_url: impl Into<String>) -> Result<Self, ChainError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ChainError::Http(e.to_string()))?;

        Ok(Self {
            base_url: normalize_base_url(base_url.into()),
            client,
            gap_limit: DEFAULT_GAP_LIMIT,
        })
    }

    pub fn for_network(network: NetworkName) -> Result<Self, ChainError> {
        Self::new(default_esplora_url(network))
    }

    pub fn with_gap_limit(mut self, gap_limit: u32) -> Self {
        self.gap_limit = gap_limit;
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn address_stats(&self, address: &str) -> Result<EsploraAddressStats, ChainError> {
        let response = self
            .client
            .get(self.url(&format!("/address/{address}")))
            .send()
            .map_err(|e| ChainError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ChainError::Api(format!(
                "address lookup failed with status {}",
                response.status()
            )));
        }

        response
            .json::<EsploraAddressStats>()
            .map_err(|e| ChainError::Parse(e.to_string()))
    }

    fn address_has_activity(&self, address: &str) -> Result<bool, ChainError> {
        let stats = self.address_stats(address)?;
        Ok(stats.chain_stats.tx_count > 0 || stats.mempool_stats.tx_count > 0)
    }

    fn address_utxos(&self, address: &str) -> Result<Vec<EsploraUtxo>, ChainError> {
        let response = self
            .client
            .get(self.url(&format!("/address/{address}/utxo")))
            .send()
            .map_err(|e| ChainError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ChainError::Api(format!(
                "utxo lookup failed with status {}",
                response.status()
            )));
        }

        response
            .json::<Vec<EsploraUtxo>>()
            .map_err(|e| ChainError::Parse(e.to_string()))
    }

    fn address_txs(&self, address: &str) -> Result<Vec<EsploraTx>, ChainError> {
        let response = self
            .client
            .get(self.url(&format!("/address/{address}/txs")))
            .send()
            .map_err(|e| ChainError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ChainError::Api(format!(
                "tx history failed with status {}",
                response.status()
            )));
        }

        response
            .json::<Vec<EsploraTx>>()
            .map_err(|e| ChainError::Parse(e.to_string()))
    }
}

impl BlockchainBackend for EsploraBackend {
    fn sync(
        &self,
        query: &DescriptorQuery,
        progress: &dyn Fn(SyncProgress),
    ) -> Result<SyncResult, ChainError> {
        let policy = &query.policy;
        let descriptor = &query.descriptor;

        let plan = build_scan_plan(
            policy,
            descriptor,
            self.gap_limit,
            |address| self.address_has_activity(address),
            progress,
        )?;

        let mut utxos = Vec::new();
        let mut balance = Balance::zero();
        let mut history_map: HashMap<String, TxSummary> = HashMap::new();

        for scanned in plan.receive.iter().chain(plan.change.iter()) {
            for utxo in self.address_utxos(&scanned.address)? {
                let confirmed = utxo.status.confirmed;
                let value = utxo.value;
                if confirmed {
                    balance.confirmed_sats += value;
                } else {
                    balance.unconfirmed_sats += value;
                }

                utxos.push(Utxo {
                    txid: utxo.txid,
                    vout: utxo.vout,
                    value_sats: value,
                    address: scanned.address.clone(),
                    confirmed,
                    block_height: utxo.status.block_height,
                    derivation_index: scanned.index,
                    is_change: scanned.is_change,
                });
            }

            for tx in self.address_txs(&scanned.address)? {
                let net = net_amount_for_address(&tx, &scanned.address);
                history_map
                    .entry(tx.txid.clone())
                    .and_modify(|existing| existing.amount_sats += net)
                    .or_insert(TxSummary {
                        txid: tx.txid.clone(),
                        amount_sats: net,
                        confirmed: tx.status.confirmed,
                        block_height: tx.status.block_height,
                    });
            }
        }

        let history = history_map.into_values().collect();

        Ok(SyncResult {
            balance,
            utxos,
            history,
            scanned_receive_count: plan.receive.len() as u32,
            scanned_change_count: plan.change.len() as u32,
        })
    }

    fn broadcast(&self, tx_hex: &str) -> Result<String, ChainError> {
        let response = self
            .client
            .post(self.url("/tx"))
            .body(tx_hex.to_string())
            .send()
            .map_err(|e| ChainError::Http(e.to_string()))?;

        if !response.status().is_success() {
            let body = response.text().unwrap_or_default();
            return Err(ChainError::Broadcast(body));
        }

        response
            .text()
            .map_err(|e| ChainError::Parse(e.to_string()))
    }
}

pub fn default_esplora_url(network: NetworkName) -> &'static str {
    match network {
        NetworkName::Mainnet => "https://blockstream.info/api",
        NetworkName::Testnet => "https://blockstream.info/testnet/api",
        // Blockstream does not host a public testnet4 Esplora yet.
        NetworkName::Testnet4 => "https://mempool.space/testnet4/api",
        NetworkName::Signet => "https://blockstream.info/signet/api",
        NetworkName::Regtest => "http://127.0.0.1:3002",
    }
}

fn normalize_base_url(url: String) -> String {
    url.trim_end_matches('/').to_string()
}

fn net_amount_for_address(tx: &EsploraTx, address: &str) -> i64 {
    let mut net = 0_i64;
    for vout in &tx.vout {
        if vout.scriptpubkey_address.as_deref() == Some(address) {
            net += vout.value as i64;
        }
    }
    for vin in &tx.vin {
        if vin.prevout.scriptpubkey_address.as_deref() == Some(address) {
            net -= vin.prevout.value as i64;
        }
    }
    net
}

#[derive(Debug, Deserialize)]
struct EsploraAddressStats {
    chain_stats: EsploraTxStats,
    mempool_stats: EsploraTxStats,
}

#[derive(Debug, Deserialize)]
struct EsploraTxStats {
    tx_count: u64,
}

#[derive(Debug, Deserialize)]
struct EsploraUtxo {
    txid: String,
    vout: u32,
    value: u64,
    status: EsploraStatus,
}

#[derive(Debug, Deserialize)]
struct EsploraTx {
    txid: String,
    status: EsploraStatus,
    vin: Vec<EsploraVin>,
    vout: Vec<EsploraVout>,
}

#[derive(Debug, Deserialize)]
struct EsploraVin {
    prevout: EsploraPrevout,
}

#[derive(Debug, Deserialize)]
struct EsploraPrevout {
    value: u64,
    scriptpubkey_address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EsploraVout {
    value: u64,
    scriptpubkey_address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EsploraStatus {
    confirmed: bool,
    block_height: Option<u32>,
}

#[cfg(test)]
mod tests {
    use descriptor_engine::compile_descriptor_from_config;
    use httpmock::prelude::*;
    use policy_engine::{
        abc_preset, test_vectors::TEST_FP, test_vectors::TEST_XPUB_A, test_vectors::TEST_XPUB_B,
        test_vectors::TEST_XPUB_C, KeyConfig, KeyRole, NetworkName,
    };

    use super::*;
    use crate::query::DescriptorQuery;

    fn sample_policy() -> (policy_engine::PolicyConfig, String) {
        let keys = [
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
        ];
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let descriptor = compile_descriptor_from_config(&policy).unwrap();
        (policy, descriptor)
    }

    #[test]
    fn esplora_sync_aggregates_balance_and_history() {
        let server = MockServer::start();
        let (policy, descriptor) = sample_policy();
        let address = address_engine::new_receive_address(&policy, &descriptor, 0)
            .unwrap()
            .address;

        server.mock(|when, then| {
            when.method(GET).path(format!("/address/{address}"));
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"chain_stats":{"tx_count":1},"mempool_stats":{"tx_count":0}}"#);
        });

        server.mock(|when, then| {
            when.method(GET)
                .path(format!("/address/{address}/utxo"));
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"[{"txid":"abc","vout":0,"value":150000,"status":{"confirmed":true,"block_height":100}}]"#);
        });

        server.mock(|when, then| {
            when.method(GET)
                .path(format!("/address/{address}/txs"));
            then.status(200)
                .header("content-type", "application/json")
                .body(format!(
                    r#"[{{"txid":"abc","status":{{"confirmed":true,"block_height":100}},"vin":[],"vout":[{{"value":150000,"scriptpubkey_address":"{address}"}}]}}]"#
                ));
        });

        server.mock(|when, then| {
            when.method(GET).path_matches(r"^/address/[^/]+$");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"chain_stats":{"tx_count":0},"mempool_stats":{"tx_count":0}}"#);
        });

        server.mock(|when, then| {
            when.method(GET).path_matches(r"^/address/[^/]+/utxo$");
            then.status(200)
                .header("content-type", "application/json")
                .body("[]");
        });

        server.mock(|when, then| {
            when.method(GET).path_matches(r"^/address/[^/]+/txs$");
            then.status(200)
                .header("content-type", "application/json")
                .body("[]");
        });

        let backend = EsploraBackend::new(server.base_url())
            .unwrap()
            .with_gap_limit(3);
        let query = DescriptorQuery::new(policy, descriptor);
        let result = backend.sync(&query, &|_| {}).unwrap();

        assert_eq!(result.balance.confirmed_sats, 150_000);
        assert_eq!(result.utxos.len(), 1);
        assert_eq!(result.history.len(), 1);
        assert_eq!(result.history[0].amount_sats, 150_000);
    }
}
