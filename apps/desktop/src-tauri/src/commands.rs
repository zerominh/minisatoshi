//! Tauri IPC commands for Minisatoshi.

use std::str::FromStr;

use blockchain::{
    default_server_presets, export_watch_only_wallet, BackendKind, BlockchainBackend,
    EsploraBackend, Utxo,
};
use descriptor_engine::compile_descriptor_from_config;
use miniscript::descriptor::DescriptorSecretKey;
use policy_engine::{NetworkName, PolicyConfig};
use psbt_engine::{
    broadcast_psbt, combine_psbt, create_psbt as build_psbt, export_psbt, finalize_psbt,
    import_psbt_base64, sign_psbt, transaction_hex, CreatePsbtOptions, ExportFormat, FeeRate,
    PsbtRecipient, SoftwareSigner, SpendingUtxo,
};
use tauri::State;
use vault::VaultService;

use crate::dto::{
    AddressDto, BalanceDto, BroadcastTxRequest, CombinePsbtRequest, CompileVaultResponse,
    CreatePsbtRequest, CreateVaultRequest, CreateWalletRequest, FinalizedTxDto, PsbtDto,
    ServerPresetDto, SignPsbtRequest, SignedPsbtDto, SparrowExportDto, SyncResultDto, VaultDto,
    VaultSummaryDto, WalletDto, WalletSummaryDto,
};
use crate::error::user_facing_error;
use crate::state::AppState;

#[tauri::command]
pub fn compile_vault_descriptor(config: PolicyConfig) -> Result<CompileVaultResponse, String> {
    let policy_string =
        policy_engine::compile_abstract_policy_string(&config).map_err(user_facing_error)?;
    let descriptor = compile_descriptor_from_config(&config).map_err(user_facing_error)?;

    Ok(CompileVaultResponse {
        descriptor,
        policy_string,
    })
}

