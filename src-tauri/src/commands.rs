use openefi_core::hardware::HardwareReport;
use openefi_core::kexts::KextEntry;
use openefi_core::macos;
use openefi_core::smbios::{self, SmbiosData, SmbiosRecommendation};
use openefi_core::config::Compensations;

use std::path::PathBuf;
use tauri::WebviewWindow;

/// База EFI: явный путь (найденный ESP) или диалог выбора папки
fn resolve_base(window: &WebviewWindow, base: Option<String>, title: &str) -> Result<PathBuf, String> {
    use tauri_plugin_dialog::DialogExt;

    if let Some(b) = base {
        let p = PathBuf::from(b);
        if p.is_dir() {
            return Ok(p);
        }
        return Err("найденный EFI-раздел недоступен — выберите папку вручную".into());
    }
    let picked = DialogExt::dialog(window)
        .file()
        .set_title(title)
        .blocking_pick_folder();
    let Some(picked) = picked else {
        return Err("Выбор папки отменён".into());
    };
    picked
        .as_path()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "недопустимый путь к папке".into())
}

#[tauri::command]
pub fn scan_hardware() -> Result<HardwareReport, String> {
    Ok(openefi_core::hardware::scan())
}

/// Матрица настроек BIOS/UEFI под железо пользователя
#[tauri::command]
pub fn bios_checklist(report: HardwareReport) -> openefi_core::bios::BiosChecklist {
    openefi_core::bios::checklist(&report)
}

/// Живая проверка Secure Boot и режима загрузки (Windows/Linux)
#[tauri::command]
pub fn bios_live_status() -> openefi_core::bios::BiosStatus {
    openefi_core::bios::live_status()
}

/// Аудит EFI-папки: чужие загрузчики, состояние OpenCore, предупреждения
#[tauri::command]
pub fn audit_efi_folder(window: tauri::WebviewWindow, base: Option<String>) -> Result<openefi_core::efiprotect::EfiAudit, String> {
    let base = resolve_base(&window, base, "Выберите EFI-папку или диск с EFI для проверки")?;
    Ok(openefi_core::efiprotect::audit(&base))
}

/// Ручной полный бэкап EFI/OC → EFI/OC.bak-openefi-<метка времени>
#[tauri::command]
pub fn backup_efi_folder(window: tauri::WebviewWindow, base: Option<String>) -> Result<openefi_core::efiprotect::BackupReport, String> {
    let base = resolve_base(&window, base, "Выберите EFI-папку или диск с EFI для бэкапа")?;
    openefi_core::efiprotect::backup_oc(&base)
}

/// Аудит существующего config.plist: Kernel/Add и ACPI/Add против файлов на диске
#[tauri::command]
pub fn audit_config_plist(window: tauri::WebviewWindow, base: Option<String>) -> Result<openefi_core::configaudit::ConfigAudit, String> {
    let base = resolve_base(&window, base, "Выберите EFI-папку или диск с EFI для проверки config.plist")?;
    openefi_core::configaudit::audit_config(&base)
}

/// Установленные кекты + проверка обновлений на GitHub
#[tauri::command]
pub fn check_kext_updates(window: tauri::WebviewWindow, base: Option<String>) -> Result<openefi_core::kextupdate::KextUpdatesReport, String> {
    let base = resolve_base(&window, base, "Выберите EFI-папку или диск с EFI для проверки обновлений кекстов")?;
    let installed = openefi_core::kextupdate::scan_installed(&base)?;
    let checks = openefi_core::kextupdate::check_updates(&installed);
    Ok(openefi_core::kextupdate::KextUpdatesReport { installed, checks })
}

/// Обновить выбранные устаревшие кекты тем же конвейером, что и установка
#[tauri::command]
pub fn update_outdated_kexts(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    base: Option<String>,
    bundles: Vec<String>,
    darwin: u32,
) -> Result<openefi_core::downloader::KextsToEfiReport, String> {
    use tauri::Emitter;

    let base = resolve_base(&window, base, "Выберите EFI-папку или диск с EFI для обновления кекстов")?;
    openefi_core::kextupdate::update_kexts(&bundles, &base, darwin, |pr| {
        let _ = app.emit("kext-dl-progress", pr);
    })
}

