//! Map Minisatoshi vault descriptors to hardware registration formats
//! (BIP-388 / Ledger wallet policy, Coldcard MicroSD text).

use descriptor_engine::compile_descriptor_from_abstract;
use policy_engine::{KeyConfig, NetworkName, PolicyConfig};
use serde::{Deserialize, Serialize};

use miniscript::descriptor::{Descriptor, TapTree};
use miniscript::{MiniscriptKey};

use crate::error::SignError;
use crate::types::DeviceType;

/// BIP-388 style wallet policy with key placeholders (`@0`, `@1`, …).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bip388Policy {
    pub name: String,
    /// Descriptor / policy template with `@n/…` placeholders.
    pub policy: String,
    /// Key info strings: `[fingerprint/origin]xpub`.
    pub keys: Vec<String>,
}

/// Stable SHA-256 hex fingerprint of a BIP-388 policy (template + keys).
pub fn bip388_policy_fingerprint(bip388: &Bip388Policy) -> String {
    use sha2::{Digest, Sha256};
    let canonical = serde_json::json!({
        "keys": bip388.keys,
        "name": bip388.name,
        "policy": bip388.policy,
    });
    let bytes = serde_json::to_vec(&canonical).expect("bip388 json");
    hex::encode(Sha256::digest(bytes))
}

/// Ledger Bitcoin app limits (see Ledger `app-bitcoin-new` docs).
pub const LEDGER_MAX_POLICY_TEMPLATE_LEN: usize = 252;

/// Adapt a BIP-388 package for `ledger-bitcoin` / Ledger V2 wallet policies.
///
/// Ledger expects `@n/**` placeholders, network-encoded extended pubkeys (`tpub` on testnet),
/// and a **binary** Taproot tree (`{left,right}`), not miniscript-rs depth encoding (`{{a,b,c,d}}`).
pub fn to_ledger_wallet_policy(
    bip388: &Bip388Policy,
    config: &PolicyConfig,
    network: NetworkName,
) -> Result<Bip388Policy, SignError> {
    let policy = ledger_policy_template(config, bip388)?;
    let policy = normalize_ledger_policy_template(&policy)?;
    validate_ledger_policy_template(&policy)?;
    let keys = bip388
        .keys
        .iter()
        .map(|k| normalize_ledger_key_info(k, network))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Bip388Policy {
        name: bip388.name.clone(),
        policy,
        keys,
    })
}

/// Rebuild a Ledger policy template (binary taptree + `@n/**`) from the vault policy config.
fn ledger_policy_template(config: &PolicyConfig, bip388: &Bip388Policy) -> Result<String, SignError> {
    let parsed = compile_descriptor_from_abstract(config)
        .map_err(|e| SignError::Unsupported(format!("compile descriptor for Ledger: {e}")))?;

    let policy = match parsed {
        Descriptor::Tr(tr) => {
            let internal = tr.internal_key().to_string();
            let tree = tr
                .tap_tree()
                .map(taptree_binary_fmt)
                .unwrap_or_default();
            if tree.is_empty() {
                format!("tr({internal})")
            } else {
                format!("tr({internal},{tree})")
            }
        }
        _ => bip388.policy.clone(),
    };

    apply_bip388_placeholders(&policy, bip388)
}

/// Ledger / embit expect explicit `{left,right}` Taproot trees, not depth-compressed `{{a,b,c,d}}`.
fn taptree_binary_fmt<Pk: MiniscriptKey>(tree: &TapTree<Pk>) -> String {
    let scripts: Vec<String> = tree
        .leaves()
        .map(|item| item.miniscript().to_string())
        .collect();
    binary_taptree_from_leaf_strings(&scripts)
}

fn binary_taptree_from_leaf_strings(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return String::new();
    }
    if leaves.len() == 1 {
        return leaves[0].clone();
    }
    let mid = leaves.len() / 2;
    format!(
        "{{{},{}}}",
        binary_taptree_from_leaf_strings(&leaves[..mid]),
        binary_taptree_from_leaf_strings(&leaves[mid..])
    )
}

