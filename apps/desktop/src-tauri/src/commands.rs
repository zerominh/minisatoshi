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
    analyze_signing_status, broadcast_psbt, combine_psbt, create_psbt as build_psbt, export_psbt,
    finalize_psbt, hw_sign_made_progress, import_psbt_base64, populate_global_xpubs, sign_psbt,
    signature_snapshot, transaction_hex, CreatePsbtOptions, ExportFormat, FeeRate, PsbtRecipient,
    SoftwareSigner, SpendingUtxo, SigningStatus,
};
use tauri::State;
use vault::WalletService;

use crate::dto::{
    AddressDto, AnalyzePsbtRequest, BalanceDto, BroadcastTxRequest, BsmsExportDto,
    CombinePsbtRequest, CompileWalletResponse, CreateHotKeystoreRequest, CreatePsbtRequest,
    CreateWalletRequest, CreateWorkspaceRequest, FinalizedTxDto, HotKeystoreStatusDto,
    HotWalletSummaryDto,     HwDeviceDto, HwGetXpubRequest, HwRegisterRequest, HwRegisterResultDto,
    HwSignPsbtRequest, HwStatusDto, HwXpubDto, ImportDescriptorRequest, ImportHotWalletRequestDto,
    ImportHotWalletResultDto, ImportWalletBackupRequest, LedgerRegistrationStatusDto,
    LedgerRuntimeStatusDto, OpenTextFileDto, PsbtDto, ServerPresetDto,
    SignPsbtHotRequest, SignPsbtRequest, SignedPsbtDto, SparrowExportDto, SyncResultDto,
    TxOutputDto, UnlockHotKeystoreRequest, WalletBackupDto, WalletDto, WalletSummaryDto,
    WorkspaceDto, WorkspaceSummaryDto,
};
use crate::error::user_facing_error;
use crate::state::AppState;
use signing_devices::{
    build_registration_package, descriptor_to_bip388, ensure_hwi, evaluate_ledger_readiness,
    expected_bitcoin_app_name, find_hwi, find_key_by_fingerprint, find_ledger_runtime, hwi_chain,
    is_registerpolicy_unavailable, is_taproot_script_path_miniscript, ledger_probe_device,
    ledger_register_wallet, ledger_registers_on_first_psbt, ledger_sign_psbt, load_registration,
    parse_derivation_path, primary_cosigner_hints, registration_stale_reason, resolve_ledger_cli,
    runtime_source_label, save_registration, single_key_display_descriptor,
    to_ledger_wallet_policy, DeviceInfo, DeviceType, HwiClient, HwiSource, RegistrationPackage,
    PINNED_HWI_VERSION, PINNED_LEDGER_BITCOIN_VERSION,
};

#[tauri::command]
pub fn compile_wallet_descriptor(config: PolicyConfig) -> Result<CompileWalletResponse, String> {
    let policy_string =
        policy_engine::compile_abstract_policy_string(&config).map_err(user_facing_error)?;
    let descriptor = compile_descriptor_from_config(&config).map_err(user_facing_error)?;

    Ok(CompileWalletResponse {
        descriptor,
        policy_string,
    })
}

#[tauri::command]
pub fn create_workspace(
    state: State<'_, AppState>,
    request: CreateWorkspaceRequest,
) -> Result<WorkspaceDto, String> {
    state.with_store(|store| {
        store
            .create_workspace(&request.name, request.network)
            .map(WorkspaceDto::from)
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn list_workspaces(state: State<'_, AppState>) -> Result<Vec<WorkspaceSummaryDto>, String> {
    state.with_store(|store| {
        store
            .list_workspaces()
            .map(|workspaces| workspaces.into_iter().map(WorkspaceSummaryDto::from).collect())
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn delete_workspace(state: State<'_, AppState>, workspace_id: String) -> Result<(), String> {
    state.with_store(|store| {
        store
            .delete_workspace(&workspace_id)
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn delete_wallet(state: State<'_, AppState>, wallet_id: String) -> Result<(), String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        service.delete_wallet(&wallet_id).map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn rename_workspace(
    state: State<'_, AppState>,
    workspace_id: String,
    name: String,
) -> Result<WorkspaceDto, String> {
    state.with_store(|store| {
        store
            .rename_workspace(&workspace_id, &name)
            .map(WorkspaceDto::from)
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn rename_wallet(
    state: State<'_, AppState>,
    wallet_id: String,
    name: String,
) -> Result<WalletDto, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        service
            .rename_wallet(&wallet_id, &name)
            .map(WalletDto::from)
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn create_wallet(
    state: State<'_, AppState>,
    request: CreateWalletRequest,
) -> Result<WalletDto, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        service
            .create_wallet_with_receive_address(&request.workspace_id, &request.name, request.policy)
            .map(|result| WalletDto::from(result.wallet))
            .map_err(user_facing_error)
    })
}

/// Import a checksummed descriptor (optional policy JSON) into a workspace.
#[tauri::command]
pub fn import_descriptor(
    state: State<'_, AppState>,
    request: ImportDescriptorRequest,
) -> Result<WalletDto, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        service
            .import_descriptor(
                &request.workspace_id,
                &request.name,
                &request.descriptor,
                request.policy,
            )
            .map(WalletDto::from)
            .map_err(user_facing_error)
    })
}

/// Import watch-only: `minisatoshi-wallet-v1.json`, bare descriptor, BSMS, or Liana-ish JSON.
#[tauri::command]
pub fn import_wallet_backup(
    state: State<'_, AppState>,
    request: ImportWalletBackupRequest,
) -> Result<WalletDto, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        service
            .import_watch_only_payload(
                &request.workspace_id,
                &request.payload,
                request.name.as_deref(),
            )
            .map(WalletDto::from)
            .map_err(user_facing_error)
    })
}

/// Export portable wallet backup (JSON + descriptor text).
#[tauri::command]
pub fn export_wallet_backup(
    state: State<'_, AppState>,
    wallet_id: String,
) -> Result<WalletBackupDto, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        let backup = service
            .export_wallet_backup(&wallet_id)
            .map_err(user_facing_error)?;
        let json = backup.to_json_pretty().map_err(user_facing_error)?;
        let descriptor_txt = backup.descriptor_txt();
        Ok(WalletBackupDto {
            format_version: backup.format_version,
            name: backup.name,
            network: backup.network,
            descriptor: backup.descriptor,
            script_type: backup.script_type,
            policy: backup.policy,
            created_at: backup.created_at,
            json,
            descriptor_txt,
        })
    })
}

