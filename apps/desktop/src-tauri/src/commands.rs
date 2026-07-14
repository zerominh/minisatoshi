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
    finalize_psbt, import_psbt_base64, sign_psbt, transaction_hex, CreatePsbtOptions, ExportFormat,
    FeeRate, PsbtRecipient, SoftwareSigner, SpendingUtxo, SigningStatus,
};
use tauri::State;
use vault::VaultService;

use crate::dto::{
    AddressDto, AnalyzePsbtRequest, BalanceDto, BroadcastTxRequest, BsmsExportDto,
    CombinePsbtRequest, CompileVaultResponse, CreateHotKeystoreRequest, CreatePsbtRequest,
    CreateVaultRequest, CreateWalletRequest, FinalizedTxDto, HotKeystoreStatusDto,
    HotWalletSummaryDto, HwDeviceDto, HwGetXpubRequest, HwRegisterRequest, HwRegisterResultDto,
    HwSignPsbtRequest, HwStatusDto, HwXpubDto, ImportDescriptorRequest, ImportHotWalletRequestDto,
    ImportHotWalletResultDto, ImportVaultBackupRequest, PsbtDto, ServerPresetDto,
    SignPsbtHotRequest, SignPsbtRequest, SignedPsbtDto, SparrowExportDto, SyncResultDto,
    UnlockHotKeystoreRequest, VaultBackupDto, VaultDto, VaultSummaryDto, WalletDto,
    WalletSummaryDto,
};
use crate::error::user_facing_error;
use crate::state::AppState;
use signing_devices::{
    build_registration_package, ensure_hwi, find_hwi, hwi_chain, parse_derivation_path,
    primary_cosigner_hints, DeviceInfo, HwiClient, HwiSource, RegistrationPackage,
    PINNED_HWI_VERSION,
};

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

/// Import a checksummed descriptor (optional policy JSON) into a wallet.
#[tauri::command]
pub fn import_descriptor(
    state: State<'_, AppState>,
    request: ImportDescriptorRequest,
) -> Result<VaultDto, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        service
            .import_descriptor(
                &request.wallet_id,
                &request.name,
                &request.descriptor,
                request.policy,
            )
            .map(VaultDto::from)
            .map_err(user_facing_error)
    })
}

/// Import watch-only: `minisatoshi-vault-v1.json`, bare descriptor, BSMS, or Liana-ish JSON.
#[tauri::command]
pub fn import_vault_backup(
    state: State<'_, AppState>,
    request: ImportVaultBackupRequest,
) -> Result<VaultDto, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        service
            .import_watch_only_payload(
                &request.wallet_id,
                &request.payload,
                request.name.as_deref(),
            )
            .map(VaultDto::from)
            .map_err(user_facing_error)
    })
}

/// Export portable vault backup (JSON + descriptor text).
#[tauri::command]
pub fn export_vault_backup(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<VaultBackupDto, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        let backup = service
            .export_vault_backup(&vault_id)
            .map_err(user_facing_error)?;
        let json = backup.to_json_pretty().map_err(user_facing_error)?;
        let descriptor_txt = backup.descriptor_txt();
        Ok(VaultBackupDto {
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
    vault_id: String,
) -> Result<BsmsExportDto, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        let text = service.export_bsms(&vault_id).map_err(user_facing_error)?;
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
) -> Result<Vec<HwDeviceDto>, String> {
    let client = hwi_client(&state, hwi_path.as_deref(), true, None)?;
    let devices = client.enumerate().map_err(user_facing_error)?;
    Ok(devices.into_iter().map(device_to_dto).collect())
}

/// Fetch an xpub from a connected device (`hwi getxpub`).
#[tauri::command]
pub fn hw_get_xpub(
    state: State<'_, AppState>,
    request: HwGetXpubRequest,
) -> Result<HwXpubDto, String> {
    let client = hwi_client(&state, request.hwi_path.as_deref(), true, None)?;
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

/// Sign a PSBT on-device (`hwi signtx`). Secrets never leave the hardware.
#[tauri::command]
pub fn hw_sign_psbt(
    state: State<'_, AppState>,
    request: HwSignPsbtRequest,
) -> Result<SignedPsbtDto, String> {
    let client = hwi_client(&state, request.hwi_path.as_deref(), true, None)?;
    let signed_b64 = client
        .sign_psbt(request.fingerprint.trim(), &request.psbt_base64)
        .map_err(user_facing_error)?;
    let psbt = parse_psbt_b64(&signed_b64)?;
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
    vault_id: String,
) -> Result<Vec<policy_engine::SpendingPath>, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        let vault = service.get_vault(&vault_id).map_err(user_facing_error)?;
        policy_engine::spending_paths(&vault.policy).map_err(user_facing_error)
    })
}