fn apply_bip388_placeholders(policy: &str, bip388: &Bip388Policy) -> Result<String, SignError> {
    let mut out = policy.to_string();
    for (index, key_info) in bip388.keys.iter().enumerate() {
        let fp = key_fingerprint(key_info)?;
        let xpub = key_xpub_part(key_info)?;
        let mut search_from = 0;
        while let Some(rel) = out[search_from..]
            .to_ascii_lowercase()
            .find(&fp.to_ascii_lowercase())
        {
            let fp_at = search_from + rel;
            let Some(start) = out[..fp_at].rfind('[') else {
                break;
            };
            let Some(bracket_rel) = out[fp_at..].find(']') else {
                break;
            };
            let after_bracket = fp_at + bracket_rel + 1;
            if !out[after_bracket..].to_ascii_lowercase().starts_with(&xpub.to_ascii_lowercase()) {
                search_from = fp_at + 8;
                continue;
            }
            let xpub_end = after_bracket + xpub.len();
            let end = extend_derivation_end(&out, xpub_end);
            let needle = out[start..end].to_string();
            let placeholder = format!("@{index}/**");
            out = out.replace(&needle, &placeholder);
            search_from = start + placeholder.len();
        }
    }
    Ok(out)
}

fn key_fingerprint(key_info: &str) -> Result<String, SignError> {
    let trimmed = key_info.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|s| s.split('/').next())
        .ok_or_else(|| SignError::Unsupported(format!("invalid key info: {trimmed}")))?;
    Ok(inner.to_ascii_lowercase())
}

fn key_xpub_part(key_info: &str) -> Result<String, SignError> {
    let trimmed = key_info.trim();
    let bracket_end = trimmed
        .find(']')
        .ok_or_else(|| SignError::Unsupported(format!("invalid key info: {trimmed}")))?;
    let xpub = trimmed[bracket_end + 1..].trim();
    let xpub = xpub.strip_suffix("/**").unwrap_or(xpub);
    if xpub.is_empty() {
        return Err(SignError::Unsupported(format!("invalid key info: {trimmed}")));
    }
    Ok(xpub.to_string())
}

pub fn validate_ledger_policy_template(policy: &str) -> Result<(), SignError> {
    if policy.len() > LEDGER_MAX_POLICY_TEMPLATE_LEN {
        return Err(SignError::Unsupported(format!(
            "wallet policy template is {} bytes — Ledger supports at most {LEDGER_MAX_POLICY_TEMPLATE_LEN}. \
             Simplify the vault or use another signer (Coldcard / software cosigner).",
            policy.len()
        )));
    }
    Ok(())
}

fn normalize_ledger_policy_template(policy: &str) -> Result<String, SignError> {
    let mut out = policy.to_string();
    for i in 0..16u8 {
        let multisite = format!("@{i}/<0;1>/*");
        let single = format!("@{i}/*");
        let ledger = format!("@{i}/**");
        out = out.replace(&multisite, &ledger);
        if !out.contains(&ledger) {
            out = out.replace(&single, &ledger);
        }
    }
    Ok(out)
}

fn normalize_ledger_key_info(key_info: &str, network: NetworkName) -> Result<String, SignError> {
    let trimmed = key_info.trim();
    let bracket_end = trimmed
        .find(']')
        .ok_or_else(|| SignError::Unsupported(format!("invalid key info: {trimmed}")))?;
    let origin = &trimmed[..=bracket_end];
    let mut xpub_part = trimmed[bracket_end + 1..].trim();
    if let Some(stripped) = xpub_part.strip_suffix("/**") {
        xpub_part = stripped;
    }
    let converted = xpub_for_network(xpub_part, network)?;
    Ok(format!("{origin}{converted}"))
}