/// Export BIP-129-ish BSMS descriptor record (watch-only share).
#[tauri::command]
pub fn export_bsms(
    state: State<'_, AppState>,
    wallet_id: String,
) -> Result<BsmsExportDto, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        let text = service.export_bsms(&wallet_id).map_err(user_facing_error)?;
        let first_address = text
            .lines()
            .nth(3)
            .unwrap_or("")
            .trim()
            .to_string();
        Ok(BsmsExportDto { text, first_address })
    })
}

#[tauri::command]
pub fn list_wallets(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<Vec<WalletSummaryDto>, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        service
            .list_wallets(&workspace_id)
            .map(|wallets| wallets.into_iter().map(WalletSummaryDto::from).collect())
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn get_wallet(state: State<'_, AppState>, wallet_id: String) -> Result<WalletDto, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        service
            .get_wallet(&wallet_id)
            .map(WalletDto::from)
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn new_receive_address(
    state: State<'_, AppState>,
    wallet_id: String,
) -> Result<AddressDto, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        service
            .new_receive_address(&wallet_id)
            .map(AddressDto::from)
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn list_addresses(
    state: State<'_, AppState>,
    wallet_id: String,
) -> Result<Vec<AddressDto>, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        service
            .list_addresses(&wallet_id)
            .map(|addrs| addrs.into_iter().map(AddressDto::from).collect())
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn get_balance(
    state: State<'_, AppState>,
    wallet_id: String,
    esplora_url: Option<String>,
) -> Result<BalanceDto, String> {
    // Do not hold the wallet-store mutex during Esplora HTTP (would freeze the UI).
    let query = state.with_store(|store| {
        let service = WalletService::new(store);
        service
            .descriptor_query(&wallet_id)
            .map_err(user_facing_error)
    })?;
    let backend = esplora_backend(esplora_url, query.network())?;
    backend
        .get_balance(&query)
        .map(BalanceDto::from)
        .map_err(user_facing_error)
}

#[tauri::command]
pub async fn sync_wallet(
    state: State<'_, AppState>,
    wallet_id: String,
    esplora_url: Option<String>,
) -> Result<SyncResultDto, String> {
    // Snapshot descriptor under a short lock, then sync on a worker thread.
    let query = state.with_store(|store| {
        let service = WalletService::new(store);
        service
            .descriptor_query(&wallet_id)
            .map_err(user_facing_error)
    })?;
    let network = query.network();

    tauri::async_runtime::spawn_blocking(move || {
        let backend = esplora_backend(esplora_url, network)?;
        backend
            .sync(&query, &|_| {})
            .map(SyncResultDto::from)
            .map_err(user_facing_error)
    })
    .await
    .map_err(|e| format!("sync task failed: {e}"))?
}

fn esplora_backend(
    esplora_url: Option<String>,
    network: NetworkName,
) -> Result<EsploraBackend, String> {
    match esplora_url {
        Some(url) => EsploraBackend::new(url).map_err(user_facing_error),
        None => EsploraBackend::for_network(network).map_err(user_facing_error),
    }
}

#[tauri::command]
pub fn create_psbt(
    state: State<'_, AppState>,
    request: CreatePsbtRequest,
) -> Result<PsbtDto, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        let wallet = service
            .get_wallet(&request.wallet_id)
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
            &wallet,
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
    wallet_id: String,
) -> Result<SparrowExportDto, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        let wallet = service.get_wallet(&wallet_id).map_err(user_facing_error)?;
        let exported = export_watch_only_wallet(&wallet).map_err(user_facing_error)?;
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
    filter_name: Option<String>,
    filter_extensions: Option<Vec<String>>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let mut builder = app.dialog().file().set_file_name(&default_filename);
    let name = filter_name.as_deref().unwrap_or("Text");
    let exts: Vec<&str> = filter_extensions
        .as_ref()
        .map(|v| v.iter().map(String::as_str).collect())
        .unwrap_or_else(|| vec!["txt"]);
    builder = builder.add_filter(name, &exts);

    let chosen = builder.blocking_save_file();

    let Some(file_path) = chosen else {
        return Ok(None);
    };

    let path = file_path
        .into_path()
        .map_err(|e| format!("invalid save path: {e}"))?;
    std::fs::write(&path, contents).map_err(|e| format!("failed to write file: {e}"))?;
    Ok(Some(path.display().to_string()))
}

/// Native Open dialog, then read UTF-8 text from the chosen path.
/// Returns `None` if the user cancelled.
#[tauri::command]
pub fn open_text_file(
    app: tauri::AppHandle,
    filter_name: Option<String>,
    filter_extensions: Option<Vec<String>>,
) -> Result<Option<OpenTextFileDto>, String> {
    use tauri_plugin_dialog::DialogExt;

    let mut builder = app.dialog().file();
    let name = filter_name.as_deref().unwrap_or("Text");
    let exts: Vec<&str> = filter_extensions
        .as_ref()
        .map(|v| v.iter().map(String::as_str).collect())
        .unwrap_or_else(|| vec!["txt"]);
    builder = builder.add_filter(name, &exts);

    let chosen = builder.blocking_pick_file();
    let Some(file_path) = chosen else {
        return Ok(None);
    };

    let path = file_path
        .into_path()
        .map_err(|e| format!("invalid open path: {e}"))?;
    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("failed to read file: {e}"))?;
    Ok(Some(OpenTextFileDto {
        path: path.display().to_string(),
        contents,
    }))
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

