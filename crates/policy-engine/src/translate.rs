use std::str::FromStr;

use crate::config::{KeyConfig, PolicyConfig};
use crate::error::PolicyError;
use miniscript::bitcoin::hashes::{hash160, ripemd160, sha256};
use miniscript::hash256;
use miniscript::policy::Concrete;
use miniscript::DescriptorPublicKey;
use miniscript::Translator;

pub struct KeyTranslator<'a> {
    pub config: &'a PolicyConfig,
}

pub fn translate_policy_keys(
    policy: &Concrete<String>,
    config: &PolicyConfig,
) -> Result<Concrete<DescriptorPublicKey>, PolicyError> {
    policy
        .translate_pk(&mut KeyTranslator { config })
        .map_err(|e| PolicyError::MiniscriptCompile(e.to_string()))
}

pub fn descriptor_key_expression(key: &KeyConfig) -> Result<String, PolicyError> {
    let base = match key.origin_path.as_deref().filter(|path| !path.is_empty()) {
        Some(path) => {
            let path = normalize_origin_path(path);
            if path.is_empty() {
                key.xpub.clone()
            } else {
                format!("[{}/{}]{}", key.fingerprint, path, key.xpub)
            }
        }
        None => key.xpub.clone(),
    };

    Ok(format!("{base}/<0;1>/*"))
}

/// Descriptor origins use `[fingerprint/86'/0'/0']` — strip BIP32 `m/` / `M/` prefix if present.
fn normalize_origin_path(path: &str) -> String {
    let trimmed = path.trim().trim_start_matches('/').to_string();
    let without_m = if let Some(rest) = trimmed.strip_prefix("m/") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("M/") {
        rest
    } else if trimmed == "m" || trimmed == "M" {
        ""
    } else {
        trimmed.as_str()
    };
    without_m.trim_start_matches('/').to_string()
}

impl Translator<String> for KeyTranslator<'_> {
    type TargetPk = DescriptorPublicKey;
    type Error = PolicyError;

    fn pk(&mut self, pk: &String) -> Result<DescriptorPublicKey, PolicyError> {
        if let Ok(desc_pk) = pk.parse::<DescriptorPublicKey>() {
            return Ok(desc_pk);
        }

        let key = self
            .config
            .keys
            .iter()
            .find(|k| &k.id == pk)
            .ok_or_else(|| PolicyError::UnknownKey(pk.clone()))?;

        descriptor_key_expression(key)?
            .parse()
            .map_err(|e| PolicyError::MiniscriptCompile(format!("invalid key '{pk}': {e}")))
    }

    fn sha256(&mut self, value: &String) -> Result<sha256::Hash, PolicyError> {
        sha256::Hash::from_str(value).map_err(|e| PolicyError::MiniscriptCompile(e.to_string()))
    }

    fn hash256(&mut self, value: &String) -> Result<hash256::Hash, PolicyError> {
        hash256::Hash::from_str(value).map_err(|e| PolicyError::MiniscriptCompile(e.to_string()))
    }

    fn ripemd160(&mut self, value: &String) -> Result<ripemd160::Hash, PolicyError> {
        ripemd160::Hash::from_str(value).map_err(|e| PolicyError::MiniscriptCompile(e.to_string()))
    }

    fn hash160(&mut self, value: &String) -> Result<hash160::Hash, PolicyError> {
        hash160::Hash::from_str(value).map_err(|e| PolicyError::MiniscriptCompile(e.to_string()))
    }
}
