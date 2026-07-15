use policy_engine::NetworkName;
use wallet_core::Wallet;

use crate::electrum::default_electrum_url;
use crate::error::ChainError;
use crate::esplora::default_esplora_url;
use crate::types::{BackendKind, ServerPreset};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparrowWalletExport {
    pub name: String,
    pub descriptor: String,
    pub network: NetworkName,
    pub import_instructions: String,
}

/// Build a Sparrow-compatible watch-only export package from a wallet.
pub fn export_watch_only_wallet(wallet: &Wallet) -> Result<SparrowWalletExport, ChainError> {
    if wallet.descriptor.trim().is_empty() {
        return Err(ChainError::Parse("wallet descriptor is empty".into()));
    }
    if !wallet.descriptor.contains('#') {
        return Err(ChainError::Parse(
            "descriptor must include checksum for Sparrow import".into(),
        ));
    }

    Ok(SparrowWalletExport {
        name: wallet.name.clone(),
        descriptor: wallet.descriptor.clone(),
        network: wallet.policy.network,
        import_instructions: sparrow_import_instructions(wallet.policy.network),
    })
}

/// Recommended server presets that Sparrow users commonly select.
pub fn default_server_presets(network: NetworkName) -> Vec<ServerPreset> {
    if network == NetworkName::Testnet4 {
        return vec![
            ServerPreset {
                label: "Mempool Esplora (testnet4)".into(),
                backend: BackendKind::Esplora,
                url: default_esplora_url(network).into(),
                network,
            },
            ServerPreset {
                label: "Mempool Electrum SSL (Sparrow native)".into(),
                backend: BackendKind::Electrum,
                url: "ssl://mempool.space:40002".into(),
                network,
            },
            ServerPreset {
                label: "Local Electrum / electrs".into(),
                backend: BackendKind::Electrum,
                url: default_electrum_url(network).into(),
                network,
            },
            ServerPreset {
                label: "Bitcoin Core RPC (local testnet4)".into(),
                backend: BackendKind::Core,
                url: "http://127.0.0.1:48332".into(),
                network,
            },
        ];
    }

    let mut presets = vec![
        ServerPreset {
            label: "Blockstream Esplora".into(),
            backend: BackendKind::Esplora,
            url: default_esplora_url(network).into(),
            network,
        },
        ServerPreset {
            label: "Blockstream Electrum (HTTPS bridge)".into(),
            backend: BackendKind::Electrum,
            url: default_electrum_url(network).into(),
            network,
        },
    ];

    match network {
        NetworkName::Mainnet => {
            presets.push(ServerPreset {
                label: "Blockstream Electrum SSL (Sparrow native)".into(),
                backend: BackendKind::Electrum,
                url: "ssl://electrum.blockstream.info:50002".into(),
                network,
            });
            presets.push(ServerPreset {
                label: "Bitcoin Core RPC (local)".into(),
                backend: BackendKind::Core,
                url: "http://127.0.0.1:8332".into(),
                network,
            });
        }
        NetworkName::Testnet => {
            presets.push(ServerPreset {
                label: "Blockstream Electrum SSL (Sparrow native)".into(),
                backend: BackendKind::Electrum,
                url: "ssl://electrum.blockstream.info:60002".into(),
                network,
            });
            presets.push(ServerPreset {
                label: "Bitcoin Core RPC (local testnet3)".into(),
                backend: BackendKind::Core,
                url: "http://127.0.0.1:18332".into(),
                network,
            });
        }
        NetworkName::Signet => {
            presets.push(ServerPreset {
                label: "Blockstream Electrum SSL (Sparrow native)".into(),
                backend: BackendKind::Electrum,
                url: "ssl://electrum.blockstream.info:60602".into(),
                network,
            });
            presets.push(ServerPreset {
                label: "Bitcoin Core RPC (local)".into(),
                backend: BackendKind::Core,
                url: "http://127.0.0.1:38332".into(),
                network,
            });
        }
        NetworkName::Regtest => {
            presets.push(ServerPreset {
                label: "Bitcoin Core RPC (local)".into(),
                backend: BackendKind::Core,
                url: "http://127.0.0.1:18443".into(),
                network,
            });
        }
        NetworkName::Testnet4 => {}
    }

    presets
}

fn sparrow_import_instructions(network: NetworkName) -> String {
    format!(
        "Sparrow: fund only — copy a Minisatoshi receive address and send (network {:?}). \
         Sparrow does not import or sign arbitrary Miniscript / Taproot script-path vaults. \
         Watch or sign with Minisatoshi (HW/software), Bitcoin Core, or Nunchuk. \
         The descriptor below is a backup for those wallets — see docs/interop.md.",
        network
    )
}

#[cfg(test)]
mod tests {
    use policy_engine::{
        abc_preset, test_vectors::TEST_FP, test_vectors::TEST_XPUB_A, test_vectors::TEST_XPUB_B,
        test_vectors::TEST_XPUB_C, KeyConfig, KeyRole, NetworkName,
    };
    use wallet_core::WalletStore;

    use super::*;

    fn sample_keys() -> [KeyConfig; 3] {
        [
            KeyConfig {
                id: "A".into(),
                role: KeyRole::Investor,
                xpub: TEST_XPUB_A.into(),
                fingerprint: "78412e3a".into(),
                origin_path: Some("44'/0'/0'".into()),
            },
            KeyConfig {
                id: "B".into(),
                role: KeyRole::Manager,
                xpub: TEST_XPUB_B.into(),
                fingerprint: TEST_FP.into(),
                origin_path: Some("86'/0'/0'".into()),
            },
            KeyConfig {
                id: "C".into(),
                role: KeyRole::Recovery,
                xpub: TEST_XPUB_C.into(),
                fingerprint: TEST_FP.into(),
                origin_path: Some("84'/0'/0'".into()),
            },
        ]
    }

    #[test]
    fn export_watch_only_wallet_includes_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let store = WalletStore::open(dir.path().join("wallet.db")).unwrap();
        let workspace = store
            .create_workspace("Sparrow", NetworkName::Testnet)
            .unwrap();
        let policy = abc_preset(
            sample_keys()[0].clone(),
            sample_keys()[1].clone(),
            sample_keys()[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let wallet = store
            .create_wallet(&workspace.id, "ABC", policy)
            .unwrap();

        let exported = export_watch_only_wallet(&wallet).unwrap();
        assert!(exported.descriptor.starts_with("tr("));
        assert!(exported.descriptor.contains('#'));
        assert_eq!(exported.network, NetworkName::Testnet);
    }

    #[test]
    fn default_server_presets_for_testnet() {
        let presets = default_server_presets(NetworkName::Testnet);
        assert!(presets
            .iter()
            .any(|preset| preset.backend == BackendKind::Esplora));
        assert!(presets.iter().any(|preset| preset.url.contains("testnet")));
        assert!(presets.iter().any(|preset| preset.url.contains("60002")));
    }

    #[test]
    fn default_server_presets_for_testnet4() {
        let presets = default_server_presets(NetworkName::Testnet4);
        assert!(presets.iter().any(|preset| preset.url.contains("testnet4")));
        assert!(presets.iter().any(|preset| preset.url.contains("48332")));
    }
}
