mod commands;
mod dto;
mod state;

use commands::{
    compile_vault_descriptor, create_psbt, create_vault, create_wallet, export_sparrow_wallet,
    get_balance, get_vault, list_server_presets, list_vaults, list_wallets, new_receive_address,
};
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| format!("failed to create app data dir: {e}"))?;
            let db_path = data_dir.join("minisatoshi.db");
            let state = AppState::open(db_path)?;
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
            create_psbt,
            export_sparrow_wallet,
            list_server_presets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