fn hwi_client(
    state: &AppState,
    hwi_path: Option<&str>,
    auto_install: bool,
    network: Option<NetworkName>,
) -> Result<HwiClient, String> {
    let preferred = hwi_path
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(std::path::PathBuf::from);
    let resolved = if auto_install {
        ensure_hwi(preferred.as_deref(), &state.data_dir).map_err(user_facing_error)?
    } else {
        find_hwi(preferred.as_deref(), &state.data_dir).ok_or_else(|| {
            format!(
                "HWI not found — click Install HWI (downloads v{PINNED_HWI_VERSION}) or set a binary path"
            )
        })?
    };
    let mut client = HwiClient::with_binary(resolved.path);
    if let Some(network) = network {
        client = client.with_chain(hwi_chain(network));
    }
    Ok(client)
}

fn device_to_dto(d: DeviceInfo) -> HwDeviceDto {
    HwDeviceDto {
        id: d.id,
        fingerprint: d.fingerprint,
        device_type: d.device_type.as_str().to_string(),
        model: d.model,
        path: d.path,
        needs_pin: d.needs_pin,
        needs_passphrase: d.needs_passphrase,
        error: d.error,
    }
}

fn count_signed_inputs(psbt: &psbt_engine::Psbt) -> usize {
    psbt.inputs
        .iter()
        .filter(|input| {
            !input.partial_sigs.is_empty()
                || input.tap_key_sig.is_some()
                || !input.tap_script_sigs.is_empty()
        })
        .count()
}

fn source_label(source: HwiSource) -> &'static str {
    match source {
        HwiSource::Preferred => "preferred",
        HwiSource::Env => "env",
        HwiSource::SystemPath => "system",
        HwiSource::Cached => "cached",
        HwiSource::Downloaded => "downloaded",
    }
}

/// Probe for HWI (PATH / env / Settings / app cache). Does not download.
#[tauri::command]
pub fn get_hwi_status(
    state: State<'_, AppState>,
    hwi_path: Option<String>,
) -> Result<HwStatusDto, String> {
    let preferred = hwi_path
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(std::path::PathBuf::from);
    match find_hwi(preferred.as_deref(), &state.data_dir) {
        Some(resolved) => Ok(HwStatusDto {
            available: true,
            path: Some(resolved.path.display().to_string()),
            version: Some(resolved.version),
            source: Some(source_label(resolved.source).to_string()),
            pinned_version: PINNED_HWI_VERSION.to_string(),
            message: None,
        }),
        None => Ok(HwStatusDto {
            available: false,
            path: None,
            version: None,
            source: None,
            pinned_version: PINNED_HWI_VERSION.to_string(),
            message: Some(format!(
                "HWI missing — app can download official v{PINNED_HWI_VERSION} (~50–90 MB)"
            )),
        }),
    }
}

/// Find HWI or download the pinned official release into app data.
#[tauri::command]
pub fn ensure_hwi_installed(
    state: State<'_, AppState>,
    hwi_path: Option<String>,
) -> Result<HwStatusDto, String> {
    let preferred = hwi_path
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(std::path::PathBuf::from);
    let resolved = ensure_hwi(preferred.as_deref(), &state.data_dir).map_err(user_facing_error)?;
    Ok(HwStatusDto {
        available: true,
        path: Some(resolved.path.display().to_string()),
        version: Some(resolved.version),
        source: Some(source_label(resolved.source).to_string()),
        pinned_version: PINNED_HWI_VERSION.to_string(),
        message: match resolved.source {
            HwiSource::Downloaded => Some(format!(
                "Downloaded HWI {PINNED_HWI_VERSION} from bitcoin-core/HWI (checksum verified)"
            )),
            _ => None,
        },
    })
}

/// Enumerate hardware wallets via HWI (`hwi enumerate`). Auto-installs HWI if missing.
#[tauri::command]
pub fn list_hw_devices(
    state: State<'_, AppState>,
    hwi_path: Option<String>,
    network: Option<NetworkName>,
) -> Result<Vec<HwDeviceDto>, String> {
    let client = hwi_client(&state, hwi_path.as_deref(), true, network)?;
    let devices = client.enumerate().map_err(user_facing_error)?;
    Ok(devices.into_iter().map(device_to_dto).collect())
}

/// Fetch an xpub from a connected device (`hwi getxpub`).
#[tauri::command]
pub fn hw_get_xpub(
    state: State<'_, AppState>,
    request: HwGetXpubRequest,
) -> Result<HwXpubDto, String> {
    let client = hwi_client(
        &state,
        request.hwi_path.as_deref(),
        true,
        request.network,
    )?;
    let path = parse_derivation_path(&request.derivation_path).map_err(user_facing_error)?;
    let xpub = client
        .get_xpub(request.fingerprint.trim(), &path)
        .map_err(user_facing_error)?;
    Ok(HwXpubDto {
        fingerprint: request.fingerprint.trim().to_string(),
        derivation_path: format!("m/{path}"),
        xpub,
    })
}

const LEDGER_NOT_REGISTERED_MSG: &str = "Register Ledger policy first: Wallet → Settings → Register on hardware → \
     Register Ledger policy. Install the Ledger signer in Settings if prompted.";

const LEDGER_STALE_MSG: &str = "Ledger registration is out of date — Wallet → Settings → Register Ledger policy again.";

