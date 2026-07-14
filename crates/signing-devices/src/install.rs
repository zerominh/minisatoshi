//! Resolve or auto-install the official HWI binary into app data.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::error::SignError;

/// Pinned release from https://github.com/bitcoin-core/HWI/releases
pub const PINNED_HWI_VERSION: &str = "3.2.0";

const RELEASE_BASE: &str =
    "https://github.com/bitcoin-core/HWI/releases/download/3.2.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwiSource {
    /// Explicit path from Settings / request.
    Preferred,
    /// `HWI_PATH` environment variable.
    Env,
    /// Found on PATH (`hwi` / `hwi.exe`).
    SystemPath,
    /// Previously downloaded into app data.
    Cached,
    /// Freshly downloaded in this call.
    Downloaded,
}

#[derive(Debug, Clone)]
pub struct ResolvedHwi {
    pub path: PathBuf,
    pub source: HwiSource,
    pub version: String,
}

#[derive(Debug, Clone, Copy)]
struct PlatformAsset {
    archive_name: &'static str,
    archive_sha256: &'static str,
    binary_name: &'static str,
    binary_sha256: &'static str,
}

fn platform_asset() -> Result<PlatformAsset, SignError> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Ok(PlatformAsset {
            archive_name: "hwi-3.2.0-windows-x86_64.zip",
            archive_sha256: "2f1a5574647e3ce11b1a05feab2fcbbf17061937c970d321d7f4c28a7b6eca23",
            binary_name: "hwi.exe",
            binary_sha256: "e068d91b664597425a8ead02d7b86a02ad6c4b72746c42961f58a58b08f9fd79",
        });
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Ok(PlatformAsset {
            archive_name: "hwi-3.2.0-linux-x86_64.tar.gz",
            archive_sha256: "3787c791fac7380a9f23a8815e4381ddc50911e70220c5a37ee8c013ea0287cd",
            binary_name: "hwi",
            binary_sha256: "d9cc65de95e3cf93fd3c953d589184a00180624ffc5ad17aade97616a8919fa6",
        });
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return Ok(PlatformAsset {
            archive_name: "hwi-3.2.0-linux-aarch64.tar.gz",
            archive_sha256: "8cc8280f687f4ecba7ea805a33636c11d0a31ae2e19f1cafd3acee8204884afd",
            binary_name: "hwi",
            binary_sha256: "c2117b96d318be0ceac217098933834ef88376c704ca9fadacd83c9471066dcc",
        });
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok(PlatformAsset {
            archive_name: "hwi-3.2.0-mac-arm64.tar.gz",
            archive_sha256: "dd1e1c37dc9c1d3f4ba63dd1e50c0b360828090ad1b97b4e1c805ef043691d31",
            binary_name: "hwi",
            binary_sha256: "87a8991848a0216213ddf6497c753cebbda492626afaf5608c30931155c922c3",
        });
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Ok(PlatformAsset {
            archive_name: "hwi-3.2.0-mac-x86_64.tar.gz",
            archive_sha256: "a8f659ef5d51d0b00dc4dd95f32c9125769515cb2716ffc049717f37fb310107",
            binary_name: "hwi",
            binary_sha256: "b3764a530b635e7a7348c9185e09e74b389f5f585094fe316f700eec7c761875",
        });
    }
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
    )))]
    {
        Err(SignError::Unsupported(format!(
            "auto-install HWI is not supported on this platform ({})",
            std::env::consts::OS
        )))
    }
}

/// Directory used to store downloaded HWI: `{data_dir}/hwi/{version}/`.
pub fn bundled_hwi_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("hwi").join(PINNED_HWI_VERSION)
}

pub fn bundled_hwi_binary(data_dir: &Path) -> PathBuf {
    let name = if cfg!(windows) { "hwi.exe" } else { "hwi" };
    bundled_hwi_dir(data_dir).join(name)
}

/// Return true if `path` runs and prints a version (`hwi --version`).
pub fn hwi_works(path: &Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    Command::new(path)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn probe_path(path: &Path, source: HwiSource) -> Option<ResolvedHwi> {
    if !hwi_works(path) {
        return None;
    }
    Some(ResolvedHwi {
        path: path.to_path_buf(),
        source,
        version: read_version(path).unwrap_or_else(|| PINNED_HWI_VERSION.to_string()),
    })
}

fn read_version(path: &Path) -> Option<String> {
    let out = Command::new(path).arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

/// Look for an existing usable HWI without downloading.
pub fn find_hwi(preferred: Option<&Path>, data_dir: &Path) -> Option<ResolvedHwi> {
    if let Some(path) = preferred {
        if let Some(found) = probe_path(path, HwiSource::Preferred) {
            return Some(found);
        }
    }
    if let Ok(env_path) = std::env::var("HWI_PATH") {
        let path = PathBuf::from(env_path.trim());
        if let Some(found) = probe_path(&path, HwiSource::Env) {
            return Some(found);
        }
    }
    let system_name = if cfg!(windows) { "hwi.exe" } else { "hwi" };
    if let Some(found) = probe_path(Path::new(system_name), HwiSource::SystemPath) {
        return Some(found);
    }
    let cached = bundled_hwi_binary(data_dir);
    probe_path(&cached, HwiSource::Cached)
}

/// Find HWI on the machine, or download+install the pinned release into `data_dir`.
pub fn ensure_hwi(preferred: Option<&Path>, data_dir: &Path) -> Result<ResolvedHwi, SignError> {
    if let Some(found) = find_hwi(preferred, data_dir) {
        return Ok(found);
    }
    install_hwi(data_dir)?;
    let path = bundled_hwi_binary(data_dir);
    probe_path(&path, HwiSource::Downloaded).ok_or_else(|| {
        SignError::Install(format!(
            "HWI installed to {} but failed to run — check antivirus / OS permissions",
            path.display()
        ))
    })
}

/// Force (re)download of the pinned HWI release into `data_dir`.
pub fn install_hwi(data_dir: &Path) -> Result<PathBuf, SignError> {
    let asset = platform_asset()?;
    let dest_dir = bundled_hwi_dir(data_dir);
    fs::create_dir_all(&dest_dir).map_err(|e| SignError::Install(e.to_string()))?;

    let url = format!("{RELEASE_BASE}/{}", asset.archive_name);
    let archive_bytes = download_bytes(&url)?;
    verify_sha256(&archive_bytes, asset.archive_sha256)?;

    let dest_bin = dest_dir.join(asset.binary_name);
    // Keep the real extension (`.zip` / `.tar.gz`) so extract_binary can detect format.
    let tmp_archive = dest_dir.join(format!(".partial-{}", asset.archive_name));
    {
        let mut f =
            File::create(&tmp_archive).map_err(|e| SignError::Install(e.to_string()))?;
        f.write_all(&archive_bytes)
            .map_err(|e| SignError::Install(e.to_string()))?;
    }

    extract_binary(&tmp_archive, asset.binary_name, &dest_bin)?;
    let _ = fs::remove_file(&tmp_archive);

    let bin_bytes = fs::read(&dest_bin).map_err(|e| SignError::Install(e.to_string()))?;
    verify_sha256(&bin_bytes, asset.binary_sha256)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest_bin)
            .map_err(|e| SignError::Install(e.to_string()))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest_bin, perms).map_err(|e| SignError::Install(e.to_string()))?;
    }

    Ok(dest_bin)
}