fn xpub_for_network(xpub_b58: &str, network: NetworkName) -> Result<String, SignError> {
    let want_test = !matches!(network, NetworkName::Mainnet);
    let is_test = xpub_b58.starts_with("tpub")
        || xpub_b58.starts_with("vpub")
        || xpub_b58.starts_with("upub");
    let is_main = xpub_b58.starts_with("xpub")
        || xpub_b58.starts_with("zpub")
        || xpub_b58.starts_with("ypub");
    if (want_test && is_test) || (!want_test && is_main) || (!is_test && !is_main) {
        return Ok(xpub_b58.to_string());
    }
    swap_xpub_version_bytes(xpub_b58, network)
}

fn swap_xpub_version_bytes(xpub_b58: &str, network: NetworkName) -> Result<String, SignError> {
    let mut data = bitcoin::base58::decode_check(xpub_b58.trim())
        .map_err(|e| SignError::Unsupported(format!("invalid extended pubkey: {e}")))?;
    if data.len() < 4 {
        return Err(SignError::Unsupported(
            "invalid extended pubkey payload".into(),
        ));
    }
    let version: [u8; 4] = match network {
        NetworkName::Mainnet => [0x04, 0x88, 0xB2, 0x1E],
        NetworkName::Testnet
        | NetworkName::Testnet4
        | NetworkName::Signet
        | NetworkName::Regtest => [0x04, 0x35, 0x87, 0xCF],
    };
    data[..4].copy_from_slice(&version);
    Ok(bitcoin::base58::encode_check(&data))
}

/// Per-vendor registration payload and human instructions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorRegistration {
    pub device_type: String,
    pub title: String,
    pub body: String,
    pub instructions: Vec<String>,
}

/// Full registration package for a vault (export + optional on-device register).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationPackage {
    pub vault_name: String,
    pub network: String,
    pub descriptor: String,
    pub bip388: Bip388Policy,
    pub coldcard_sd_text: String,
    pub ledger_hmac: Option<String>,
    pub vendors: Vec<VendorRegistration>,
    /// True when HWI CLI exposes `registerpolicy` (not in stock 3.2.0).
    pub hwi_registerpolicy_supported: bool,
}

/// Miniscript Taproot wallets cannot use HWI 3.2 `displayaddress` (`and_v` rejected) and
/// have no `registerpolicy` — Ledger registers on the first PSBT sign instead.
///
/// Also: stock HWI 3.2 `ledger.py` has `TODO: Support script path signing` for
/// `tap_bip32_paths`, so ABC-style script-path spends never receive signatures.
pub fn ledger_registers_on_first_psbt(descriptor: &str) -> bool {
    is_taproot_script_path_miniscript(descriptor)
}

/// True for `tr(NUMS, {{and_v(...), ...}})`-style script-path Miniscript descriptors.
pub fn is_taproot_script_path_miniscript(descriptor: &str) -> bool {
    let d = descriptor.trim();
    d.starts_with("tr(")
        && (d.contains("and_v")
            || d.contains("andor")
            || d.contains("thresh(")
            || d.contains("older(")
            || d.contains("{{"))
}

/// Build registration materials from policy + compiled descriptor.
pub fn build_registration_package(
    vault_name: &str,
    config: &PolicyConfig,
    descriptor: &str,
) -> Result<RegistrationPackage, SignError> {
    let bip388 = descriptor_to_bip388(vault_name, config, descriptor)?;
    let coldcard_sd_text = format_coldcard_sd(vault_name, config.network, descriptor, &bip388);
    let vendors = vec![
        ledger_vendor(&bip388),
        coldcard_vendor(),
        trezor_vendor(),
    ];

    Ok(RegistrationPackage {
        vault_name: vault_name.to_string(),
        network: network_label(config.network).to_string(),
        descriptor: descriptor.trim().to_string(),
        bip388,
        coldcard_sd_text,
        ledger_hmac: None,
        vendors,
        hwi_registerpolicy_supported: false,
    })
}

