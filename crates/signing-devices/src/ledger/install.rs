//! Install and resolve the ledger-bitcoin Python runtime into app data.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::SignError;

pub const PINNED_LEDGER_BITCOIN_VERSION: &str = "0.4.1";
pub const LEDGER_CLI_SCRIPT_VERSION: &str = "5";
/// Pip extras required for USB HID (Ledger). Bumping forces `ensure_ledger_runtime` reinstall.
pub const RUNTIME_DEPS_TAG: &str = "ledgercomm[hid]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerRuntimeSource {
    Env,
    Venv,
    System,
}

#[derive(Debug, Clone)]
pub struct ResolvedLedgerRuntime {
    pub python: PathBuf,
    pub python_prefix: Vec<String>,
    pub script: PathBuf,
    pub source: LedgerRuntimeSource,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeManifest {
    ledger_bitcoin_version: String,
    cli_script_version: String,
    #[serde(default)]
    runtime_deps_tag: String,
    installed_at_secs: u64,
}

#[derive(Debug, Clone)]
struct PythonCandidate {
    program: PathBuf,
    prefix: Vec<String>,
}

pub fn ledger_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("ledger")
}

pub fn ledger_venv_dir(data_dir: &Path) -> PathBuf {
    ledger_dir(data_dir).join("venv")
}

pub fn venv_python_path(venv: &Path) -> PathBuf {
    if cfg!(windows) {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python3")
    }
}

pub fn manifest_path(data_dir: &Path) -> PathBuf {
    ledger_dir(data_dir).join("runtime.json")
}

pub fn bundled_ledger_cli_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/ledger_cli.py")
}

/// Copy bundled `ledger_cli.py` into app data (always refresh from repo bundle).
pub fn ensure_ledger_cli_script(data_dir: &Path) -> Result<PathBuf, SignError> {
    let dest_dir = ledger_dir(data_dir);
    fs::create_dir_all(&dest_dir).map_err(|e| SignError::Ledger(format!("create ledger dir: {e}")))?;
    let dest = dest_dir.join("ledger_cli.py");
    let bundled = bundled_ledger_cli_script();
    fs::copy(&bundled, &dest).map_err(|e| {
        SignError::Ledger(format!(
            "copy ledger_cli.py from {}: {e}",
            bundled.display()
        ))
    })?;
    Ok(dest)
}

pub fn runtime_source_label(source: LedgerRuntimeSource) -> &'static str {
    match source {
        LedgerRuntimeSource::Env => "env",
        LedgerRuntimeSource::Venv => "bundled",
        LedgerRuntimeSource::System => "system",
    }
}

