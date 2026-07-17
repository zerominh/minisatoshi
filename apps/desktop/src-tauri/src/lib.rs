mod commands;
mod dto;
mod error;
mod state;

use commands::{
    analyze_psbt_status, app_version, broadcast_psbt_cmd, combine_psbts, compile_wallet_descriptor,
    create_hot_keystore, create_psbt, create_wallet, create_workspace, delete_hot_wallet,
    delete_wallet, delete_workspace, ensure_hwi_installed, export_bsms, export_sparrow_wallet,
    export_wallet_backup, finalize_psbt_cmd, get_balance, get_hwi_status, get_ledger_registration_status,
    get_ledger_runtime_status, get_wallet,
    hot_keystore_status, hw_get_xpub, hw_register_wallet, hw_sign_psbt, import_descriptor,
    import_hot_wallet, import_psbt, import_wallet_backup, list_hot_wallets, list_hw_devices,
    list_server_presets, list_spending_paths, list_wallets, list_workspaces, list_addresses,
    lock_hot_keystore, new_receive_address, open_hot_wallet, prepare_hw_registration,
    rename_hot_wallet, rename_wallet, rename_workspace, save_text_file, open_text_file,
    sign_psbt_hot, sign_psbt_software, sync_wallet, unlock_hot_keystore, ensure_ledger_runtime_installed,
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
            compile_wallet_descriptor,
            create_workspace,
            list_workspaces,
            delete_workspace,
            rename_workspace,
            create_wallet,
            import_descriptor,
            import_wallet_backup,
            export_wallet_backup,
            export_bsms,
            list_wallets,
            delete_wallet,
            rename_wallet,
            get_wallet,
            new_receive_address,
            list_addresses,
            get_balance,
            sync_wallet,
            create_psbt,
            import_psbt,
            sign_psbt_software,
            sign_psbt_hot,
            create_hot_keystore,
            unlock_hot_keystore,
            lock_hot_keystore,
            hot_keystore_status,
            list_hot_wallets,
            import_hot_wallet,
            open_hot_wallet,
            rename_hot_wallet,
            delete_hot_wallet,
            list_spending_paths,
            analyze_psbt_status,
            list_hw_devices,
            hw_get_xpub,
            hw_sign_psbt,
            get_hwi_status,
            ensure_hwi_installed,
            prepare_hw_registration,
            hw_register_wallet,
            get_ledger_registration_status,
            get_ledger_runtime_status,
            ensure_ledger_runtime_installed,
            combine_psbts,
            finalize_psbt_cmd,
            broadcast_psbt_cmd,
            export_sparrow_wallet,
            list_server_presets,
            app_version,
            save_text_file,
            open_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