/// Convert a checksummed output descriptor into BIP-388 policy + key infos.
pub fn descriptor_to_bip388(
    name: &str,
    config: &PolicyConfig,
    descriptor: &str,
) -> Result<Bip388Policy, SignError> {
    let (body, _checksum) = split_checksum(descriptor.trim());
    if body.is_empty() {
        return Err(SignError::Unsupported("empty descriptor".into()));
    }

    let ordered = ordered_keys(config);
    if ordered.is_empty() {
        return Err(SignError::Unsupported(
            "vault has no keys to register".into(),
        ));
    }

    let mut policy = body.to_string();
    let mut key_infos = Vec::with_capacity(ordered.len());
    let mut replaced = 0usize;

    for (index, key) in ordered.iter().enumerate() {
        if let Some((info, needle)) = find_key_in_descriptor(&policy, key) {
            let suffix = derivation_suffix_after_xpub(&needle);
            let placeholder = format!("@{index}{suffix}");
            policy = policy.replace(&needle, &placeholder);
            key_infos.push(info);
            replaced += 1;
        } else {
            key_infos.push(key_info_string(key));
        }
    }

    if replaced == 0 {
        return Err(SignError::Unsupported(
            "could not map descriptor keys to BIP-388 @placeholders — check fingerprints/xpubs match the vault descriptor".into(),
        ));
    }

    Ok(Bip388Policy {
        name: sanitize_wallet_name(name),
        policy,
        keys: key_infos,
    })
}

fn ordered_keys(config: &PolicyConfig) -> Vec<&KeyConfig> {
    let mut keys: Vec<&KeyConfig> = config.keys.iter().collect();
    keys.sort_by(|a, b| a.id.cmp(&b.id));
    keys
}

fn split_checksum(descriptor: &str) -> (&str, Option<&str>) {
    match descriptor.rsplit_once('#') {
        Some((body, sum)) if sum.len() == 8 && sum.chars().all(|c| c.is_ascii_alphanumeric()) => {
            (body, Some(sum))
        }
        _ => (descriptor, None),
    }
}

fn key_info_string(key: &KeyConfig) -> String {
    let fp = key.fingerprint.trim().to_ascii_lowercase();
    match key.origin_path.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        Some(path) => {
            let path = path.trim_start_matches("m/").trim_start_matches("M/");
            format!("[{fp}/{path}]{}", key.xpub.trim())
        }
        None => format!("[{fp}]{}", key.xpub.trim()),
    }
}

/// Find a vault key by master fingerprint (case-insensitive).
pub fn find_key_by_fingerprint<'a>(
    keys: &'a [KeyConfig],
    fingerprint: &str,
) -> Option<&'a KeyConfig> {
    let want = fingerprint.trim().to_ascii_lowercase();
    keys.iter()
        .find(|k| k.fingerprint.trim().to_ascii_lowercase() == want)
}

/// Single-key Taproot descriptor HWI `displayaddress` accepts (no `and_v`).
pub fn single_key_display_descriptor(key: &KeyConfig) -> String {
    format!("tr({}/<0;1>/*)", key_info_string(key))
}

/// Locate `[fp/…]xpub…[/derivation]` inside the descriptor body.
fn find_key_in_descriptor(descriptor: &str, key: &KeyConfig) -> Option<(String, String)> {
    let fp = key.fingerprint.trim().to_ascii_lowercase();
    let xpub = key.xpub.trim();
    if fp.len() != 8 || xpub.is_empty() {
        return None;
    }

    let lower = descriptor.to_ascii_lowercase();
    let fp_lower = fp.to_ascii_lowercase();
    let mut search_from = 0;
    while let Some(rel) = lower[search_from..].find(&fp_lower) {
        let fp_at = search_from + rel;
        // Walk left to '['
        let start = descriptor[..fp_at].rfind('[')?;
        // Find xpub after ']'
        let after_bracket = descriptor[fp_at..].find(']')? + fp_at + 1;
        if !descriptor[after_bracket..].starts_with(xpub)
            && !descriptor[after_bracket..]
                .to_ascii_lowercase()
                .starts_with(&xpub.to_ascii_lowercase())
        {
            // fingerprint matched elsewhere — continue
            search_from = fp_at + 8;
            continue;
        }
        let xpub_end = after_bracket + xpub.len();
        let end = extend_derivation_end(descriptor, xpub_end);
        let needle = descriptor[start..end].to_string();
        let info = descriptor[start..xpub_end].to_string();
        return Some((info, needle));
    }
    None
}

