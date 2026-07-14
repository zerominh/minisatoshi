//! Tauri IPC commands for Minisatoshi.

use blockchain::{
    default_server_presets, export_watch_only_wallet, BackendKind, EsploraBackend, Utxo,
};
use descriptor_engine::compile_descriptor_from_config;
use policy_engine::{NetworkName, PolicyConfig};
use psbt_engine::{
    create_psbt as build_psbt, export_psbt, CreatePsbtOptions, ExportFormat, FeeRate,
    PsbtRecipient, SpendingUtxo,
};
use tauri::State;
use vault::VaultService;

use crate::dto::{
    AddressDto, BalanceDto, CompileVaultResponse, CreatePsbtRequest, CreateVaultRequest,
    CreateWalletRequest, PsbtDto, ServerPresetDto, SparrowExportDto, SyncResultDto, VaultDto,
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