fn download_bytes(url: &str) -> Result<Vec<u8>, SignError> {
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .user_agent(format!("minisatoshi/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| SignError::Download(e.to_string()))?
        .get(url)
        .send()
        .map_err(|e| SignError::Download(e.to_string()))?;

    if !response.status().is_success() {
        return Err(SignError::Download(format!(
            "HTTP {} fetching {url}",
            response.status()
        )));
    }

    response
        .bytes()
        .map(|b| b.to_vec())
        .map_err(|e| SignError::Download(e.to_string()))
}

fn verify_sha256(bytes: &[u8], expected_hex: &str) -> Result<(), SignError> {
    let digest = Sha256::digest(bytes);
    let actual = hex::encode(digest);
    if actual.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(SignError::ChecksumMismatch {
            expected: expected_hex.to_string(),
            actual,
        })
    }
}

fn extract_binary(archive: &Path, binary_name: &str, dest: &Path) -> Result<(), SignError> {
    let name = archive
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.ends_with(".zip") {
        extract_zip_member(archive, binary_name, dest)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        extract_tar_gz_member(archive, binary_name, dest)
    } else {
        Err(SignError::Install(format!(
            "unsupported archive format: {name}"
        )))
    }
}

fn extract_zip_member(archive: &Path, binary_name: &str, dest: &Path) -> Result<(), SignError> {
    let file = File::open(archive).map_err(|e| SignError::Install(e.to_string()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| SignError::Install(e.to_string()))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| SignError::Install(e.to_string()))?;
        let entry_name = entry.name().replace('\\', "/");
        let file_name = entry_name.rsplit('/').next().unwrap_or(&entry_name);
        if file_name != binary_name {
            continue;
        }
        let mut out = File::create(dest).map_err(|e| SignError::Install(e.to_string()))?;
        io::copy(&mut entry, &mut out).map_err(|e| SignError::Install(e.to_string()))?;
        return Ok(());
    }
    Err(SignError::Install(format!(
        "{binary_name} not found inside zip archive"
    )))
}

fn extract_tar_gz_member(archive: &Path, binary_name: &str, dest: &Path) -> Result<(), SignError> {
    let file = File::open(archive).map_err(|e| SignError::Install(e.to_string()))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive
        .entries()
        .map_err(|e| SignError::Install(e.to_string()))?
    {
        let mut entry = entry.map_err(|e| SignError::Install(e.to_string()))?;
        let path = entry
            .path()
            .map_err(|e| SignError::Install(e.to_string()))?;
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if file_name != binary_name {
            continue;
        }
        let mut out = File::create(dest).map_err(|e| SignError::Install(e.to_string()))?;
        io::copy(&mut entry, &mut out).map_err(|e| SignError::Install(e.to_string()))?;
        return Ok(());
    }
    Err(SignError::Install(format!(
        "{binary_name} not found inside tar.gz archive"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn bundled_paths_include_version() {
        let dir = PathBuf::from("/tmp/minisatoshi-data");
        let p = bundled_hwi_binary(&dir);
        assert!(p.to_string_lossy().contains(PINNED_HWI_VERSION));
    }

    #[test]
    fn verify_sha256_ok_and_fail() {
        let data = b"hello-hwi";
        let hash = hex::encode(Sha256::digest(data));
        assert!(verify_sha256(data, &hash).is_ok());
        assert!(matches!(
            verify_sha256(data, "00"),
            Err(SignError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn extract_zip_picks_named_binary() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("fake.zip");
        {
            let file = File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("nested/hwi.exe", opts).unwrap();
            zip.write_all(b"fake-hwi-bytes").unwrap();
            zip.finish().unwrap();
        }
        let dest = dir.path().join("out.exe");
        extract_zip_member(&zip_path, "hwi.exe", &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"fake-hwi-bytes");
    }

    #[test]
    fn platform_asset_defined_for_this_target() {
        // Should succeed on Windows/Linux/macOS CI targets we support.
        assert!(platform_asset().is_ok());
    }
}