/// Sign a PSBT on-device (`hwi signtx` or ledger-bitcoin for Ledger ABC).
#[tauri::command]
pub fn hw_sign_psbt(
    state: State<'_, AppState>,
    request: HwSignPsbtRequest,
) -> Result<SignedPsbtDto, String> {
    let (wallet_descriptor, wallet_network, wallet_policy, bip388) =
        if let Some(wallet_id) = request.wallet_id.as_deref().filter(|id| !id.is_empty()) {
            state.with_store(|store| {
                let service = WalletService::new(store);
                let wallet = service.get_wallet(wallet_id).map_err(user_facing_error)?;
                let bip388 =
                    descriptor_to_bip388(&wallet.name, &wallet.policy, &wallet.descriptor)
                        .map_err(user_facing_error)?;
                Ok((
                    Some(wallet.descriptor),
                    Some(wallet.policy.network),
                    Some(wallet.policy),
                    Some(bip388),
                ))
            })?
        } else {
            (None, None, None, None)
        };

    let mut psbt = parse_psbt_b64(&request.psbt_base64)?;
    let before = signature_snapshot(&psbt);

    if let Some(wallet_id) = request.wallet_id.as_deref().filter(|id| !id.is_empty()) {
        state.with_store(|store| {
            let service = WalletService::new(store);
            let wallet = service.get_wallet(wallet_id).map_err(user_facing_error)?;
            populate_global_xpubs(&mut psbt, &wallet.policy).map_err(user_facing_error)
        })?;
    }

    let psbt_for_hwi = encode_psbt(&psbt)?;
    let network = request.network.or(wallet_network);
    let client = hwi_client(
        &state,
        request.hwi_path.as_deref(),
        true,
        network,
    )?;
    let fingerprint = client
        .resolve_fingerprint(request.fingerprint.trim())
        .map_err(user_facing_error)?;

    let script_path = wallet_descriptor
        .as_deref()
        .is_some_and(is_taproot_script_path_miniscript);
    let ledger_device = client
        .find_device(&fingerprint)
        .map(|d| d.device_type == DeviceType::Ledger)
        .unwrap_or(false);

    let signed_b64 = if script_path && ledger_device {
        let wallet_id = request
            .wallet_id
            .as_deref()
            .filter(|id| !id.is_empty())
            .ok_or("wallet_id is required for Ledger ABC signing")?;
        let bip388 = bip388.ok_or("could not load wallet policy for Ledger signing")?;
        let net = network.ok_or("network is required for Ledger signing")?;
        let policy = wallet_policy.ok_or("could not load wallet policy for Ledger signing")?;
        let ledger_bip388 = to_ledger_wallet_policy(&bip388, &policy, net)
            .map_err(user_facing_error)?;
        let reg = load_registration(&state.data_dir, wallet_id, &fingerprint)
            .ok_or(LEDGER_NOT_REGISTERED_MSG)?;
        if let Some(reason) = registration_stale_reason(&reg, &ledger_bip388, net) {
            return Err(format!("{LEDGER_STALE_MSG} ({reason})"));
        }
        let ledger_cli =
            resolve_ledger_cli(&state.data_dir, net).map_err(user_facing_error)?;
        ledger_sign_psbt(&ledger_cli, &ledger_bip388, &reg.hmac, &psbt_for_hwi)
            .map_err(user_facing_error)?
    } else {
        client
            .sign_psbt(&fingerprint, &psbt_for_hwi)
            .map_err(user_facing_error)?
    };

    let psbt = parse_psbt_b64(&signed_b64)?;
    let after = signature_snapshot(&psbt);

    if !hw_sign_made_progress(&before, &after) {
        let hint = if script_path && ledger_device {
            LEDGER_NOT_REGISTERED_MSG
        } else if script_path {
            "Hardware wallet did not add any signature for this script-path PSBT. \
             For Ledger ABC wallets, register the policy in Settings first. \
             Coldcard: unlock device and approve on screen."
        } else {
            "Hardware wallet did not add any signature. Unlock the device, open the Bitcoin app, \
             approve on screen, and ensure the fingerprint matches a key in this PSBT. \
             Multi-key paths need each cosigner on the same PSBT (or Combine partial PSBTs)."
        };
        return Err(hint.into());
    }

    Ok(SignedPsbtDto {
        base64: signed_b64,
        input_count: psbt.inputs.len(),
        output_count: psbt.outputs.len(),
        signed_inputs: count_signed_inputs(&psbt),
        total_inputs: psbt.inputs.len(),
    })
}

/// Enumerate policy spending paths (primary branches + timelock fallbacks).
#[tauri::command]
pub fn list_spending_paths(
    state: State<'_, AppState>,
    wallet_id: String,
) -> Result<Vec<policy_engine::SpendingPath>, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        let wallet = service.get_wallet(&wallet_id).map_err(user_facing_error)?;
        policy_engine::spending_paths(&wallet.policy).map_err(user_facing_error)
    })
}

/// Analyze which wallet keys have signed a PSBT and which paths are satisfied.
#[tauri::command]
pub fn analyze_psbt_status(
    state: State<'_, AppState>,
    request: AnalyzePsbtRequest,
) -> Result<SigningStatus, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        let wallet = service
            .get_wallet(&request.wallet_id)
            .map_err(user_facing_error)?;
        let psbt = parse_psbt_b64(&request.psbt_base64)?;
        analyze_signing_status(
            &wallet.policy,
            &psbt,
            request.active_path_id.as_deref(),
        )
        .map_err(user_facing_error)
    })
}

/// Build BIP-388 / Coldcard / Ledger registration materials for a wallet.
#[tauri::command]
pub fn prepare_hw_registration(
    state: State<'_, AppState>,
    wallet_id: String,
) -> Result<RegistrationPackage, String> {
    state.with_store(|store| {
        let service = WalletService::new(store);
        let wallet = service.get_wallet(&wallet_id).map_err(user_facing_error)?;
        build_registration_package(&wallet.name, &wallet.policy, &wallet.descriptor)
            .map_err(user_facing_error)
    })
}

