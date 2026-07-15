use policy_engine::{NetworkName, PolicyConfig, ScriptTypeName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub network: NetworkName,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub network: NetworkName,
    pub wallet_count: usize,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wallet {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub policy: PolicyConfig,
    pub descriptor: String,
    pub script_type: ScriptTypeName,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletSummary {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub script_type: ScriptTypeName,
    pub created_at: i64,
}

pub(crate) fn network_from_str(value: &str) -> Result<NetworkName, String> {
    match value {
        "mainnet" => Ok(NetworkName::Mainnet),
        // "testnet" and "testnet3" both mean classic testnet3.
        "testnet" | "testnet3" => Ok(NetworkName::Testnet),
        "testnet4" => Ok(NetworkName::Testnet4),
        "signet" => Ok(NetworkName::Signet),
        "regtest" => Ok(NetworkName::Regtest),
        other => Err(other.to_string()),
    }
}

pub(crate) fn network_to_str(network: NetworkName) -> &'static str {
    match network {
        NetworkName::Mainnet => "mainnet",
        NetworkName::Testnet => "testnet",
        NetworkName::Testnet4 => "testnet4",
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

#[cfg(test)]
mod tests {
    use super::*;
    use policy_engine::NetworkName;

    #[test]
    fn network_from_str_accepts_testnet3_alias() {
        assert_eq!(network_from_str("testnet").unwrap(), NetworkName::Testnet);
        assert_eq!(network_from_str("testnet3").unwrap(), NetworkName::Testnet);
        assert_eq!(network_from_str("testnet4").unwrap(), NetworkName::Testnet4);
    }

    #[test]
    fn network_to_str_roundtrips_known_networks() {
        assert_eq!(network_to_str(NetworkName::Testnet), "testnet");
        assert_eq!(network_to_str(NetworkName::Testnet4), "testnet4");
        assert_eq!(
            network_from_str(network_to_str(NetworkName::Signet)).unwrap(),
            NetworkName::Signet
        );
    }

    #[test]
    fn network_from_str_rejects_unknown() {
        assert!(network_from_str("nonsense").is_err());
    }
}
