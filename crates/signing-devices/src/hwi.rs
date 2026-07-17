//! HWI subprocess client (`enumerate`, `getxpub`, `signtx`).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
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
    /// HWI `--chain` value: `main`, `test`, `testnet4`, `signet`, `regtest`.
    pub chain: Option<String>,
}

impl Default for HwiConfig {
    fn default() -> Self {
        Self {
            binary: PathBuf::from(std::env::var("HWI_PATH").unwrap_or_else(|_| "hwi".into())),
            chain: None,
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
            chain: None,
        })
    }

    pub fn with_chain(mut self, chain: impl Into<String>) -> Self {
        self.config.chain = Some(chain.into());
        self
    }

    pub fn binary_path(&self) -> &Path {
        &self.config.binary
    }

    pub fn enumerate(&self) -> Result<Vec<DeviceInfo>, SignError> {
        let stdout = self.run_raw(&["enumerate"])?;
        parse_enumerate_json(&stdout)
    }

    /// Match a user fingerprint against `enumerate` results (normalized lowercase hex).
    pub fn resolve_fingerprint(&self, fingerprint: &str) -> Result<String, SignError> {
        let want = fingerprint.trim().to_ascii_lowercase();
        if want.len() != 8 || !want.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(SignError::DeviceNotFound(format!(
                "invalid fingerprint “{fingerprint}” — use 8 hex characters from Settings → Signing devices → Scan"
            )));
        }
        let devices = self.enumerate()?;
        if let Some(device) = devices
            .iter()
            .find(|d| d.fingerprint.trim().to_ascii_lowercase() == want)
        {
            return Ok(device.fingerprint.trim().to_ascii_lowercase());
        }
        let found: Vec<String> = devices
            .iter()
            .filter(|d| !d.fingerprint.trim().is_empty())
            .map(|d| {
                let label = if d.model.is_empty() {
                    d.device_type.as_str().to_string()
                } else {
                    d.model.clone()
                };
                format!("{label} (fp {})", d.fingerprint.trim().to_ascii_lowercase())
            })
            .collect();
        if found.is_empty() {
            Err(SignError::DeviceNotFound(
                "no hardware wallet detected — unlock device, open the Bitcoin app, then scan in Settings → Signing devices"
                    .into(),
            ))
        } else {
            Err(SignError::DeviceNotFound(format!(
                "no device with fingerprint {want}; connected: {} — pick the correct fingerprint from Scan",
                found.join("; ")
            )))
        }
    }

    /// Whether this HWI binary exposes `registerpolicy` on the CLI (not stock 3.2.0).
    pub fn cli_supports_registerpolicy(&self) -> bool {
        Command::new(&self.config.binary)
            .arg("--help")
            .output()
            .map(|o| {
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                );
                text.contains("registerpolicy")
            })
            .unwrap_or(false)
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
        let psbt = normalize_psbt_base64(psbt_base64);
        // HWI `--stdin` reads one CLI arg per line; wrapped PSBT base64 must be a single line.
        // Put fingerprint + subcommand on argv; pipe only the PSBT (see HWI examples).
        let stdout = self.run_signtx_stdin(fingerprint, &psbt)?;
        let obj: HwiSign =
            serde_json::from_str(stdout.trim()).map_err(|e| SignError::Parse(e.to_string()))?;
        if let Some(err) = obj.error {
            return Err(map_hwi_error(&err));
        }
        obj.psbt
            .ok_or_else(|| SignError::Hwi("signtx missing psbt field".into()))
    }

    /// Try HWI `registerpolicy` (available on some builds; not stock 3.2.0).
    /// Returns HMAC / proof string when the device provides one.
    pub fn register_policy(
        &self,
        fingerprint: &str,
        name: &str,
        policy: &str,
        keys_json: &str,
    ) -> Result<Option<String>, SignError> {
        let stdout = self.run_raw(&[
            "--fingerprint",
            fingerprint,
            "registerpolicy",
            "--name",
            name,
            "--policy",
            policy,
            "--keys",
            keys_json,
        ])?;
        let value: Value =
            serde_json::from_str(stdout.trim()).map_err(|e| SignError::Parse(e.to_string()))?;
        if let Some(err) = value.get("error").and_then(|v| v.as_str()) {
            let lower = err.to_ascii_lowercase();
            if lower.contains("invalid choice")
                || lower.contains("unknown")
                || lower.contains("unrecognized")
            {
                return Err(SignError::Unsupported(
                    "this HWI build has no registerpolicy — export BIP-388 / Coldcard files instead"
                        .into(),
                ));
            }
            return Err(map_hwi_error(err));
        }
        let hmac = value
            .get("hmac")
            .or_else(|| value.get("hmac_b64"))
            .or_else(|| value.get("registration"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        Ok(hmac)
    }

    /// Whether HWI lacks `registerpolicy` (stock 3.2.0) or rejected that subcommand.
    pub fn is_registerpolicy_unavailable(err: &SignError) -> bool {
        match err {
            SignError::Unsupported(msg) => msg.to_ascii_lowercase().contains("registerpolicy"),
            SignError::Hwi(msg) => {
                let lower = msg.to_ascii_lowercase();
                lower.contains("registerpolicy") || lower.contains("invalid choice")
            }
            _ => false,
        }
    }

    /// Display an address for `--desc` (device confirmation / soft register path).
    pub fn display_address_desc(
        &self,
        fingerprint: &str,
        descriptor: &str,
    ) -> Result<String, SignError> {
        let stdout = self.run_raw(&[
            "--fingerprint",
            fingerprint,
            "displayaddress",
            "--desc",
            descriptor.trim(),
        ])?;
        let value: Value =
            serde_json::from_str(stdout.trim()).map_err(|e| SignError::Parse(e.to_string()))?;
        if let Some(err) = value.get("error").and_then(|v| v.as_str()) {
            return Err(map_hwi_error(err));
        }
        value
            .get("address")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| SignError::Hwi("displayaddress missing address".into()))
    }

    fn run_signtx_stdin(&self, fingerprint: &str, psbt_base64: &str) -> Result<String, SignError> {
        let mut cmd = Command::new(&self.config.binary);
        if let Some(chain) = &self.config.chain {
            cmd.arg("--chain").arg(chain);
        }
        cmd.arg("--fingerprint").arg(fingerprint.trim());
        cmd.arg("--stdin").arg("signtx");
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| SignError::Binary(format!("{}: {e}", self.config.binary.display())))?;

        if let Some(mut stdin) = child.stdin.take() {
            let payload = build_signtx_stdin_payload(psbt_base64);
            stdin
                .write_all(payload.as_bytes())
                .map_err(|e| SignError::Binary(format!("hwi stdin write failed: {e}")))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| SignError::Binary(format!("{}: {e}", self.config.binary.display())))?;
        self.parse_output(output)
    }

    fn run_raw(&self, args: &[&str]) -> Result<String, SignError> {
        let mut cmd = Command::new(&self.config.binary);
        if let Some(chain) = &self.config.chain {
            cmd.arg("--chain").arg(chain);
        }
        let output = cmd
            .args(args)
            .output()
            .map_err(|e| SignError::Binary(format!("{}: {e}", self.config.binary.display())))?;
        self.parse_output(output)
    }

    fn parse_output(&self, output: Output) -> Result<String, SignError> {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr);

        if let Some(err) = hwi_json_error(stdout.trim()) {
            return Err(map_hwi_error(&err));
        }

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

    fn register_policy(&self, descriptor: &str) -> Result<(), SignError> {
        // Soft path: ask device to display address derived from desc (confirms recognition).
        let _addr = self
            .client
            .display_address_desc(&self.fingerprint, descriptor)?;
        Ok(())
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

fn normalize_psbt_base64(psbt_base64: &str) -> String {
    psbt_base64.split_whitespace().collect()
}

fn build_signtx_stdin_payload(psbt_base64: &str) -> String {
    format!("{}\n\n", normalize_psbt_base64(psbt_base64))
}

fn hwi_json_error(stdout: &str) -> Option<String> {
    let value: Value = serde_json::from_str(stdout).ok()?;
    value
        .get("error")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn map_hwi_error(msg: &str) -> SignError {
    let lower = msg.to_ascii_lowercase();
    if lower.contains("cancel") || lower.contains("abort") || lower.contains("disconnect") {
        SignError::Cancelled
    } else if lower.contains("usage:") || lower.contains("unrecognized arguments") {
        SignError::Hwi(
            "HWI rejected the command (invalid arguments). If the PSBT was pasted from a file, \
             try again from a freshly built transaction. Otherwise: Settings -> Verify HWI, \
             reconnect/unlock the device, and try signing again."
                .into(),
        )
    } else {
        SignError::Hwi(msg.to_string())
    }
}

/// Whether HWI lacks `registerpolicy` (stock 3.2.0) or rejected that subcommand.
pub fn is_registerpolicy_unavailable(err: &SignError) -> bool {
    HwiClient::is_registerpolicy_unavailable(err)
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
    fn detects_registerpolicy_unavailable() {
        let err = SignError::Hwi(
            "hwi.exe: error: argument command: invalid choice: 'registerpolicy'".into(),
        );
        assert!(is_registerpolicy_unavailable(&err));
        let unsupported = SignError::Unsupported(
            "this HWI build has no registerpolicy — export BIP-388 / Coldcard files instead"
                .into(),
        );
        assert!(is_registerpolicy_unavailable(&unsupported));
    }

    #[test]
    fn signtx_stdin_payload_format() {
        let payload = build_signtx_stdin_payload("cHNidP8B");
        assert_eq!(payload, "cHNidP8B\n\n");
    }

    #[test]
    fn normalize_psbt_base64_collapses_wrapped_lines() {
        let wrapped = "cHNidP8B\ncHNidP8C\n";
        assert_eq!(normalize_psbt_base64(wrapped), "cHNidP8BcHNidP8C");
    }

    #[test]
    fn hwi_json_error_parsed_from_stdout() {
        let raw = r#"{"error": "Could not find device", "code": -3}"#;
        assert_eq!(
            hwi_json_error(raw).as_deref(),
            Some("Could not find device")
        );
    }

    /// Run with `HWI_INTEGRATION=1` and HWI on PATH or in `%TEMP%\hwi-test\hwi.exe`.
    #[test]
    fn hwi_signtx_wrapped_psbt_integration() {
        if std::env::var("HWI_INTEGRATION").ok().as_deref() != Some("1") {
            return;
        }
        let hwi_path = std::env::var("HWI_PATH").unwrap_or_else(|_| {
            format!(
                "{}\\hwi-test\\hwi.exe",
                std::env::var("TEMP").or(std::env::var("TMP")).unwrap()
            )
        });
        if !std::path::Path::new(&hwi_path).exists() {
            eprintln!("skip: HWI not found at {hwi_path}");
            return;
        }
        let client = HwiClient::with_binary(&hwi_path).with_chain("test");
        let wrapped = "cHNidP8B\ncHNidP8C\n";
        let err = client
            .sign_psbt("a1b2c3d4", wrapped)
            .expect_err("expected device-not-found, not invalid-args");
        let msg = err.to_string().to_ascii_lowercase();
        assert!(
            !msg.contains("usage:") && !msg.contains("invalid arguments"),
            "wrapped PSBT should not break HWI CLI: {err}"
        );
        assert!(msg.contains("could not find device"), "got: {err}");
    }

    #[test]
    fn device_type_aliases() {
        assert_eq!(DeviceType::from_hwi_type("bitbox02_btc"), DeviceType::BitBox02);
        assert_eq!(DeviceType::from_hwi_type("trezor_t"), DeviceType::Trezor);
    }
}