/// Attempt on-device registration (HWI `registerpolicy` when available).
#[tauri::command]
pub fn hw_register_wallet(
    state: State<'_, AppState>,
    request: HwRegisterRequest,
) -> Result<HwRegisterResultDto, String> {
    let (mut package, network, cosigner_hints, wallet_keys, wallet_policy) = state.with_store(|store| {
        let service = WalletService::new(store);
        let wallet = service
            .get_wallet(&request.wallet_id)
            .map_err(user_facing_error)?;
        let package =
            build_registration_package(&wallet.name, &wallet.policy, &wallet.descriptor)
                .map_err(user_facing_error)?;
        Ok((
            package,
            wallet.policy.network,
            primary_cosigner_hints(&wallet.policy),
            wallet.policy.keys.clone(),
            wallet.policy,
        ))
    })?;

    let client = hwi_client(
        &state,
        request.hwi_path.as_deref(),
        true,
        Some(network),
    )?;
    let fingerprint = client
        .resolve_fingerprint(request.fingerprint.trim())
        .map_err(user_facing_error)?;

    if ledger_registers_on_first_psbt(&package.descriptor) {
        if let Ok(device) = client.find_device(&fingerprint) {
            if device.device_type == DeviceType::Ledger {
                let ledger_cli =
                    resolve_ledger_cli(&state.data_dir, network).map_err(user_facing_error)?;
                let ledger_bip388 =
                    to_ledger_wallet_policy(&package.bip388, &wallet_policy, network)
                        .map_err(user_facing_error)?;
                let readiness = ledger_readiness_for_wallet(
                    &state.data_dir,
                    network,
                    Some(&ledger_bip388.policy),
                );
                if !readiness.ready {
                    let expected = expected_bitcoin_app_name(network);
                    let mut lines = readiness.warnings;
                    if lines.is_empty() {
                        lines.push(format!(
                            "Open \"{expected}\" on the Ledger (≥ 2.2.1) before registering."
                        ));
                    }
                    return Err(user_facing_error(signing_devices::SignError::Ledger(
                        lines.join(" "),
                    )));
                }
                match ledger_register_wallet(&ledger_cli, &ledger_bip388) {
                    Ok(hmac) => {
                        save_registration(
                            &state.data_dir,
                            &request.wallet_id,
                            &fingerprint,
                            &hmac,
                            &ledger_bip388,
                            network,
                        )
                        .map_err(user_facing_error)?;
                        package.ledger_hmac = Some(hmac.clone());
                        return Ok(HwRegisterResultDto {
                            ok: true,
                            message: "Ledger wallet policy registered — confirm prompts on the device screen."
                                .into(),
                            hmac: Some(hmac),
                            package,
                            cosigner_hints,
                        });
                    }
                    Err(err) => return Err(user_facing_error(err)),
                }
            }
        }
        if let Some(key) = find_key_by_fingerprint(&wallet_keys, &fingerprint) {
            let single_desc = single_key_display_descriptor(key);
            if let Ok(addr) = client.display_address_desc(&fingerprint, &single_desc) {
                return Ok(HwRegisterResultDto {
                    ok: true,
                    message: format!(
                        "Key {key_id} (fp {fingerprint}) confirmed on Ledger — check the device screen for address {addr}. \
                         The full ABC multisig policy still registers when you sign your first PSBT in Send.",
                        key_id = key.id,
                    ),
                    hmac: None,
                    package,
                    cosigner_hints,
                });
            }
        }
        return Ok(HwRegisterResultDto {
            ok: true,
            message: format!(
                "Device {fingerprint} connected over USB — no Ledger prompt at this step is normal for non-Ledger devices. \
                 For Ledger ABC wallets: Settings → Install Ledger signer, then Register Ledger policy here."
            ),
            hmac: None,
            package,
            cosigner_hints,
        });
    }

    let keys_json = serde_json::to_string(&package.bip388.keys).map_err(user_facing_error)?;

    if client.cli_supports_registerpolicy() {
        match client.register_policy(
            &fingerprint,
            &package.bip388.name,
            &package.bip388.policy,
            &keys_json,
        ) {
            Ok(hmac) => {
                package.ledger_hmac = hmac.clone();
                package.hwi_registerpolicy_supported = true;
                return Ok(HwRegisterResultDto {
                    ok: true,
                    message: "Wallet policy registered on device — confirm remaining prompts on the hardware screen.".into(),
                    hmac,
                    package,
                    cosigner_hints,
                });
            }
            Err(err) if is_registerpolicy_unavailable(&err) => {}
            Err(err) => return Err(user_facing_error(err)),
        }
    }

    match client.display_address_desc(&fingerprint, &package.descriptor) {
        Ok(addr) => Ok(HwRegisterResultDto {
            ok: true,
            message: format!(
                "Confirmed wallet on device ({addr}). If this is your first spend, the device may still prompt when signing the PSBT."
            ),
            hmac: None,
            package,
            cosigner_hints,
        }),
        Err(display_err) => Ok(HwRegisterResultDto {
            ok: false,
            message: format!(
                "Could not confirm address on device: {display_err}. Save BIP-388 / Coldcard files below, or register when you sign your first PSBT."
            ),
            hmac: None,
            package,
            cosigner_hints,
        }),
    }
}

fn ledger_runtime_to_dto(
    runtime: signing_devices::ResolvedLedgerRuntime,
    message: Option<String>,
) -> LedgerRuntimeStatusDto {
    LedgerRuntimeStatusDto {
        available: true,
        python_path: Some(runtime.python.display().to_string()),
        script_path: Some(runtime.script.display().to_string()),
        pinned_version: PINNED_LEDGER_BITCOIN_VERSION.to_string(),
        installed_version: Some(runtime.version),
        source: Some(runtime_source_label(runtime.source).to_string()),
        script_ready: true,
        message,
    }
}

/// Probe for ledger-bitcoin runtime in app data / env. Does not install.
#[tauri::command]
pub fn get_ledger_runtime_status(
    state: State<'_, AppState>,
) -> Result<LedgerRuntimeStatusDto, String> {
    let script_ready = signing_devices::ensure_ledger_cli_script(&state.data_dir).is_ok();
    match find_ledger_runtime(&state.data_dir) {
        Some(runtime) => Ok(LedgerRuntimeStatusDto {
            script_ready,
            ..ledger_runtime_to_dto(runtime, None)
        }),
        None => Ok(LedgerRuntimeStatusDto {
            available: false,
            python_path: None,
            script_path: signing_devices::ensure_ledger_cli_script(&state.data_dir)
                .ok()
                .map(|p| p.display().to_string()),
            pinned_version: PINNED_LEDGER_BITCOIN_VERSION.to_string(),
            installed_version: None,
            source: None,
            script_ready,
            message: Some(format!(
                "Ledger signer missing — installs ledger-bitcoin v{PINNED_LEDGER_BITCOIN_VERSION} into app data \
                 (one-time Python bootstrap may be required)"
            )),
        }),
    }
}

