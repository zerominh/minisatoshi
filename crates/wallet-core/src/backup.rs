//! Wallet backup package (`minisatoshi-wallet-v1.json`).

use policy_engine::{NetworkName, PolicyConfig, ScriptTypeName};
use serde::{Deserialize, Serialize};

/// Current on-disk / export format id.
pub const WALLET_BACKUP_FORMAT: &str = "minisatoshi-wallet-v1";

/// Legacy on-disk / export format id, still accepted when importing.
pub const LEGACY_VAULT_BACKUP_FORMAT: &str = "minisatoshi-vault-v1";

/// Portable wallet backup — descriptor is source of truth; policy optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletBackup {
    pub format_version: String,
    pub name: String,
    pub network: NetworkName,
    pub descriptor: String,
    pub script_type: ScriptTypeName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<PolicyConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<serde_json::Map<String, serde_json::Value>>,
    pub created_at: i64,
}

impl WalletBackup {
    pub fn new(
        name: impl Into<String>,
        network: NetworkName,
        descriptor: impl Into<String>,
        script_type: ScriptTypeName,
        policy: Option<PolicyConfig>,
        created_at: i64,
    ) -> Self {
        Self {
            format_version: WALLET_BACKUP_FORMAT.to_string(),
            name: name.into(),
            network,
            descriptor: descriptor.into(),
            script_type,
            policy,
            labels: None,
            created_at,
        }
    }

    /// True if `format_version` is a format this build can import (current or legacy).
    pub fn is_supported_format(&self) -> bool {
        self.format_version == WALLET_BACKUP_FORMAT
            || self.format_version == LEGACY_VAULT_BACKUP_FORMAT
    }

    /// Serialize pretty JSON for `.json` export.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parse from JSON text (backup file or paste).
    pub fn from_json(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw.trim())
    }

    /// Plain descriptor-only export line(s).
    pub fn descriptor_txt(&self) -> String {
        format!("{}\n", self.descriptor.trim())
    }
}
