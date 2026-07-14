//! HWI subprocess client (`enumerate`, `getxpub`, `signtx`).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use bitcoin::bip32::DerivationPath;
use serde::Deserialize;
use serde_json::Value;

use crate::error::SignError;
use crate::types::{DeviceInfo, DeviceType};
use crate::HardwareSigner;

#[derive(Debug, Clone)]
pub struct HwiConfig {
    pub binary: PathBuf,
}

impl Default for HwiConfig {
    fn default() -> Self {
        Self {
            binary: PathBuf::from(std::env::var("HWI_PATH").unwrap_or_else(|_| "hwi".into())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HwiClient {
    config: HwiConfig,
}

impl HwiClient {
    pub fn new(config: HwiConfig) -> Self {
        Self { config }
    }

    pub fn with_binary(path: impl AsRef<Path>) -> Self {
        Self::new(HwiConfig {
            binary: path.as_ref().to_path_buf(),
        })
    }

    pub fn binary_path(&self) -> &Path {
        &self.config.binary
    }

    pub fn enumerate(&self) -> Result<Vec<DeviceInfo>, SignError> {
        let stdout = self.run_raw(&["enumerate"])?;
        parse_enumerate_json(&stdout)
    }

    pub fn get_xpub(&self, fingerprint: &str, path: &DerivationPath) -> Result<String, SignError> {
        let path_str = format!("m/{path}");
        let stdout = self.run_raw(&[
            "--fingerprint",
            fingerprint,
            "getxpub",
            &path_str,
        ])?;
        let obj: HwiXpub =
            serde_json::from_str(stdout.trim()).map_err(|e| SignError::Parse(e.to_string()))?;
        if let Some(err) = obj.error {
            return Err(map_hwi_error(&err));
        }
        obj.xpub
            .ok_or_else(|| SignError::Hwi("getxpub missing xpub field".into()))
    }

    pub fn sign_psbt(&self, fingerprint: &str, psbt_base64: &str) -> Result<String, SignError> {
        let stdout = self.run_raw(&[
            "--fingerprint",
            fingerprint,
            "signtx",
            psbt_base64.trim(),
        ])?;
        let obj: HwiSign =
            serde_json::from_str(stdout.trim()).map_err(|e| SignError::Parse(e.to_string()))?;
        if let Some(err) = obj.error {
            return Err(map_hwi_error(&err));
        }
        obj.psbt
            .ok_or_else(|| SignError::Hwi("signtx missing psbt field".into()))
    }

    fn run_raw(&self, args: &[&str]) -> Result<String, SignError> {
        let output = Command::new(&self.config.binary)
            .args(args)
            .output()
            .map_err(|e| SignError::Binary(format!("{}: {e}", self.config.binary.display())))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            let msg = if !stderr.trim().is_empty() {
                stderr.trim().to_string()
            } else {
                stdout.trim().to_string()
            };
            return Err(map_hwi_error(&msg));
        }

        if stdout.trim().is_empty() {
            return Err(SignError::Hwi("empty HWI stdout".into()));
        }

        // Some HWI errors still exit 0 with {"error":...}
        if let Ok(value) = serde_json::from_str::<Value>(stdout.trim()) {
            if let Some(err) = value.get("error").and_then(|v| v.as_str()) {
                return Err(map_hwi_error(err));
            }
        }

        Ok(stdout)
    }
}

/// Bound signer for one enumerated device.
#[derive(Debug, Clone)]
pub struct HwiDeviceSigner {
    client: HwiClient,
    fingerprint: String,
    id: String,
}

impl HwiDeviceSigner {
    pub fn new(client: HwiClient, fingerprint: impl Into<String>) -> Self {
        let fingerprint = fingerprint.into();
        Self {
            id: fingerprint.clone(),
            client,
            fingerprint,
        }
    }
}

impl HardwareSigner for HwiDeviceSigner {
    fn device_id(&self) -> &str {
        &self.id
    }

    fn fingerprint(&self) -> Result<String, SignError> {
        Ok(self.fingerprint.clone())
    }

    fn get_xpub(&self, path: &DerivationPath) -> Result<String, SignError> {
        self.client.get_xpub(&self.fingerprint, path)
    }

    fn sign_psbt(&self, psbt_base64: &str) -> Result<String, SignError> {
        self.client.sign_psbt(&self.fingerprint, psbt_base64)
    }
}

#[derive(Debug, Deserialize)]
struct HwiEnumerateRow {
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    fingerprint: Option<String>,
    #[serde(default)]
    needs_pin_sent: bool,
    #[serde(default)]
    needs_passphrase_sent: bool,
    #[serde(default)]
    error: Option<String>,
}

impl From<HwiEnumerateRow> for DeviceInfo {
    fn from(row: HwiEnumerateRow) -> Self {
        let fingerprint = row.fingerprint.clone().unwrap_or_default();
        let id = if !fingerprint.is_empty() {
            fingerprint.clone()
        } else {
            row.path.clone().unwrap_or_else(|| row.model.clone())
        };
        DeviceInfo {
            id,
            fingerprint,
            device_type: DeviceType::from_hwi_type(&row.r#type),
            model: if row.model.is_empty() {
                row.r#type.clone()
            } else {
                row.model
            },
            path: row.path,
            needs_pin: row.needs_pin_sent,
            needs_passphrase: row.needs_passphrase_sent,
            error: row.error,
        }
    }
}

#[derive(Debug, Deserialize)]
struct HwiXpub {
    xpub: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HwiSign {
    psbt: Option<String>,
    error: Option<String>,
}

pub fn parse_enumerate_json(raw: &str) -> Result<Vec<DeviceInfo>, SignError> {
    let rows: Vec<HwiEnumerateRow> =
        serde_json::from_str(raw.trim()).map_err(|e| SignError::Parse(e.to_string()))?;
    Ok(rows.into_iter().map(DeviceInfo::from).collect())
}

fn map_hwi_error(msg: &str) -> SignError {
    let lower = msg.to_ascii_lowercase();
    if lower.contains("cancel") || lower.contains("abort") || lower.contains("disconnect") {
        SignError::Cancelled
    } else {
        SignError::Hwi(msg.to_string())
    }
}

/// Parse a user-facing path string (`m/86'/1'/0'` or `86'/1'/0'`) into BIP32.
pub fn parse_derivation_path(raw: &str) -> Result<DerivationPath, SignError> {
    let trimmed = raw.trim().trim_start_matches("m/").trim_start_matches("M/");
    DerivationPath::from_str(trimmed).map_err(|e| SignError::InvalidPath(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_path_strips_m_prefix() {
        let path = parse_derivation_path("m/86'/1'/0'").unwrap();
        assert_eq!(path.to_string(), "86'/1'/0'");
    }

    #[test]
    fn enumerate_json_fixture() {
        let raw = r#"[
          {"type":"ledger","model":"ledger_nano_s","path":"usb","fingerprint":"a1b2c3d4","needs_pin_sent":false,"needs_passphrase_sent":false},
          {"type":"coldcard","model":"coldcard","fingerprint":"deadbeef","error":null}
        ]"#;
        let devices = parse_enumerate_json(raw).unwrap();
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].device_type, DeviceType::Ledger);
        assert_eq!(devices[1].device_type, DeviceType::Coldcard);
        assert_eq!(devices[1].fingerprint, "deadbeef");
    }

    #[test]
    fn maps_cancel_error() {
        assert!(matches!(map_hwi_error("user cancelled"), SignError::Cancelled));
    }

    #[test]
    fn device_type_aliases() {
        assert_eq!(DeviceType::from_hwi_type("bitbox02_btc"), DeviceType::BitBox02);
        assert_eq!(DeviceType::from_hwi_type("trezor_t"), DeviceType::Trezor);
    }
}
