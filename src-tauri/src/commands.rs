use openefi_core::hardware::HardwareReport;
use openefi_core::kexts::KextEntry;
use openefi_core::macos;
use openefi_core::smbios::{self, SmbiosData, SmbiosRecommendation};

#[tauri::command]
pub fn scan_hardware() -> Result<HardwareReport, String> {
    Ok(openefi_core::hardware::scan())
}

#[tauri::command]
pub fn macos_versions(max_darwin: u32) -> Vec<macos::MacosVersion> {
    macos::build_version_list(max_darwin)
}

#[tauri::command]
pub fn compute_max_darwin(report: HardwareReport) -> u32 {
    let mut max = macos::LATEST_DARWIN;
    for gpu in &report.gpus {
        if let Some(limit) = gpu.max_darwin {
            if limit > 0 && limit < max {
                max = limit;
            }
        }
    }
    if report.cpu.manufacturer == "Intel" {
        if let Some(codename) = &report.cpu.codename {
            if let Some((_, limit)) = openefi_core::hardware::MAX_DARWIN_BY_INTEL_IGPU
                .iter()
                .find(|(c, _)| c == codename)
            {
                let has_igpu = report
                    .gpus
                    .iter()
                    .any(|g| g.device_type == "Integrated GPU" && g.vendor_id == "8086");
                if has_igpu && *limit < max {
                    max = *limit;
                }
            }
        }
    }
    max
}

#[tauri::command]
pub fn match_kexts(report: HardwareReport, darwin: u32) -> Vec<KextEntry> {
    openefi_core::kexts::match_kexts(&report, darwin)
}

#[tauri::command]
pub fn kext_catalog() -> Vec<KextEntry> {
    openefi_core::kexts::kext_catalog()
}

#[tauri::command]
pub fn recommend_smbios(report: HardwareReport, darwin: u32) -> SmbiosRecommendation {
    smbios::recommend(&report, darwin)
}

#[tauri::command]
pub fn list_smbios_models(darwin: u32, is_laptop: bool) -> Vec<smbios::MacDevice> {
    smbios::compatible_models(darwin, is_laptop)
        .into_iter()
        .cloned()
        .collect()
}

#[tauri::command]
pub fn generate_smbios(model: String) -> SmbiosData {
    smbios::generate(&model)
}

#[tauri::command]
pub fn export_report(
    window: tauri::WebviewWindow,
    report: HardwareReport,
    kexts: Vec<String>,
    smbios: SmbiosData,
) -> Result<String, String> {
    let data = serde_json::json!({
        "generated_by": "OpenEFI",
        "hardware": report,
        "smbios": smbios,
        "kexts": kexts,
    });
    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;

    let path = tauri_plugin_dialog::DialogExt::dialog(&window)
        .file()
        .set_file_name("OpenEFI-report.json")
        .blocking_save_file();

    match path {
        Some(p) => {
            let target = p
                .as_path()
                .map(|q| q.to_path_buf())
                .ok_or("недопустимый путь")?;
            std::fs::write(&target, json).map_err(|e| e.to_string())?;
            Ok(target.to_string_lossy().to_string())
        }
        None => Err("Сохранение отменено".into()),
    }
}
