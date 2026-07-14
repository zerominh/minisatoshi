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

/// Spending policy expression and optional timelocked fallback(s).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PolicyExpression {
    pub primary: String,
    /// Legacy single fallback (still accepted when deserializing older vaults).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<FallbackPolicy>,
    /// Timelocked recovery paths (Phase 2). Prefer this over `fallback`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<FallbackPolicy>,
}

impl PolicyExpression {
    /// All fallbacks: legacy `fallback` first (if any), then `fallbacks`.
    pub fn all_fallbacks(&self) -> Vec<&FallbackPolicy> {
        let mut out = Vec::with_capacity(self.fallbacks.len() + 1);
        if let Some(fb) = &self.fallback {
            out.push(fb);
        }
        out.extend(self.fallbacks.iter());
        out
    }
}

/// Timelocked fallback path (e.g. inheritance after N years).
///
/// `allow` is a key expression using the same grammar as `primary`
/// (e.g. `A` or `A && B`).
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
    /// Classic Bitcoin testnet (testnet3). Serialized as `"testnet"`.
    Testnet,
    /// Bitcoin testnet4. Serialized as `"testnet4"`.
    Testnet4,
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
            Self::Testnet4 => bitcoin::Network::Testnet4,
            Self::Signet => bitcoin::Network::Signet,
            Self::Regtest => bitcoin::Network::Regtest,
        }
    }
}