fn extend_derivation_end(descriptor: &str, mut end: usize) -> usize {
    let rest = &descriptor[end..];
    if rest.starts_with("/<0;1>/*") {
        end += "/<0;1>/*".len();
    } else if rest.starts_with("/**") {
        end += "/**".len();
    } else if rest.starts_with("/*") {
        end += "/*".len();
    } else if rest.starts_with("/0/*") {
        end += "/0/*".len();
    } else if rest.starts_with("/1/*") {
        end += "/1/*".len();
    }
    end
}

fn derivation_suffix_after_xpub(needle: &str) -> String {
    if let Some(idx) = needle.find("]") {
        let after = &needle[idx + 1..];
        // skip xpub body
        if let Some(slash) = after.find('/') {
            return after[slash..].to_string();
        }
    }
    "/**".to_string()
}

fn sanitize_wallet_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "Minisatoshi".into();
    }
    trimmed.chars().take(64).collect()
}

fn network_label(network: NetworkName) -> &'static str {
    match network {
        NetworkName::Mainnet => "mainnet",
        NetworkName::Testnet => "testnet",
        NetworkName::Testnet4 => "testnet4",
        NetworkName::Signet => "signet",
        NetworkName::Regtest => "regtest",
    }
}

/// HWI `--chain` value for this network.
pub fn hwi_chain(network: NetworkName) -> &'static str {
    match network {
        NetworkName::Mainnet => "main",
        NetworkName::Testnet => "test",
        NetworkName::Testnet4 => "testnet4",
        NetworkName::Signet => "signet",
        NetworkName::Regtest => "regtest",
    }
}

