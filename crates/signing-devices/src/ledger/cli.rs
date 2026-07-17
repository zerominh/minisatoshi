//! Subprocess bridge to `tools/ledger_cli.py` (ledger-bitcoin).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use policy_engine::NetworkName;
use serde::Deserialize;
use serde_json::Value;

use crate::error::SignError;
use crate::ledger::errors::map_ledger_cli_error;
use crate::ledger::install::ensure_ledger_runtime;
use crate::registration::Bip388Policy;

/// Device prompts (register / sign) can take several minutes.
const LEDGER_CLI_TIMEOUT: Duration = Duration::from_secs(180);
/// Probe only needs a quick GET_VERSION; HID can hang if Ledger Live holds the device.
const LEDGER_PROBE_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct LedgerCliConfig {
    pub python: PathBuf,
    pub python_prefix: Vec<String>,
    pub script: PathBuf,
    pub chain: String,
}

#[derive(Debug, Deserialize)]
struct CliResponse {
    ok: bool,
    error: Option<String>,
    hmac: Option<String>,
    psbt: Option<String>,
    #[serde(rename = "appName")]
    app_name: Option<String>,
    #[serde(rename = "appVersion")]
    app_version: Option<String>,
}

/// `ledger-bitcoin` `--chain` value for a Minisatoshi network.
pub fn ledger_chain(network: NetworkName) -> &'static str {
    match network {
        NetworkName::Mainnet => "main",
        NetworkName::Testnet | NetworkName::Testnet4 => "test",
        NetworkName::Signet => "signet",
        NetworkName::Regtest => "regtest",
    }
}

pub fn resolve_ledger_cli(
    data_dir: &std::path::Path,
    network: NetworkName,
) -> Result<LedgerCliConfig, SignError> {
    let runtime = ensure_ledger_runtime(data_dir)?;
    Ok(LedgerCliConfig {
        python: runtime.python,
        python_prefix: runtime.python_prefix,
        script: runtime.script,
        chain: ledger_chain(network).to_string(),
    })
}

fn policy_payload(bip388: &Bip388Policy) -> Value {
    serde_json::json!({
        "name": bip388.name,
        "policy": bip388.policy,
        "keys": bip388.keys,
    })
}

fn wait_child(
    child: std::process::Child,
    timeout: Duration,
) -> Result<std::process::Output, SignError> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(map_ledger_cli_error(&format!("wait ledger_cli.py: {e}"))),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(map_ledger_cli_error("operation timed out")),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(map_ledger_cli_error("ledger_cli.py worker disconnected"))
        }
    }
}

fn run_cli(config: &LedgerCliConfig, command: &str, payload: &Value) -> Result<CliResponse, SignError> {
    let stdin_json = serde_json::to_string(payload)
        .map_err(|e| SignError::Ledger(format!("encode request: {e}")))?;

    let mut cmd = Command::new(&config.python);
    for arg in &config.python_prefix {
        cmd.arg(arg);
    }
    let mut child = cmd
        .arg(&config.script)
        .arg(command)
        .arg("--chain")
        .arg(&config.chain)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| SignError::Binary(format!("start ledger_cli.py: {e}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_json.as_bytes())
            .map_err(|e| SignError::Ledger(format!("write stdin: {e}")))?;
    }

    let timeout = if command == "probe" {
        LEDGER_PROBE_TIMEOUT
    } else {
        LEDGER_CLI_TIMEOUT
    };
    let output = wait_child(child, timeout)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let parsed: CliResponse = serde_json::from_str(stdout.trim()).unwrap_or(CliResponse {
        ok: false,
        error: Some(if stdout.trim().is_empty() {
            if stderr.trim().is_empty() {
                format!("ledger_cli.py exited with {}", output.status)
            } else {
                stderr.trim().to_string()
            }
        } else {
            stdout.trim().to_string()
        }),
        hmac: None,
        psbt: None,
        app_name: None,
        app_version: None,
    });

    if parsed.ok {
        return Ok(parsed);
    }

    let msg = parsed
        .error
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if stderr.trim().is_empty() {
                "ledger-bitcoin command failed".into()
            } else {
                stderr.trim().to_string()
            }
        });
    Err(map_ledger_cli_error(&msg))
}

pub fn probe_device(config: &LedgerCliConfig) -> Result<(String, String), SignError> {
    let resp = run_cli(config, "probe", &serde_json::json!({}))?;
    let name = resp
        .app_name
        .filter(|s| !s.is_empty())
        .ok_or_else(|| map_ledger_cli_error("probe missing appName"))?;
    let version = resp
        .app_version
        .filter(|s| !s.is_empty())
        .ok_or_else(|| map_ledger_cli_error("probe missing appVersion"))?;
    Ok((name, version))
}

pub fn register_wallet(
    config: &LedgerCliConfig,
    bip388: &Bip388Policy,
) -> Result<String, SignError> {
    let resp = run_cli(config, "register", &policy_payload(bip388))?;
    resp.hmac
        .filter(|h| h.len() == 64)
        .ok_or_else(|| map_ledger_cli_error("register missing hmac in response"))
}

pub fn sign_psbt(
    config: &LedgerCliConfig,
    bip388: &Bip388Policy,
    hmac: &str,
    psbt_base64: &str,
) -> Result<String, SignError> {
    let mut payload = policy_payload(bip388);
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("hmac".into(), Value::String(hmac.trim().to_ascii_lowercase()));
        obj.insert(
            "psbt".into(),
            Value::String(psbt_base64.trim().replace(['\n', '\r', ' '], "")),
        );
    }
    let resp = run_cli(config, "sign", &payload)?;
    resp.psbt
        .ok_or_else(|| map_ledger_cli_error("sign missing psbt in response"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_chain_maps_testnet4_to_test() {
        assert_eq!(ledger_chain(NetworkName::Testnet4), "test");
    }
}