#[tauri::command]
pub fn create_wallet(
    state: State<'_, AppState>,
    request: CreateWalletRequest,
) -> Result<WalletDto, String> {
    state.with_store(|store| {
        store
            .create_wallet(&request.name, request.network)
            .map(WalletDto::from)
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn list_wallets(state: State<'_, AppState>) -> Result<Vec<WalletSummaryDto>, String> {
    state.with_store(|store| {
        store
            .list_wallets()
            .map(|wallets| wallets.into_iter().map(WalletSummaryDto::from).collect())
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn create_vault(
    state: State<'_, AppState>,
    request: CreateVaultRequest,
) -> Result<VaultDto, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        service
            .create_vault_with_receive_address(&request.wallet_id, &request.name, request.policy)
            .map(|result| VaultDto::from(result.vault))
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn list_vaults(
    state: State<'_, AppState>,
    wallet_id: String,
) -> Result<Vec<VaultSummaryDto>, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        service
            .list_vaults(&wallet_id)
            .map(|vaults| vaults.into_iter().map(VaultSummaryDto::from).collect())
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn get_vault(state: State<'_, AppState>, vault_id: String) -> Result<VaultDto, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        service
            .get_vault(&vault_id)
            .map(VaultDto::from)
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn new_receive_address(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<AddressDto, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        service
            .new_receive_address(&vault_id)
            .map(AddressDto::from)
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn get_balance(
    state: State<'_, AppState>,
    vault_id: String,
    esplora_url: Option<String>,
) -> Result<BalanceDto, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        let vault = service.get_vault(&vault_id).map_err(user_facing_error)?;
        let backend = match esplora_url {
            Some(url) => EsploraBackend::new(url).map_err(user_facing_error)?,
            None => EsploraBackend::for_network(vault.policy.network).map_err(user_facing_error)?,
        };
        service
            .vault_balance(&vault_id, &backend)
            .map(BalanceDto::from)
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn sync_vault(
    state: State<'_, AppState>,
    vault_id: String,
    esplora_url: Option<String>,
) -> Result<SyncResultDto, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        let vault = service.get_vault(&vault_id).map_err(user_facing_error)?;
        let backend = match esplora_url {
            Some(url) => EsploraBackend::new(url).map_err(user_facing_error)?,
            None => EsploraBackend::for_network(vault.policy.network).map_err(user_facing_error)?,
        };
        service
            .sync_vault(&vault_id, &backend, &|_| {})
            .map(SyncResultDto::from)
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn create_psbt(
    state: State<'_, AppState>,
    request: CreatePsbtRequest,
) -> Result<PsbtDto, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        let vault = service
            .get_vault(&request.vault_id)
            .map_err(user_facing_error)?;

        let recipients: Vec<PsbtRecipient> = request
            .recipients
            .into_iter()
            .map(|r| PsbtRecipient {
                address: r.address,
                amount_sats: r.amount_sats,
            })
            .collect();

        let utxos: Vec<SpendingUtxo> = request
            .utxos
            .into_iter()
            .map(|u| {
                SpendingUtxo::new(
                    Utxo {
                        txid: u.txid,
                        vout: u.vout,
                        value_sats: u.value_sats,
                        address: u.address,
                        confirmed: u.confirmed,
                        block_height: u.block_height,
                        derivation_index: u.derivation_index,
                        is_change: u.is_change,
                    },
                    u.derivation_index,
                    u.is_change,
                )
            })
            .collect();

        let psbt = build_psbt(
            &vault,
            &recipients,
            FeeRate::new(request.fee_rate_sat_per_vb),
            &utxos,
            CreatePsbtOptions {
                input_sequence: request.input_sequence,
                change_index: request.change_index,
            },
        )
        .map_err(user_facing_error)?;

        let base64 =
            String::from_utf8(export_psbt(&psbt, ExportFormat::Base64).map_err(user_facing_error)?)
                .map_err(user_facing_error)?;

        Ok(PsbtDto {
            base64,
            input_count: psbt.inputs.len(),
            output_count: psbt.outputs.len(),
        })
    })
}

#[tauri::command]
pub fn export_sparrow_wallet(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<SparrowExportDto, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        let vault = service.get_vault(&vault_id).map_err(user_facing_error)?;
        let exported = export_watch_only_wallet(&vault).map_err(user_facing_error)?;
        Ok(SparrowExportDto {
            name: exported.name,
            descriptor: exported.descriptor,
            network: exported.network,
            import_instructions: exported.import_instructions,
        })
    })
}

#[tauri::command]
pub fn list_server_presets(network: NetworkName) -> Result<Vec<ServerPresetDto>, String> {
    Ok(default_server_presets(network)
        .into_iter()
        .map(|preset| ServerPresetDto {
            label: preset.label,
            backend: match preset.backend {
                BackendKind::Esplora => "esplora".into(),
                BackendKind::Electrum => "electrum".into(),
                BackendKind::Core => "core".into(),
            },
            url: preset.url,
            network: preset.network,
        })
        .collect())
}

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Native Save As dialog, then write UTF-8 text to the chosen path.
/// Returns `None` if the user cancelled.
#[tauri::command]
pub fn save_text_file(
    app: tauri::AppHandle,
    default_filename: String,
    contents: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let chosen = app
        .dialog()
        .file()
        .set_file_name(&default_filename)
        .add_filter("Descriptor / Text", &["txt"])
        .blocking_save_file();

    let Some(file_path) = chosen else {
        return Ok(None);
    };

    let path = file_path
        .into_path()
        .map_err(|e| format!("invalid save path: {e}"))?;
    std::fs::write(&path, contents).map_err(|e| format!("failed to write file: {e}"))?;
    Ok(Some(path.display().to_string()))
}

fn parse_psbt_b64(base64: &str) -> Result<psbt_engine::Psbt, String> {
    import_psbt_base64(base64.trim().as_bytes()).map_err(user_facing_error)
}

fn encode_psbt(psbt: &psbt_engine::Psbt) -> Result<String, String> {
    let bytes = export_psbt(psbt, ExportFormat::Base64).map_err(user_facing_error)?;
    String::from_utf8(bytes).map_err(user_facing_error)
}

pub(crate) fn assert_hot_key_allowed(
    network: NetworkName,
    allow_mainnet_hot_keys: bool,
) -> Result<(), String> {
    if network == NetworkName::Mainnet && !allow_mainnet_hot_keys {
        return Err(
            "software / hot-key signing is disabled on mainnet — enable allowMainnetHotKeys only if you accept the risk"
                .into(),
        );
    }
    Ok(())
}

/// Sign a PSBT with a descriptor secret (`tprv…` / `xprv…`, optionally with origin + path).
#[tauri::command]
pub fn sign_psbt_software(request: SignPsbtRequest) -> Result<SignedPsbtDto, String> {
    assert_hot_key_allowed(request.network, request.allow_mainnet_hot_keys)?;

    let mut psbt = parse_psbt_b64(&request.psbt_base64)?;
    let secret = DescriptorSecretKey::from_str(request.secret_key.trim())
        .map_err(|e| user_facing_error(format!("invalid secret key: {e}")))?;

    let progress = sign_psbt(&mut psbt, &SoftwareSigner::from_secret(secret))
        .map_err(user_facing_error)?;
    let base64 = encode_psbt(&psbt)?;

    Ok(SignedPsbtDto {
        base64,
        input_count: psbt.inputs.len(),
        output_count: psbt.outputs.len(),
        signed_inputs: progress.signed_inputs,
        total_inputs: progress.total_inputs,
    })
}

#[tauri::command]
pub fn combine_psbts(request: CombinePsbtRequest) -> Result<PsbtDto, String> {
    if request.parts.len() < 2 {
        return Err("combine requires at least two PSBT parts".into());
    }
    let mut iter = request.parts.into_iter();
    let first = parse_psbt_b64(&iter.next().unwrap())?;
    let mut combined = first;
    for part in iter {
        let other = parse_psbt_b64(&part)?;
        combined = combine_psbt(combined, other).map_err(user_facing_error)?;
    }
    let base64 = encode_psbt(&combined)?;
    Ok(PsbtDto {
        base64,
        input_count: combined.inputs.len(),
        output_count: combined.outputs.len(),
    })
}

#[tauri::command]
pub fn finalize_psbt_cmd(psbt_base64: String) -> Result<FinalizedTxDto, String> {
    let mut psbt = parse_psbt_b64(&psbt_base64)?;
    let tx = finalize_psbt(&mut psbt).map_err(user_facing_error)?;
    Ok(FinalizedTxDto {
        hex: transaction_hex(&tx),
        txid: tx.compute_txid().to_string(),
        fully_signed: true,
    })
}

/// Finalize (if needed) and broadcast via Esplora for the vault's network.
#[tauri::command]
pub fn broadcast_psbt_cmd(
    state: State<'_, AppState>,
    request: BroadcastTxRequest,
) -> Result<String, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        let vault = service
            .get_vault(&request.vault_id)
            .map_err(user_facing_error)?;
        let backend = match &request.esplora_url {
            Some(url) => EsploraBackend::new(url.clone()).map_err(user_facing_error)?,
            None => EsploraBackend::for_network(vault.policy.network).map_err(user_facing_error)?,
        };

        if let Some(hex) = request.tx_hex.as_ref().filter(|h| !h.trim().is_empty()) {
            return backend.broadcast(hex.trim()).map_err(user_facing_error);
        }

        let psbt_b64 = request
            .psbt_base64
            .as_ref()
            .ok_or_else(|| "provide psbtBase64 or txHex".to_string())?;
        let psbt = parse_psbt_b64(psbt_b64)?;
        broadcast_psbt(&psbt, &backend).map_err(user_facing_error)
    })
}

#[cfg(test)]
mod tests {
    use super::assert_hot_key_allowed;
    use policy_engine::NetworkName;

    #[test]
    fn rejects_mainnet_hot_keys_by_default() {
        let err = assert_hot_key_allowed(NetworkName::Mainnet, false).unwrap_err();
        assert!(err.contains("mainnet"));
    }

    #[test]
    fn allows_testnet_hot_keys() {
        assert_hot_key_allowed(NetworkName::Testnet, false).unwrap();
        assert_hot_key_allowed(NetworkName::Mainnet, true).unwrap();
    }
}
