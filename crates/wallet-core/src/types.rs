use policy_engine::{NetworkName, PolicyConfig, ScriptTypeName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wallet {
    pub id: String,
    pub name: String,
    pub network: NetworkName,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletSummary {
    pub id: String,
    pub name: String,
    pub network: NetworkName,
    pub vault_count: usize,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vault {
    pub id: String,
    pub wallet_id: String,
    pub name: String,
    pub policy: PolicyConfig,
    pub descriptor: String,
    pub script_type: ScriptTypeName,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultSummary {
    pub id: String,
    pub wallet_id: String,
    pub name: String,
    pub script_type: ScriptTypeName,
    pub created_at: i64,
}

pub(crate) fn network_from_str(value: &str) -> Result<NetworkName, String> {
    match value {
        "mainnet" => Ok(NetworkName::Mainnet),
        "testnet" => Ok(NetworkName::Testnet),
        "signet" => Ok(NetworkName::Signet),
        "regtest" => Ok(NetworkName::Regtest),
        other => Err(other.to_string()),
    }
}

pub(crate) fn network_to_str(network: NetworkName) -> &'static str {
    match network {
        NetworkName::Mainnet => "mainnet",
        NetworkName::Testnet => "testnet",
        NetworkName::Signet => "signet",
        NetworkName::Regtest => "regtest",
    }
}

pub(crate) fn script_type_from_str(value: &str) -> Result<ScriptTypeName, String> {
    match value {
        "taproot" => Ok(ScriptTypeName::Taproot),
        "wsh" => Ok(ScriptTypeName::Wsh),
        other => Err(other.to_string()),
    }
}

pub(crate) fn script_type_to_str(script_type: ScriptTypeName) -> &'static str {
    match script_type {
        ScriptTypeName::Taproot => "taproot",
        ScriptTypeName::Wsh => "wsh",
    }
}
