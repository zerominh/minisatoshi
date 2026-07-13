use serde::{Deserialize, Serialize};

/// Supported policy schema version.
pub const POLICY_SCHEMA_VERSION: u32 = 1;

/// Full policy configuration submitted by the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub version: u32,
    pub network: NetworkName,
    pub script_type: ScriptTypeName,
    pub keys: Vec<KeyConfig>,
    pub policy: PolicyExpression,
}

/// Spending policy expression and optional timelocked fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyExpression {
    pub primary: String,
    pub fallback: Option<FallbackPolicy>,
}

/// Timelocked fallback path (e.g. inheritance after N years).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FallbackPolicy {
    pub after: String,
    pub allow: String,
}

/// Key participant in a vault policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyConfig {
    pub id: String,
    pub role: KeyRole,
    pub xpub: String,
    pub fingerprint: String,
    /// Optional BIP32 origin path, e.g. `84'/0'/0'`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyRole {
    Investor,
    Manager,
    Recovery,
    Cosigner,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkName {
    Mainnet,
    Testnet,
    Signet,
    Regtest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScriptTypeName {
    Taproot,
    Wsh,
}

impl NetworkName {
    pub fn to_bitcoin_network(self) -> bitcoin::Network {
        match self {
            Self::Mainnet => bitcoin::Network::Bitcoin,
            Self::Testnet => bitcoin::Network::Testnet,
            Self::Signet => bitcoin::Network::Signet,
            Self::Regtest => bitcoin::Network::Regtest,
        }
    }
}