pub fn ledger_import_works(python: &Path, prefix: &[String]) -> bool {
    let mut cmd = Command::new(python);
    for arg in prefix {
        cmd.arg(arg);
    }
    cmd.args(["-c", "import ledger_bitcoin"]);
    cmd.output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// USB HID via hidapi — required for Ledger on Windows/macOS/Linux.
pub fn ledger_hid_works(python: &Path, prefix: &[String]) -> bool {
    let mut cmd = Command::new(python);
    for arg in prefix {
        cmd.arg(arg);
    }
    cmd.args(["-c", "import hid"]);
    cmd.output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn runtime_manifest(data_dir: &Path) -> Option<RuntimeManifest> {
    let raw = fs::read_to_string(manifest_path(data_dir)).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_manifest(data_dir: &Path) -> Result<(), SignError> {
    let manifest = RuntimeManifest {
        ledger_bitcoin_version: PINNED_LEDGER_BITCOIN_VERSION.to_string(),
        cli_script_version: LEDGER_CLI_SCRIPT_VERSION.to_string(),
        runtime_deps_tag: RUNTIME_DEPS_TAG.to_string(),
        installed_at_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| SignError::Ledger(format!("serialize runtime manifest: {e}")))?;
    fs::write(manifest_path(data_dir), json)
        .map_err(|e| SignError::Ledger(format!("write runtime manifest: {e}")))?;
    Ok(())
}

fn read_ledger_bitcoin_version(python: &Path, prefix: &[String]) -> Option<String> {
    let mut cmd = Command::new(python);
    for arg in prefix {
        cmd.arg(arg);
    }
    let out = cmd
        .args([
            "-c",
            "import ledger_bitcoin; print(getattr(ledger_bitcoin, '__version__', 'unknown'))",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}

fn probe_python(
    python: &Path,
    prefix: &[String],
    source: LedgerRuntimeSource,
    script: &Path,
) -> Option<ResolvedLedgerRuntime> {
    if !script.exists() || !ledger_import_works(python, prefix) || !ledger_hid_works(python, prefix) {
        return None;
    }
    let version = read_ledger_bitcoin_version(python, prefix)
        .unwrap_or_else(|| PINNED_LEDGER_BITCOIN_VERSION.to_string());
    Some(ResolvedLedgerRuntime {
        python: python.to_path_buf(),
        python_prefix: prefix.to_vec(),
        script: script.to_path_buf(),
        source,
        version,
    })
}

fn system_python_candidates() -> Vec<PythonCandidate> {
    let mut out = Vec::new();
    if cfg!(windows) {
        out.push(PythonCandidate {
            program: PathBuf::from("py"),
            prefix: vec!["-3".into()],
        });
    }
    for name in ["python3", "python"] {
        out.push(PythonCandidate {
            program: PathBuf::from(name),
            prefix: vec![],
        });
    }
    out
}

fn bootstrap_python() -> Result<PythonCandidate, SignError> {
    for candidate in system_python_candidates() {
        let mut cmd = Command::new(&candidate.program);
        for arg in &candidate.prefix {
            cmd.arg(arg);
        }
        if cmd
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(candidate);
        }
    }
    Err(SignError::Binary(
        "Python 3 not found for Ledger signer install. Install Python 3.10+ from python.org, \
         then use Settings → Install Ledger signer (one-time bootstrap).".into(),
    ))
}

fn run_python(program: &Path, prefix: &[String], args: &[&str]) -> Result<Output, SignError> {
    let mut cmd = Command::new(program);
    for part in prefix {
        cmd.arg(part);
    }
    cmd.args(args);
    cmd.output()
        .map_err(|e| SignError::Ledger(format!("run {}: {e}", program.display())))
}

fn create_venv(bootstrap: &PythonCandidate, venv: &Path) -> Result<(), SignError> {
    if venv.exists() {
        return Ok(());
    }
    if let Some(parent) = venv.parent() {
        fs::create_dir_all(parent).map_err(|e| SignError::Ledger(e.to_string()))?;
    }
    let venv_arg = venv
        .to_str()
        .ok_or_else(|| SignError::Ledger("invalid venv path".into()))?;
    let out = run_python(
        &bootstrap.program,
        &bootstrap.prefix,
        &["-m", "venv", venv_arg],
    )?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(SignError::Ledger(format!("create venv: {stderr}")));
    }
    Ok(())
}

fn pip_install(venv_py: &Path) -> Result<(), SignError> {
    let pip_ok = run_python(venv_py, &[], &["-m", "pip", "--version"])
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !pip_ok {
        let out = run_python(venv_py, &[], &["-m", "ensurepip", "--upgrade"])?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(SignError::Ledger(format!("ensurepip: {stderr}")));
        }
    }

    let pkg = format!("ledger-bitcoin=={PINNED_LEDGER_BITCOIN_VERSION}");
    let out = run_python(
        venv_py,
        &[],
        &[
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--no-input",
            &pkg,
            RUNTIME_DEPS_TAG,
        ],
    )?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        return Err(SignError::Ledger(format!(
            "pip install ledger-bitcoin + USB HID deps: {stderr}{stdout}"
        )));
    }
    if !ledger_hid_works(venv_py, &[]) {
        return Err(SignError::Ledger(
            "hidapi (USB) failed to install — retry Settings → Install Ledger signer, or check antivirus / network."
                .into(),
        ));
    }
    Ok(())
}

/// Look for a usable ledger-bitcoin runtime without installing.
pub fn find_ledger_runtime(data_dir: &Path) -> Option<ResolvedLedgerRuntime> {
    let script = ensure_ledger_cli_script(data_dir).ok()?;

    for var in ["MINISATOSHI_PYTHON", "LEDGER_PYTHON"] {
        if let Ok(raw) = std::env::var(var) {
            let path = PathBuf::from(raw.trim());
            if let Some(found) = probe_python(&path, &[], LedgerRuntimeSource::Env, &script) {
                return Some(found);
            }
        }
    }

    let venv_py = venv_python_path(&ledger_venv_dir(data_dir));
    if let Some(found) = probe_python(&venv_py, &[], LedgerRuntimeSource::Venv, &script) {
        return Some(found);
    }

    for candidate in system_python_candidates() {
        if let Some(found) = probe_python(
            &candidate.program,
            &candidate.prefix,
            LedgerRuntimeSource::System,
            &script,
        ) {
            return Some(found);
        }
    }

    None
}

/// Create venv + install pinned ledger-bitcoin into app data.
pub fn install_ledger_runtime(data_dir: &Path) -> Result<ResolvedLedgerRuntime, SignError> {
    ensure_ledger_cli_script(data_dir)?;
    let bootstrap = bootstrap_python()?;
    let venv = ledger_venv_dir(data_dir);
    create_venv(&bootstrap, &venv)?;
    let venv_py = venv_python_path(&venv);
    pip_install(&venv_py)?;
    write_manifest(data_dir)?;
    find_ledger_runtime(data_dir).ok_or_else(|| {
        SignError::Ledger(
            "Ledger runtime installed but failed verification — check network / antivirus".into(),
        )
    })
}

/// Find or install the ledger-bitcoin runtime; refresh when pinned versions change.
pub fn ensure_ledger_runtime(data_dir: &Path) -> Result<ResolvedLedgerRuntime, SignError> {
    let needs_reinstall = runtime_manifest(data_dir)
        .map(|manifest| {
            manifest.ledger_bitcoin_version != PINNED_LEDGER_BITCOIN_VERSION
                || manifest.cli_script_version != LEDGER_CLI_SCRIPT_VERSION
                || manifest.runtime_deps_tag != RUNTIME_DEPS_TAG
        })
        .unwrap_or(true);

    if let Some(found) = find_ledger_runtime(data_dir) {
        ensure_ledger_cli_script(data_dir)?;
        if !needs_reinstall {
            return Ok(found);
        }
    }

    install_ledger_runtime(data_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn bundled_script_exists() {
        assert!(bundled_ledger_cli_script().exists());
    }

    #[test]
    fn venv_python_path_windows_style() {
        let root = PathBuf::from("/tmp/venv");
        let py = venv_python_path(&root);
        if cfg!(windows) {
            assert!(py.ends_with("Scripts\\python.exe") || py.ends_with("Scripts/python.exe"));
        } else {
            assert!(py.ends_with("bin/python3"));
        }
    }

    #[test]
    fn manifest_roundtrip() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(ledger_dir(dir.path())).unwrap();
        write_manifest(dir.path()).unwrap();
        let manifest = runtime_manifest(dir.path()).unwrap();
        assert_eq!(manifest.ledger_bitcoin_version, PINNED_LEDGER_BITCOIN_VERSION);
        assert_eq!(manifest.runtime_deps_tag, RUNTIME_DEPS_TAG);
    }
}