/// Install or refresh the ledger-bitcoin runtime into app data.
#[tauri::command]
pub fn ensure_ledger_runtime_installed(
    state: State<'_, AppState>,
) -> Result<LedgerRuntimeStatusDto, String> {
    let had_runtime = find_ledger_runtime(&state.data_dir).is_some();
    let runtime =
        signing_devices::ensure_ledger_runtime(&state.data_dir).map_err(user_facing_error)?;
    let message = if had_runtime {
        Some(format!(
            "Ledger signer ready — ledger-bitcoin v{} ({})",
            runtime.version,
            runtime_source_label(runtime.source)
        ))
    } else {
        Some(format!(
            "Installed ledger-bitcoin v{} into app data",
            runtime.version
        ))
    };
    Ok(ledger_runtime_to_dto(runtime, message))
}

fn ledger_readiness_for_wallet(
    data_dir: &std::path::Path,
    network: NetworkName,
    policy: Option<&str>,
) -> signing_devices::LedgerReadiness {
    let device = find_ledger_runtime(data_dir).and_then(|_| {
        let cli = resolve_ledger_cli(data_dir, network).ok()?;
        ledger_probe_device(&cli).ok()
    });
    let device_info = device.as_ref().map(|(n, v)| (n.as_str(), v.as_str()));
    evaluate_ledger_readiness(network, device_info, policy)
}

/// Whether a Ledger wallet-policy HMAC is stored for this wallet + fingerprint.
#[tauri::command]
pub fn get_ledger_registration_status(
    state: State<'_, AppState>,
    wallet_id: String,
    fingerprint: String,
) -> Result<LedgerRegistrationStatusDto, String> {
    let fp = fingerprint.trim().to_ascii_lowercase();
    let runtime = find_ledger_runtime(&state.data_dir);
    let reg = load_registration(&state.data_dir, &wallet_id, &fp);

    let (stale, stale_reason, registered, network, ledger_policy) = state.with_store(|store| {
        let service = WalletService::new(store);
        let wallet = service.get_wallet(&wallet_id).map_err(user_facing_error)?;
        let bip388 =
            descriptor_to_bip388(&wallet.name, &wallet.policy, &wallet.descriptor)
                .map_err(user_facing_error)?;
        let ledger_bip388 =
            to_ledger_wallet_policy(&bip388, &wallet.policy, wallet.policy.network)
                .map_err(user_facing_error)?;
        let reg = reg.as_ref();
        let stale_reason = reg
            .and_then(|r| registration_stale_reason(r, &ledger_bip388, wallet.policy.network))
            .map(str::to_string);
        let stale = stale_reason.is_some();
        let registered = reg.is_some() && !stale;
        Ok((
            stale,
            stale_reason,
            registered,
            wallet.policy.network,
            ledger_bip388.policy,
        ))
    })?;

    let readiness = if runtime.is_some() {
        ledger_readiness_for_wallet(&state.data_dir, network, Some(&ledger_policy))
    } else {
        evaluate_ledger_readiness(network, None, Some(&ledger_policy))
    };

    Ok(LedgerRegistrationStatusDto {
        registered,
        stale,
        stale_reason,
        fingerprint: fp,
        python_available: runtime.is_some(),
        ledger_cli_ready: signing_devices::ensure_ledger_cli_script(&state.data_dir).is_ok(),
        runtime_source: runtime
            .as_ref()
            .map(|r| runtime_source_label(r.source).to_string()),
        installed_version: runtime.map(|r| r.version),
        app_name: readiness.app_name,
        app_version: readiness.app_version,
        expected_app_name: readiness.expected_app_name,
        device_connected: readiness.device_connected,
        warnings: readiness.warnings,
        ready: readiness.ready,
    })
}