fn format_coldcard_sd(
    vault_name: &str,
    network: NetworkName,
    descriptor: &str,
    bip388: &Bip388Policy,
) -> String {
    format!(
        "# Minisatoshi vault — Coldcard / MicroSD\n\
         # Name: {vault_name}\n\
         # Network: {}\n\
         # Import via Advanced → MicroSD → Descriptor / or keep as backup.\n\
         #\n\
         # Full output descriptor:\n\
         {descriptor}\n\
         #\n\
         # BIP-388 policy template:\n\
         {}\n\
         #\n\
         # Keys:\n\
         {}\n",
        network_label(network),
        bip388.policy,
        bip388
            .keys
            .iter()
            .enumerate()
            .map(|(i, k)| format!("#  @{i} = {k}"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn ledger_vendor(bip388: &Bip388Policy) -> VendorRegistration {
    VendorRegistration {
        device_type: DeviceType::Ledger.as_str().into(),
        title: "Ledger (wallet policy)".into(),
        body: serde_json::to_string_pretty(bip388).unwrap_or_else(|_| bip388.policy.clone()),
        instructions: vec![
            "Open the Bitcoin app on the Ledger (taproot / Miniscript capable firmware).".into(),
            "In Minisatoshi: Wallet → Settings → Register on hardware → Register Ledger policy.".into(),
            "Approve the wallet policy on the Ledger screen — HMAC is stored locally for signing.".into(),
            "Requires Python 3 with pip install ledger-bitcoin (bundled in a future release).".into(),
            "Confirm address on device after registering (display / receive verify).".into(),
            "HMAC proof of registration (if returned) can be stored for later signing sessions."
                .into(),
        ],
    }
}

fn coldcard_vendor() -> VendorRegistration {
    VendorRegistration {
        device_type: DeviceType::Coldcard.as_str().into(),
        title: "Coldcard (MicroSD descriptor)".into(),
        body: String::new(), // filled by package coldcard_sd_text for save dialog
        instructions: vec![
            "Save the Coldcard MicroSD text file from this screen.".into(),
            "Copy the file onto a MicroSD card and insert into the Coldcard.".into(),
            "Use Advanced → MicroSD → import/export descriptor features (Mk4+), or paste \
             the descriptor into a co-signing flow / Airgap PSBT."
                .into(),
            "For multi-sig ABC vaults, Investor (A) + Manager (B) must both sign primary spends."
                .into(),
        ],
    }
}

fn trezor_vendor() -> VendorRegistration {
    VendorRegistration {
        device_type: DeviceType::Trezor.as_str().into(),
        title: "Trezor".into(),
        body: String::new(),
        instructions: vec![
            "Trezor + HWI can sign many scripts; Miniscript/taproot script-path support \
             depends on firmware — verify on testnet first."
                .into(),
            "Prefer registering/exporting the same BIP-388 policy used for Ledger when tools allow."
                .into(),
            "Sign PSBT in Minisatoshi Send after the co-signer policy is understood on-device."
                .into(),
        ],
    }
}

/// Which key roles are typically required for the primary policy path.
pub fn primary_cosigner_hints(config: &PolicyConfig) -> Vec<String> {
    let primary = config.policy.primary.to_ascii_uppercase();
    config
        .keys
        .iter()
        .filter(|k| {
            let id = k.id.to_ascii_uppercase();
            // Mention keys that appear in primary (rough filter for UI).
            primary.split(|c: char| !c.is_ascii_alphanumeric()).any(|t| t == id)
                || primary.contains(&id)
        })
        .map(|k| {
            let role = match k.role {
                policy_engine::KeyRole::Investor => "investor",
                policy_engine::KeyRole::Manager => "manager",
                policy_engine::KeyRole::Recovery => "recovery",
                policy_engine::KeyRole::Cosigner => "cosigner",
                policy_engine::KeyRole::Other => "other",
            };
            format!("{} ({role}) · fp {}", k.id, k.fingerprint)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use policy_engine::{
        FallbackPolicy, KeyConfig, KeyRole, PolicyConfig, PolicyExpression, ScriptTypeName,
    };

    fn sample_config() -> PolicyConfig {
        let json = include_str!("../../../tests/vectors/policy_abc_testnet.json");
        serde_json::from_str(json).expect("fixture")
    }

    #[test]
    fn ledger_policy_normalizes_multipath_placeholders() {
        let config = sample_config();
        let descriptor = include_str!("../../../tests/vectors/policy_abc_testnet_descriptor.txt");
        let bip = descriptor_to_bip388("ABC Vault", &config, descriptor).unwrap();
        let ledger =
            to_ledger_wallet_policy(&bip, &config, NetworkName::Testnet).unwrap();
        assert!(ledger.policy.contains("@0/**"));
        assert!(ledger.policy.contains("@1/**"));
        assert!(!ledger.policy.contains("<0;1>"));
        assert!(ledger.keys[0].starts_with("[78412e3a"));
        assert!(ledger.keys[0].contains("tpub") || ledger.keys[0].contains("xpub"));
    }

    #[test]
    fn abc_ledger_policy_uses_binary_taptree() {
        let config = sample_config();
        let descriptor = include_str!("../../../tests/vectors/policy_abc_testnet_descriptor.txt");
        let bip = descriptor_to_bip388("ABC Vault", &config, descriptor).unwrap();
        let ledger =
            to_ledger_wallet_policy(&bip, &config, NetworkName::Testnet).unwrap();
        // Depth encoding chains all leaves: `)),and_v(...),and_v(...),0}}`
        assert!(
            !ledger.policy.contains(")),and_v(v:pk(@0/**),pk(@1/**)),and_v"),
            "expected binary taptree, got depth encoding: {}",
            ledger.policy
        );
        // Binary encoding nests subtrees: `...)),{and_v(...),0}}`
        assert!(
            ledger.policy.contains("))},{and_v"),
            "expected binary taptree nesting: {}",
            ledger.policy
        );
    }

    #[test]
    fn abc_testnet_bip388_fits_ledger_template_limit() {
        let config = sample_config();
        let descriptor = include_str!("../../../tests/vectors/policy_abc_testnet_descriptor.txt");
        let bip = descriptor_to_bip388("ABC Vault", &config, descriptor).unwrap();
        let ledger =
            to_ledger_wallet_policy(&bip, &config, NetworkName::Testnet).unwrap();
        assert!(
            ledger.policy.len() <= LEDGER_MAX_POLICY_TEMPLATE_LEN,
            "policy len {}: {}",
            ledger.policy.len(),
            ledger.policy
        );
    }

    #[test]
    fn bip388_policy_fingerprint_is_stable() {
        let bip = Bip388Policy {
            name: "ABC".into(),
            policy: "tr(@0)".into(),
            keys: vec!["[fp/86'/1'/0']xpub".into()],
        };
        let a = bip388_policy_fingerprint(&bip);
        let b = bip388_policy_fingerprint(&bip);
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn bip388_replaces_keys_with_placeholders() {
        let config = sample_config();
        let descriptor = include_str!("../../../tests/vectors/policy_abc_testnet_descriptor.txt");
        let bip = descriptor_to_bip388("ABC Vault", &config, descriptor).unwrap();
        assert!(bip.policy.contains("@0"), "{bip:?}");
        assert!(bip.policy.contains("@1"), "{bip:?}");
        assert!(bip.policy.contains("@2"), "{bip:?}");
        assert!(!bip.policy.contains("xpub"), "xpubs should move to key vector");
        assert_eq!(bip.keys.len(), 3);
        assert!(bip.keys[0].contains("78412e3a"));
        assert!(bip.policy.starts_with("tr("));
    }

    #[test]
    fn single_key_display_descriptor_format() {
        let config = sample_config();
        let key = &config.keys[0];
        let desc = single_key_display_descriptor(key);
        assert!(desc.starts_with("tr(["), "{desc}");
        assert!(desc.contains("/<0;1>/*)"), "{desc}");
        assert!(!desc.contains("and_v"), "{desc}");
    }

    #[test]
    fn ledger_registers_on_first_psbt_for_abc_descriptor() {
        let descriptor = include_str!("../../../tests/vectors/policy_abc_testnet_descriptor.txt");
        assert!(ledger_registers_on_first_psbt(descriptor.trim()));
        assert!(!ledger_registers_on_first_psbt(
            "wpkh([deadbeef/84'/0'/0']xpub6DeadBeefDeadBeefDeadBeefDeadBeefDeadBeefDeadBeefDeadBeefDeadBee/<0;1>/*)"
        ));
    }

    #[test]
    fn registration_package_has_vendors() {
        let config = sample_config();
        let descriptor = include_str!("../../../tests/vectors/policy_abc_testnet_descriptor.txt");
        let pkg = build_registration_package("ABC", &config, descriptor).unwrap();
        assert_eq!(pkg.vendors.len(), 3);
        assert!(pkg.coldcard_sd_text.contains("tr("));
        assert_eq!(hwi_chain(NetworkName::Testnet), "test");
    }

    #[test]
    fn primary_hints_list_abc_keys() {
        let config = sample_config();
        let hints = primary_cosigner_hints(&config);
        assert!(hints.iter().any(|h| h.starts_with('A')));
        assert!(hints.iter().any(|h| h.starts_with('B')));
    }

    #[test]
    fn rejects_unmappable_descriptor() {
        let config = PolicyConfig {
            version: 1,
            network: NetworkName::Testnet,
            script_type: ScriptTypeName::Taproot,
            keys: vec![KeyConfig {
                id: "A".into(),
                role: KeyRole::Investor,
                xpub: "xpub6DeadBeefDeadBeefDeadBeefDeadBeefDeadBeefDeadBeefDeadBeefDeadBee".into(),
                fingerprint: "deadbeef".into(),
                origin_path: Some("86'/1'/0'".into()),
            }],
            policy: PolicyExpression {
                primary: "A".into(),
                fallback: Some(FallbackPolicy {
                    after: "1y".into(),
                    allow: "A".into(),
                }),
                fallbacks: vec![],
            },
        };
        let err = descriptor_to_bip388("x", &config, "tr(01,#abcdefgh)").unwrap_err();
        assert!(matches!(err, SignError::Unsupported(_)));
    }
}
