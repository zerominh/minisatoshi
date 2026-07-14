//! Helpers for PSBT integration tests with regtest tprvs.

use miniscript::descriptor::DescriptorSecretKey;
use policy_engine::{KeyConfig, KeyRole};

pub const TEST_TPRV_A: &str =
    "tprv8ZgxMBicQKsPeZjVFDhZR5wjfCvFNev9qKGPDPC77p5cAEgEMUCR8Cecaf8pYfY7NTz8QcjVnP8uR8NedPz8o7iG7qWgnFMyQy9BAhMVZgb/86'/0'/0'/0/*";
pub const TEST_TPRV_B: &str =
    "tprv8ZgxMBicQKsPeHy2kPPVzYpbUqwTVBjSthMJUcGyqUiXk8eZTQ6xrJKEmdX8NYJKLLGCHGjuByqz2ahJXp52E8zCUV7njziJzwN7V7zfrKZ/86'/0'/0'/0/*";

pub fn key_config_from_tprv(id: &str, role: KeyRole, tprv: &str) -> KeyConfig {
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let sk = DescriptorSecretKey::from_str(tprv).expect("valid tprv");
    let pk = sk.to_public(&secp).expect("tprv to public");
    let rendered = pk.to_string();

    let (meta, remainder) = rendered
        .split_once(']')
        .unwrap_or((&rendered, rendered.as_str()));
    let fingerprint = meta
        .trim_start_matches('[')
        .split('/')
        .next()
        .unwrap_or("00000000")
        .to_string();
    let origin_path = meta.split_once('/').map(|(_, path)| path.to_string());
    let xpub = remainder
        .split('/')
        .next()
        .expect("xpub segment")
        .to_string();

    KeyConfig {
        id: id.into(),
        role,
        xpub,
        fingerprint,
        origin_path,
    }
}

use std::str::FromStr;