#[tauri::command]
pub fn import_psbt(psbt_base64: String) -> Result<PsbtDto, String> {
    let psbt = parse_psbt_b64(&psbt_base64)?;
    let base64 = encode_psbt(&psbt)?;
    Ok(PsbtDto {
        base64,
        input_count: psbt.inputs.len(),
        output_count: psbt.outputs.len(),
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
pub fn finalize_psbt_cmd(
    state: State<'_, AppState>,
    psbt_base64: String,
    wallet_id: Option<String>,
) -> Result<FinalizedTxDto, String> {
    let mut psbt = parse_psbt_b64(&psbt_base64)?;
    let tx = finalize_psbt(&mut psbt).map_err(user_facing_error)?;

    let btc_net = match wallet_id.as_deref().filter(|id| !id.is_empty()) {
        Some(id) => state.with_store(|store| {
            WalletService::new(store)
                .get_wallet(id)
                .map(|w| w.policy.network.to_bitcoin_network())
                .map_err(user_facing_error)
        })?,
        None => bitcoin::Network::Bitcoin,
    };

    let outputs = tx
        .output
        .iter()
        .map(|out| {
            let address = bitcoin::Address::from_script(&out.script_pubkey, btc_net)
                .ok()
                .map(|a| a.to_string());
            TxOutputDto {
                address,
                amount_sats: out.value.to_sat(),
            }
        })
        .collect();

    Ok(FinalizedTxDto {
        hex: transaction_hex(&tx),
        txid: tx.compute_txid().to_string(),
        fully_signed: true,
        outputs,
    })
}

/// Finalize (if needed) and broadcast via Esplora for the wallet's network.
#[tauri::command]
pub fn broadcast_psbt_cmd(
    state: State<'_, AppState>,
    request: BroadcastTxRequest,
) -> Result<String, String> {
    let network = state.with_store(|store| {
        let service = WalletService::new(store);
        service
            .get_wallet(&request.wallet_id)
            .map(|w| w.policy.network)
            .map_err(user_facing_error)
    })?;
    let backend = esplora_backend(request.esplora_url.clone(), network)?;

    if let Some(hex) = request.tx_hex.as_ref().filter(|h| !h.trim().is_empty()) {
        return backend.broadcast(hex.trim()).map_err(user_facing_error);
    }

    let psbt_b64 = request
        .psbt_base64
        .as_ref()
        .ok_or_else(|| "provide psbtBase64 or txHex".to_string())?;
    let psbt = parse_psbt_b64(psbt_b64)?;
    broadcast_psbt(&psbt, &backend).map_err(user_facing_error)
}

fn hot_summary_dto(s: hot_keystore::HotWalletSummary) -> HotWalletSummaryDto {
    HotWalletSummaryDto {
        id: s.id,
        name: s.name,
        network: s.network,
        fingerprint: s.fingerprint,
        origin_path: s.origin_path,
        xpub: s.xpub,
        linked_workspace_id: s.linked_workspace_id,
        linked_wallet_id: s.linked_wallet_id,
        created_at: s.created_at,
    }
}

/// Extract mnemonic from raw BIP-39 text or Sparrow/Electrum-ish JSON.
fn extract_mnemonic_payload(raw: &str) -> Result<(String, Option<String>), String> {
    let trimmed = raw.trim();
    if trimmed.starts_with('{') {
        let value: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|e| format!("invalid JSON: {e}"))?;
        let obj = value
            .as_object()
            .ok_or_else(|| "JSON root must be an object".to_string())?;
        let mnemonic = ["mnemonic", "seed", "words", "bip39"]
            .iter()
            .find_map(|k| {
                obj.get(*k)
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
            })
            .ok_or_else(|| {
                "JSON has no mnemonic/seed field (Sparrow/Electrum-style hot import)".to_string()
            })?;
        let passphrase = ["passphrase", "bip39Passphrase", "bip39_passphrase"]
            .iter()
            .find_map(|k| {
                obj.get(*k)
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
            });
        Ok((mnemonic.to_string(), passphrase))
    } else {
        Ok((trimmed.to_string(), None))
    }
}

#[tauri::command]
pub fn hot_keystore_status(state: State<'_, AppState>) -> Result<HotKeystoreStatusDto, String> {
    let exists = hot_keystore::HotKeystore::exists(&state.data_dir);
    let unlocked = state.with_hot_mut(|slot| Ok(slot.is_some()))?;
    Ok(HotKeystoreStatusDto {
        exists,
        unlocked,
        path: hot_keystore::HotKeystore::path_in(&state.data_dir)
            .display()
            .to_string(),
    })
}

#[tauri::command]
pub fn create_hot_keystore(
    state: State<'_, AppState>,
    request: CreateHotKeystoreRequest,
) -> Result<HotKeystoreStatusDto, String> {
    state.with_hot_mut(|slot| {
        let ks = hot_keystore::HotKeystore::create(&state.data_dir, &request.master_password)
            .map_err(user_facing_error)?;
        *slot = Some(ks);
        Ok(())
    })?;
    hot_keystore_status(state)
}

#[tauri::command]
pub fn unlock_hot_keystore(
    state: State<'_, AppState>,
    request: UnlockHotKeystoreRequest,
) -> Result<HotKeystoreStatusDto, String> {
    state.with_hot_mut(|slot| {
        let ks = hot_keystore::HotKeystore::unlock(&state.data_dir, &request.master_password)
            .map_err(user_facing_error)?;
        *slot = Some(ks);
        Ok(())
    })?;
    hot_keystore_status(state)
}

#[tauri::command]
pub fn lock_hot_keystore(state: State<'_, AppState>) -> Result<HotKeystoreStatusDto, String> {
    state.with_hot_mut(|slot| {
        *slot = None;
        Ok(())
    })?;
    hot_keystore_status(state)
}

#[tauri::command]
pub fn list_hot_wallets(state: State<'_, AppState>) -> Result<Vec<HotWalletSummaryDto>, String> {
    state.with_hot_unlocked(|ks| Ok(ks.list().into_iter().map(hot_summary_dto).collect()))
}

/// Import BIP-39 as a hot Bitcoin wallet (send / receive / history).
#[tauri::command]
pub fn import_hot_wallet(
    state: State<'_, AppState>,
    request: ImportHotWalletRequestDto,
) -> Result<ImportHotWalletResultDto, String> {
    let (mnemonic, json_pass) = extract_mnemonic_payload(&request.mnemonic_or_json)?;
    let passphrase = if !request.bip39_passphrase.is_empty() {
        request.bip39_passphrase.clone()
    } else {
        json_pass.unwrap_or_default()
    };

    let import_req = hot_keystore::ImportHotWalletRequest {
        name: request.name.clone(),
        mnemonic,
        bip39_passphrase: passphrase,
        network: request.network,
        account_path: request.account_path.clone(),
    };
    let (mut record, key) =
        hot_keystore::derive_bip86_account(&import_req).map_err(user_facing_error)?;

    let _ = request.create_nested_wallet;
    let workspace_id = if request.workspace_id.trim().is_empty() {
        ensure_hot_parent_workspace(&state, request.network)?
    } else {
        state.with_store(|store| {
            let workspace = store
                .open_workspace(&request.workspace_id)
                .map_err(user_facing_error)?;
            if workspace.network != request.network {
                return Err(format!(
                    "network mismatch: workspace is {:?}, hot import is {:?}",
                    workspace.network, request.network
                ));
            }
            Ok(request.workspace_id.clone())
        })?
    };
    let wallet_name = request.name.trim().to_string();

    let wallet = state.with_store(|store| {
        let service = WalletService::new(store);
        let policy = PolicyConfig {
            version: policy_engine::POLICY_SCHEMA_VERSION,
            network: request.network,
            script_type: policy_engine::ScriptTypeName::Taproot,
            keys: vec![key],
            policy: policy_engine::PolicyExpression {
                primary: "A".into(),
                fallback: None,
                fallbacks: vec![],
            },
        };
        service
            .create_wallet_with_receive_address(&workspace_id, &wallet_name, policy)
            .map(|result| WalletDto::from(result.wallet))
            .map_err(user_facing_error)
    })?;

    record.linked_workspace_id = Some(workspace_id);
    record.linked_wallet_id = Some(wallet.id.clone());

    let summary = state.with_hot_unlocked_mut(|ks| {
        ks.insert(record)
            .map(hot_summary_dto)
            .map_err(user_facing_error)
    })?;

    Ok(ImportHotWalletResultDto {
        hot_wallet: summary,
        wallet: Some(wallet),
    })
}

fn ensure_hot_parent_workspace(
    state: &State<'_, AppState>,
    network: NetworkName,
) -> Result<String, String> {
    state.with_store(|store| {
        let workspaces = store.list_workspaces().map_err(user_facing_error)?;
        if let Some(existing) = workspaces.into_iter().find(|w| w.network == network) {
            return Ok(existing.id);
        }
        store
            .create_workspace("Hot wallets", network)
            .map(|w| w.id)
            .map_err(user_facing_error)
    })
}

/// Prefer an existing linked / requested parent; if that SQLite workspace was deleted, recreate.
fn resolve_hot_parent_workspace(
    state: &State<'_, AppState>,
    network: NetworkName,
    preferred: Option<String>,
) -> Result<String, String> {
    if let Some(id) = preferred.filter(|s| !s.trim().is_empty()) {
        let alive = state.with_store(|store| match store.open_workspace(&id) {
            Ok(workspace) if workspace.network == network => Ok(true),
            Ok(_) => Ok(false),
            Err(_) => Ok(false),
        })?;
        if alive {
            return Ok(id);
        }
    }
    ensure_hot_parent_workspace(state, network)
}

/// Open a hot wallet’s detail: reuse linked chain row, or create storage if missing.
#[tauri::command]
pub fn open_hot_wallet(
    state: State<'_, AppState>,
    hot_wallet_id: String,
    workspace_id: Option<String>,
) -> Result<WalletDto, String> {
    let rec = state.with_hot_unlocked(|ks| {
        ks.get(&hot_wallet_id)
            .map(|r| r.clone())
            .map_err(user_facing_error)
    })?;

    if let Some(ref wallet_id) = rec.linked_wallet_id {
        match state.with_store(|store| {
            let service = WalletService::new(store);
            service
                .get_wallet(wallet_id)
                .map(WalletDto::from)
                .map_err(user_facing_error)
        }) {
            Ok(wallet) => {
                // Hot singlesig used to compile as tr(NUMS,{pk(A)}) — wrong addresses vs Sparrow BIP-86.
                if wallet.policy.keys.len() == 1
                    && wallet.policy.policy.primary.trim() == "A"
                    && wallet.policy.policy.all_fallbacks().is_empty()
                    && wallet
                        .descriptor
                        .contains(descriptor_engine::NUMS_UNSPENDABLE_KEY)
                {
                    let _ = state.with_store(|store| {
                        let service = WalletService::new(store);
                        service.delete_wallet(wallet_id).map_err(user_facing_error)
                    });
                } else {
                    return Ok(wallet);
                }
            }
            Err(_) => {
                // Linked row was deleted — recreate below.
            }
        }
    }

    let preferred = workspace_id
        .filter(|s| !s.trim().is_empty())
        .or(rec.linked_workspace_id.clone());
    let parent_id = resolve_hot_parent_workspace(&state, rec.network, preferred)?;

    let key = hot_keystore::account_policy_key(&rec);
    let wallet = state.with_store(|store| {
        let service = WalletService::new(store);
        let policy = PolicyConfig {
            version: policy_engine::POLICY_SCHEMA_VERSION,
            network: rec.network,
            script_type: policy_engine::ScriptTypeName::Taproot,
            keys: vec![key],
            policy: policy_engine::PolicyExpression {
                primary: "A".into(),
                fallback: None,
                fallbacks: vec![],
            },
        };
        service
            .create_wallet_with_receive_address(&parent_id, &rec.name, policy)
            .map(|result| WalletDto::from(result.wallet))
            .map_err(user_facing_error)
    })?;

    state.with_hot_unlocked_mut(|ks| {
        ks.set_links(
            &hot_wallet_id,
            Some(parent_id),
            Some(wallet.id.clone()),
        )
        .map_err(user_facing_error)?;
        Ok(())
    })?;

    Ok(wallet)
}

#[tauri::command]
pub fn rename_hot_wallet(
    state: State<'_, AppState>,
    hot_wallet_id: String,
    name: String,
) -> Result<HotWalletSummaryDto, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("name required".into());
    }

    let linked_wallet_id = state.with_hot_unlocked(|ks| {
        ks.get(&hot_wallet_id)
            .map(|r| r.linked_wallet_id.clone())
            .map_err(user_facing_error)
    })?;

    // Linked wallet may be gone (parent workspace deleted) — still rename the hot record.
    if let Some(wallet_id) = linked_wallet_id.as_ref() {
        let _ = state.with_store(|store| {
            let service = WalletService::new(store);
            service.rename_wallet(wallet_id, &name).map_err(user_facing_error)
        });
    }

    state.with_hot_unlocked_mut(|ks| {
        ks.rename(&hot_wallet_id, &name)
            .map(hot_summary_dto)
            .map_err(user_facing_error)
    })
}

#[tauri::command]
pub fn delete_hot_wallet(state: State<'_, AppState>, hot_wallet_id: String) -> Result<(), String> {
    state.with_hot_unlocked_mut(|ks| {
        ks.remove(&hot_wallet_id).map_err(user_facing_error)?;
        Ok(())
    })
}

/// Sign a PSBT using a stored hot wallet (no paste of tprv).
#[tauri::command]
pub fn sign_psbt_hot(
    state: State<'_, AppState>,
    request: SignPsbtHotRequest,
) -> Result<SignedPsbtDto, String> {
    assert_hot_key_allowed(request.network, request.allow_mainnet_hot_keys)?;
    let secret = state.with_hot_unlocked(|ks| {
        ks.descriptor_secret(&request.hot_wallet_id)
            .map_err(user_facing_error)
    })?;

    let mut psbt = parse_psbt_b64(&request.psbt_base64)?;
    let secret_key = DescriptorSecretKey::from_str(secret.trim())
        .map_err(|e| user_facing_error(format!("invalid stored secret: {e}")))?;
    let progress = sign_psbt(&mut psbt, &SoftwareSigner::from_secret(secret_key))
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
