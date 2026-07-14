use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Ledger,
    Trezor,
    Coldcard,
    BitBox02,
    Jade,
    KeepKey,
    Unknown,
}

impl DeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ledger => "ledger",
            Self::Trezor => "trezor",
            Self::Coldcard => "coldcard",
            Self::BitBox02 => "bitbox02",
            Self::Jade => "jade",
            Self::KeepKey => "keepkey",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_hwi_type(raw: &str) -> Self {
        let lower = raw.to_ascii_lowercase();
        if lower.contains("ledger") {
            Self::Ledger
        } else if lower.contains("trezor") {
            Self::Trezor
        } else if lower.contains("coldcard") {
            Self::Coldcard
        } else if lower.contains("bitbox") {
            Self::BitBox02
        } else if lower.contains("jade") {
            Self::Jade
        } else if lower.contains("keepkey") {
            Self::KeepKey
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub fingerprint: String,
    pub device_type: DeviceType,
    pub model: String,
    pub path: Option<String>,
    pub needs_pin: bool,
    pub needs_passphrase: bool,
    pub error: Option<String>,
}