/// Analyze which vault keys have signed a PSBT and which paths are satisfied.
#[tauri::command]
pub fn analyze_psbt_status(
    state: State<'_, AppState>,
    request: AnalyzePsbtRequest,
) -> Result<SigningStatus, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        let vault = service
            .get_vault(&request.vault_id)
            .map_err(user_facing_error)?;
        let psbt = parse_psbt_b64(&request.psbt_base64)?;
        analyze_signing_status(
            &vault.policy,
            &psbt,
            request.active_path_id.as_deref(),
        )
        .map_err(user_facing_error)
    })
}

/// Build BIP-388 / Coldcard / Ledger registration materials for a vault.
#[tauri::command]
pub fn prepare_hw_registration(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<RegistrationPackage, String> {
    state.with_store(|store| {
        let service = VaultService::new(store);
        let vault = service.get_vault(&vault_id).map_err(user_facing_error)?;
        build_registration_package(&vault.name, &vault.policy, &vault.descriptor)
            .map_err(user_facing_error)
    })
}

/// Attempt on-device registration (HWI `registerpolicy` when available).
#[tauri::command]
pub fn hw_register_vault(
    state: State<'_, AppState>,
    request: HwRegisterRequest,
) -> Result<HwRegisterResultDto, String> {
    let (mut package, network, cosigner_hints) = state.with_store(|store| {
        let service = VaultService::new(store);
        let vault = service
            .get_vault(&request.vault_id)
            .map_err(user_facing_error)?;
        let package =
            build_registration_package(&vault.name, &vault.policy, &vault.descriptor)
                .map_err(user_facing_error)?;
        Ok((
            package,
            vault.policy.network,
            primary_cosigner_hints(&vault.policy),
        ))
    })?;

    let client = hwi_client(
        &state,
        request.hwi_path.as_deref(),
        true,
        Some(network),
    )?;
    let keys_json = serde_json::to_string(&package.bip388.keys).map_err(user_facing_error)?;

    match client.register_policy(
        request.fingerprint.trim(),
        &package.bip388.name,
        &package.bip388.policy,
        &keys_json,
    ) {
        Ok(hmac) => {
            package.ledger_hmac = hmac.clone();
            package.hwi_registerpolicy_supported = true;
            Ok(HwRegisterResultDto {
                ok: true,
                message: "Wallet policy registered on device — confirm remaining prompts on the hardware screen.".into(),
                hmac,
                package,
                cosigner_hints,
            })
        }
        Err(err) => {
            let msg = err.to_string();
            let unsupported = msg.to_ascii_lowercase().contains("registerpolicy")
                || msg.to_ascii_lowercase().contains("unsupported");
            if unsupported {
                Ok(HwRegisterResultDto {
                    ok: false,
                    message: format!(
                        "On-device registerpolicy unavailable ({msg}). Export BIP-388 / Coldcard files from the package, then register via firmware flow or first co-sign."
                    ),
                    hmac: None,
                    package,
                    cosigner_hints,
                })
            } else {
                Err(user_facing_error(err))
            }
        }
    }
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

fn hot_summary_dto(s: hot_keystore::HotWalletSummary) -> HotWalletSummaryDto {
    HotWalletSummaryDto {
        id: s.id,
        name: s.name,
        network: s.network,
        fingerprint: s.fingerprint,
        origin_path: s.origin_path,
        xpub: s.xpub,
        linked_wallet_id: s.linked_wallet_id,
        linked_vault_id: s.linked_vault_id,
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

/// Import BIP-39 (or Sparrow/Electrum JSON with mnemonic) as a nested hot vault under a wallet.
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

    // Ensure parent wallet network matches the hot wallet network.
    state.with_store(|store| {
        let wallet = store
            .open_wallet(&request.wallet_id)
            .map_err(user_facing_error)?;
        if wallet.network != request.network {
            return Err(format!(
                "network mismatch: wallet is {:?}, hot import is {:?}",
                wallet.network, request.network
            ));
        }
        Ok(())
    })?;

    let create_vault = request.create_nested_vault;
    let wallet_id = request.wallet_id.clone();
    let vault_name = format!("{} (hot)", request.name.trim());

    let vault = if create_vault {
        Some(state.with_store(|store| {
            let service = VaultService::new(store);
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
                .create_vault_with_receive_address(&wallet_id, &vault_name, policy)
                .map(|result| VaultDto::from(result.vault))
                .map_err(user_facing_error)
        })?)
    } else {
        None
    };

    if let Some(ref v) = vault {
        record.linked_wallet_id = Some(wallet_id.clone());
        record.linked_vault_id = Some(v.id.clone());
    } else {
        record.linked_wallet_id = Some(wallet_id);
    }

    let summary = state.with_hot_unlocked_mut(|ks| {
        ks.insert(record)
            .map(hot_summary_dto)
            .map_err(user_facing_error)
    })?;

    Ok(ImportHotWalletResultDto {
        hot_wallet: summary,
        vault,
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