/// Автопоиск EFI-разделов (ESP) на этой машине
#[tauri::command]
pub fn find_esp_partitions() -> Vec<openefi_core::espdetect::EfiPartition> {
    openefi_core::espdetect::find_esp()
}

/// Смонтировать EFI-раздел и вернуть точку монтирования
#[tauri::command]
pub fn mount_esp_partition(device: String) -> Result<String, String> {
    openefi_core::espdetect::mount_esp(&device)
}

/// Состояние установленного OpenCore: версия (NVRAM/лог) + последняя на GitHub
#[tauri::command]
pub fn opencore_status(window: tauri::WebviewWindow, base: Option<String>) -> Result<openefi_core::opencore::OpenCoreStatus, String> {
    let base = resolve_base(&window, base, "Выберите EFI-папку или диск с EFI")?;
    Ok(openefi_core::opencore::detect_opencore_status(&base))
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

#[tauri::command]
pub fn recommend_ssdt(report: HardwareReport) -> Vec<openefi_core::ssdts::SsdtEntry> {
    openefi_core::ssdts::recommend(&report)
}

#[tauri::command]
pub fn generate_config_plist(
    report: HardwareReport,
    kexts: Vec<openefi_core::kexts::KextEntry>,
    ssdts: Vec<openefi_core::ssdts::SsdtEntry>,
    smbios: SmbiosData,
    darwin: u32,
    compensations: Option<Compensations>,
) -> Result<String, String> {
    let input = openefi_core::config::ConfigInput {
        report: &report,
        kexts: &kexts,
        ssdts: &ssdts,
        smbios: &smbios,
        darwin,
        compensations: compensations.unwrap_or_default(),
    };
    let bytes = openefi_core::config::generate_config(&input)?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_config_plist(
    window: tauri::WebviewWindow,
    report: HardwareReport,
    kexts: Vec<openefi_core::kexts::KextEntry>,
    ssdts: Vec<openefi_core::ssdts::SsdtEntry>,
    smbios: SmbiosData,
    darwin: u32,
    compensations: Option<Compensations>,
) -> Result<String, String> {
    let xml = generate_config_plist(report, kexts, ssdts, smbios, darwin, compensations)?;
    let path = tauri_plugin_dialog::DialogExt::dialog(&window)
        .file()
        .set_file_name("config.plist")
        .add_filter("Plist конфиг OpenCore", &["plist"])
        .blocking_save_file();

    match path {
        Some(p) => {
            let target = p
                .as_path()
                .map(|q| q.to_path_buf())
                .ok_or("недопустимый путь")?;
            std::fs::write(&target, xml).map_err(|e| e.to_string())?;
            Ok(target.to_string_lossy().to_string())
        }
        None => Err("Сохранение отменено".into()),
    }
}

#[tauri::command]
pub fn download_kexts_to_efi(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    kexts: Vec<openefi_core::downloader::KextDownloadItem>,
    darwin: u32,
    base: Option<String>,
) -> Result<openefi_core::downloader::KextsToEfiReport, String> {
    use tauri::Emitter;

    let base = resolve_base(&window, base, "Выберите EFI-папку или диск с EFI")?;
    openefi_core::downloader::download_kexts(&kexts, &base, darwin, |pr| {
        let _ = app.emit("kext-dl-progress", pr);
    })
}

/// Скачать выбранные SSDT (.aml) из репозитория Dortania прямо в EFI/OC/ACPI.
/// Если в EFI/OC найден config.plist — таблицы включаются в ACPI/Add (с бэкапом).
#[tauri::command]
pub fn download_ssdts_to_efi(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    ssdts: Vec<openefi_core::downloader::SsdtDownloadItem>,
    base: Option<String>,
) -> Result<openefi_core::downloader::SsdtsToEfiReport, String> {
    use tauri::Emitter;

    let base = resolve_base(&window, base, "Выберите EFI-папку или диск с EFI")?;
    openefi_core::downloader::download_ssdts(&ssdts, &base, |pr| {
        let _ = app.emit("ssdt-dl-progress", pr);
    })
}

/// Сборка полной EFI одной кнопкой / обновление OpenCore: OpenCorePkg + OcBinaryData.
/// Пишет только внутрь EFI/OC и EFI/BOOT; чужой BOOTX64.EFI не трогается.
#[tauri::command]
pub fn build_full_efi_to_efi(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    base: Option<String>,
    tools: Option<bool>,
    audio_dxe: Option<bool>,
) -> Result<openefi_core::opencore::FullEfiReport, String> {
    use tauri::Emitter;

    let base = resolve_base(&window, base, "Выберите EFI-папку или диск, куда собрать полную EFI")?;
    let options = openefi_core::opencore::FullEfiOptions {
        tools: tools.unwrap_or(false),
        audio_dxe: audio_dxe.unwrap_or(false),
    };
    openefi_core::opencore::build_full_efi(&base, options, |pr| {
        let _ = app.emit("fullefi-progress", pr);
    })
}

/// Скачать утилиту USBToolBox (windows|macos) через диалог сохранения
#[tauri::command]
pub fn download_utb_tool(window: tauri::WebviewWindow, target: String) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;

    let default_name = match target.as_str() {
        "windows" => "USBToolBox-Windows.exe",
        "macos" => "USBToolBox-macOS.zip",
        other => return Err(format!("неизвестная цель утилиты: {other}")),
    };
    let picked = DialogExt::dialog(&window)
        .file()
        .set_file_name(default_name)
        .blocking_save_file();

    let Some(picked) = picked else {
        return Err("Сохранение отменено".into());
    };
    let dest = picked
        .as_path()
        .map(|p| p.to_path_buf())
        .ok_or("недопустимый путь")?;

    openefi_core::downloader::download_utb_tool(&target, &dest)?;
    Ok(dest.to_string_lossy().to_string())
}

/// Установить сгенерированный UTBMap.kext в EFI: выбор папки с кекстом,
/// выбор EFI-папки, копирование в OC/Kexts и правка config.plist
#[tauri::command]
pub fn install_utbmap_to_efi(
    window: tauri::WebviewWindow,
    base: Option<String>,
) -> Result<openefi_core::usbmap::UtbMapReport, String> {
    use tauri_plugin_dialog::DialogExt;

    let picked_kext = DialogExt::dialog(&window)
        .file()
        .set_title("Выберите папку UTBMap.kext или папку вывода USBToolBox")
        .blocking_pick_folder();
    let Some(picked_kext) = picked_kext else {
        return Err("Выбор папки отменён".into());
    };
    let kext_path = picked_kext
        .as_path()
        .map(|p| p.to_path_buf())
        .ok_or("недопустимый путь к кексту")?;

    let efi_path = resolve_base(&window, base, "Выберите EFI-папку или диск с EFI")?;

    openefi_core::usbmap::install_utbmap(&kext_path, &efi_path)
}

/// Скачать файл из последнего релиза репозитория по расширению (для системных драйверов)
#[tauri::command]
pub fn download_driver_file(
    window: tauri::WebviewWindow,
    repo: String,
    ext: String,
) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;

    if repo.matches('/').count() != 1 {
        return Err("некорректный репозиторий".into());
    }
    let ext_norm = ext.to_lowercase();
    // сначала узнаём имя ассета — чтобы предложить вменяемое имя файла
    let asset = openefi_core::downloader::pick_release_asset(&repo, &ext_norm)?;

    let picked = DialogExt::dialog(&window)
        .file()
        .set_file_name(&asset.name)
        .blocking_save_file();

    let Some(picked) = picked else {
        return Err("Сохранение отменено".into());
    };
    let dest = picked
        .as_path()
        .map(|p| p.to_path_buf())
        .ok_or("недопустимый путь")?;

    openefi_core::downloader::download_to_file_url(&asset.browser_download_url, &dest)?;
    Ok(dest.to_string_lossy().to_string())
}
