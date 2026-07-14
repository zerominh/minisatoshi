mod commands;
mod dto;
mod error;
mod state;

use commands::{
    app_version, broadcast_psbt_cmd, combine_psbts, compile_vault_descriptor, create_psbt,
    create_vault, create_wallet, export_sparrow_wallet, finalize_psbt_cmd, get_balance, get_vault,
    hw_get_xpub, hw_sign_psbt, list_hw_devices, list_server_presets, list_vaults, list_wallets,
    new_receive_address, prepare_hw_registration, save_text_file, sign_psbt_software, sync_vault,
    ensure_hwi_installed, get_hwi_status, hw_register_vault,
};
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| format!("failed to create app data dir: {e}"))?;
            let db_path = data_dir.join("minisatoshi.db");
            let state = AppState::open(data_dir, db_path)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            compile_vault_descriptor,
            create_wallet,
            list_wallets,
            create_vault,
            list_vaults,
            get_vault,
            new_receive_address,
            get_balance,
            sync_vault,
            create_psbt,
            sign_psbt_software,
            list_hw_devices,
            hw_get_xpub,
            hw_sign_psbt,
            get_hwi_status,
            ensure_hwi_installed,
            prepare_hw_registration,
            hw_register_vault,
            combine_psbts,
            finalize_psbt_cmd,
            broadcast_psbt_cmd,
            export_sparrow_wallet,
            list_server_presets,
            app_version,
            save_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
