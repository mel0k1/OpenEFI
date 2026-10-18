mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::scan_hardware,
            commands::macos_versions,
            commands::compute_max_darwin,
            commands::match_kexts,
            commands::kext_catalog,
            commands::recommend_smbios,
            commands::list_smbios_models,
            commands::generate_smbios,
            commands::export_report,
            commands::recommend_ssdt,
            commands::generate_config_plist,
            commands::save_config_plist,
            commands::download_kexts_to_efi,
            commands::download_ssdts_to_efi,
            commands::download_utb_tool,
            commands::install_utbmap_to_efi,
            commands::download_driver_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
