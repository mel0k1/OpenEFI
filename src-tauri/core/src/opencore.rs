//! Сборка полной EFI одной кнопкой: скачивание релиза OpenCorePkg
//! (BOOTx64.efi, OpenCore.efi, OpenRuntime.efi, OpenCanopy.efi, Sample.plist)
//! и OcBinaryData (HfsPlus.efi + вспомогательные драйверы + Resources для
//! графического пикера), раскладка файлов по папкам и регистрация драйверов
//! в UEFI/Drivers конфига.
//!
//! Политика защиты EFI (см. efiprotect.rs):
//! - BOOTX64.EFI пишется только если его ещё нет или он уже является копией
//!   OpenCore (байты совпадают с EFI/OC/OpenCore.efi либо с новой версией);
//!   чужой fallback-загрузчик (Windows, GRUB, shim) никогда не перезаписывается;
//! - всё остальное пишется только внутрь EFI/OC;
//! - перед изменениями уже установленного EFI/OC снимается полная копия.

use std::collections::HashSet;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use plist::{Dictionary, Value};
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::downloader::{
    download_to_file, download_to_file_limited, efi_safety, http_agent, http_agent_no_redirects,
};

/// GUID переменных NVRAM OpenCore (opencore-version и др.)
const OC_VENDOR_GUID_EFIVAR: &str = "opencore-version-4d1ede05-38c7-4a6a-9cc6-4bcca8b38c14";
/// Резервный GUID Apple (встречается в сторонних инструментах)
const OC_APPLE_GUID_EFIVAR: &str = "opencore-version-7c436110-ab2a-4bbb-a880-fe41995c9f82";
/// Аудио-файлы из OcBinaryData, реально нужные пикеру (без всех языков)
const AUDIO_FILES: &[&str] = &[
    "AXEFIAudio_Boot.mp3",
    "AXEFIAudio_Beep.mp3",
    "AXEFIAudio_Click.mp3",
    "AXEFIAudio_VoiceOver_Boot.mp3",
];

pub const OC_PKG_REPO: &str = "acidanthera/OpenCorePkg";
pub const OC_PKG_LATEST_URL: &str = "https://github.com/acidanthera/OpenCorePkg/releases/latest";
pub const OC_BINARY_DATA_URL: &str =
    "https://codeload.github.com/acidanthera/OcBinaryData/zip/refs/heads/master";

/// Лимит для архива OcBinaryData (весь репозиторий с Resources) — заметно
/// крупнее кекстовых ZIP, но ограничен от абсурдных размеров.
const MAX_OCBINARY_BYTES: u64 = 512 * 1024 * 1024;
/// Бюджет на распаковку Resources из OcBinaryData.
const MAX_RESOURCES_BYTES: u64 = 512 * 1024 * 1024;

/// Прогресс сборки (событие «fullefi-progress»)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BuildProgress {
    /// Этап: prepare | oc_info | oc_download | oc_extract | data_download |
    /// data_extract | config | done
    pub stage: String,
    /// Человекочитаемое сообщение (переводится tb())
    pub message: String,
    /// 0..=100
    pub percent: u8,
}

/// Что произошло с EFI/BOOT/BOOTX64.EFI
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Bootx64Outcome {
    /// Файла не было — установлен
    Installed,
    /// Был старый OpenCore (байты совпали с OpenCore.efi) — обновлён
    Updated,
    /// Уже совпадает с новой версией — не трогали
    SkippedSame,
    /// Принадлежит другому загрузчику — не тронут (защита EFI)
    SkippedForeign,
}

/// Опции сборки/обновления полной EFI
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FullEfiOptions {
    /// Установить OpenShell.efi в OC/Tools и зарегистрировать в Misc/Tools
    #[serde(default)]
    pub tools: bool,
    /// Установить AudioDxe.efi + аудиоресурсы (звук в пикере, boot chime)
    #[serde(default)]
    pub audio_dxe: bool,
}

/// Итоговый отчёт сборки полной EFI
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FullEfiReport {
    pub efi_dir: String,
    pub oc_dir: String,
    /// Версия OpenCore из тега релиза (например «1.0.4»)
    pub opencore_version: Option<String>,
    /// Версия установленного OpenCore до обновления (NVRAM/лог-файл, best-effort)
    #[serde(default)]
    pub previous_version: Option<String>,
    /// Установленный OpenCore.efi уже совпадал с новой версией (по байтам)
    #[serde(default)]
    pub was_up_to_date: Option<bool>,
    /// Относительные пути установленных файлов (от корня EFI)
    pub files_installed: Vec<String>,
    /// Что стало с EFI/BOOT/BOOTX64.EFI
    pub bootx64: Bootx64Outcome,
    /// Пояснение по BOOTX64 (пусто, если всё однозначно)
    pub bootx64_note: Option<String>,
    /// existing | sample
    pub config_source: String,
    /// Путь к config.plist (если он есть после сборки)
    pub config_path: Option<String>,
    /// Драйверы зарегистрированы/включены в UEFI/Drivers
    pub config_patched: bool,
    /// Бэкап config.plist (.bak-openefi) сделан
    pub config_backup: bool,
    /// Графический пикер включён (OpenCanopy + PickerMode=External)
    pub canopy_enabled: bool,
    /// Папки Resources, скопированные из OcBinaryData (Font/Label/Image)
    pub resources_installed: Vec<String>,
    /// Инструменты, установленные в OC/Tools (например OpenShell.efi)
    #[serde(default)]
    pub tools_installed: Vec<String>,
    /// AudioDxe.efi + аудиоресурсы установлены
    #[serde(default)]
    pub audio_installed: bool,
    /// Куда сделан автоматический бэкап EFI/OC перед изменениями
    pub backup_path: Option<String>,
    /// Предупреждения защиты EFI (нет OpenCore, не похоже на ESP и т.п.)
    pub audit_warnings: Vec<String>,
    /// Предупреждения самой сборки (чужой BOOTX64, Sample-конфиг и т.п.)
    pub warnings: Vec<String>,
}

/// Состояние установленного OpenCore (для кнопки «Обновить OpenCore»)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OpenCoreStatus {
    /// OpenCore.efi или config.plist найден в EFI/OC
    pub installed: bool,
    /// Версия установленного OpenCore (best-effort: NVRAM/лог)
    pub version: Option<String>,
    /// Последняя версия на GitHub (redirect releases/latest)
    pub latest: Option<String>,
    /// Как OpenEFI определил версию (nvram | log | none) — для подсказки
    #[serde(default)]
    pub version_source: String,
}

/// Результат раскладки файлов из архива OpenCorePkg
struct OpenCoreInstall {
    files: Vec<String>,
    bootx64: Bootx64Outcome,
    bootx64_note: Option<String>,
    canopy: bool,
    sample_plist: Option<Vec<u8>>,
}

fn temp_zip(label: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("openefi-{label}-{}-{ts}.zip", std::process::id()))
}

fn open_zip(path: &Path) -> Result<ZipArchive<std::io::BufReader<std::fs::File>>, String> {
    let f = std::fs::File::open(path).map_err(|e| format!("не открыть скачанный архив: {e}"))?;
    ZipArchive::new(std::io::BufReader::new(f)).map_err(|e| format!("битый ZIP-архив: {e}"))
}

/// Индекс записи в архиве по хвосту нормализованного пути (регистр не важен)
fn find_entry<R: Read + Seek>(ar: &mut ZipArchive<R>, needle: &str) -> Option<usize> {
    (0..ar.len()).find(|&i| {
        ar.by_index_raw(i)
            .map(|f| f.name().replace('\\', "/").to_lowercase().ends_with(needle))
            .unwrap_or(false)
    })
}

fn entry_bytes<R: Read + Seek>(ar: &mut ZipArchive<R>, index: usize) -> Result<Vec<u8>, String> {
    let mut f = ar.by_index(index).map_err(|e| format!("битый ZIP: {e}"))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)
        .map_err(|e| format!("ошибка чтения из архива: {e}"))?;
    Ok(buf)
}

fn write_file(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("не создать {}: {e}", parent.display()))?;
    }
    std::fs::write(dest, bytes).map_err(|e| format!("не записать {}: {e}", dest.display()))
}

/// Найти существующий EFI/BOOT/BOOTX64.EFI (регистр имени не важен)
fn find_bootx64(efi_base: &Path) -> Option<PathBuf> {
    let rd = std::fs::read_dir(efi_base).ok()?;
    let boot_dir = rd
        .flatten()
        .find(|e| e.file_name().to_string_lossy().to_lowercase() == "boot")
        .map(|e| e.path())?;
    let rd = std::fs::read_dir(&boot_dir).ok()?;
    rd.flatten()
        .find(|e| {
            let n = e.file_name().to_string_lossy().to_lowercase();
            n == "bootx64.efi" || n == "bootx64"
        })
        .map(|e| e.path())
}

/// Раскладка файлов OpenCore из архива релиза в EFI.
/// Сравнение BOOTX64 с СТАРЫМ OpenCore.efi — до перезаписи OpenCore.efi.
fn install_opencore_files<R: Read + Seek>(
    ar: &mut ZipArchive<R>,
    efi_base: &Path,
) -> Result<OpenCoreInstall, String> {
    let mut files: Vec<String> = Vec::new();
    let mut warnings_note: Option<String> = None;

    // --- BOOTX64.EFI: только по политике защиты ---
    let boot_idx = find_entry(ar, "x64/efi/boot/bootx64.efi")
        .ok_or("в архиве OpenCorePkg нет X64/EFI/BOOT/BOOTx64.efi")?;
    let new_boot = entry_bytes(ar, boot_idx)?;

    let bootx64 = match find_bootx64(efi_base) {
        None => {
            write_file(&efi_base.join("BOOT").join("BOOTX64.EFI"), &new_boot)?;
            files.push("BOOT/BOOTX64.EFI".into());
            Bootx64Outcome::Installed
        }
        Some(existing) => {
            let cur = std::fs::read(&existing).unwrap_or_default();
            if cur == new_boot {
                Bootx64Outcome::SkippedSame
            } else {
                let old_oc = efi_base.join("OC").join("OpenCore.efi");
                let is_old_oc =
                    old_oc.is_file() && std::fs::read(&old_oc).map(|b| b == cur).unwrap_or(false);
                if is_old_oc {
                    write_file(&existing, &new_boot)?;
                    files.push("BOOT/BOOTX64.EFI".into());
                    Bootx64Outcome::Updated
                } else {
                    warnings_note = Some(
                        "EFI/BOOT/BOOTX64.EFI принадлежит другому загрузчику — OpenEFI его не тронул. \
                         Запускайте OpenCore через EFI/OC/OpenCore.efi (выберите его в BIOS/NVRAM или \
                         назначьте BootNext)"
                            .into(),
                    );
                    Bootx64Outcome::SkippedForeign
                }
            }
        }
    };

    // --- Ядро OpenCore: OpenCore.efi + OpenRuntime.efi (одной версии) ---
    let oc_idx = find_entry(ar, "x64/efi/oc/opencore.efi")
        .ok_or("в архиве OpenCorePkg нет X64/EFI/OC/OpenCore.efi")?;
    let oc_bytes = entry_bytes(ar, oc_idx)?;
    write_file(&efi_base.join("OC").join("OpenCore.efi"), &oc_bytes)?;
    files.push("OC/OpenCore.efi".into());

    // OpenRuntime: в новых релизах лежит в OC/Drivers, в старых — прямо в OC
    let rt_needles = [
        ("x64/efi/oc/drivers/openruntime.efi", "OC/Drivers/OpenRuntime.efi"),
        ("x64/efi/oc/openruntime.efi", "OC/OpenRuntime.efi"),
    ];
    let mut rt_installed = false;
    for (needle, rel) in rt_needles {
        if let Some(idx) = find_entry(ar, needle) {
            let bytes = entry_bytes(ar, idx)?;
            let dest = efi_base.join("OC").join(rel.trim_start_matches("OC/"));
            write_file(&dest, &bytes)?;
            files.push(rel.into());
            rt_installed = true;
            break;
        }
    }
    if !rt_installed {
        return Err("в архиве OpenCorePkg нет OpenRuntime.efi".into());
    }

    // --- Маркеры .contentFlavour/.contentVisibility (рекомендация Dortania) ---
    for (needle, fname) in [
        ("x64/efi/boot/.contentflavour", ".contentFlavour"),
        ("x64/efi/boot/.contentvisibility", ".contentVisibility"),
    ] {
        if let Some(idx) = find_entry(ar, needle) {
            let bytes = entry_bytes(ar, idx)?;
            write_file(&efi_base.join("BOOT").join(fname), &bytes)?;
            files.push(format!("BOOT/{fname}"));
        }
    }

    // --- Драйверы-записи пикера из того же пакета (опциональные) ---
    let mut canopy = false;
    for (needle, fname) in [
        ("x64/efi/oc/drivers/opencanopy.efi", "OpenCanopy.efi"),
        ("x64/efi/oc/drivers/resetnvramentry.efi", "ResetNvramEntry.efi"),
        ("x64/efi/oc/drivers/togglesipentry.efi", "ToggleSipEntry.efi"),
    ] {
        if let Some(idx) = find_entry(ar, needle) {
            let bytes = entry_bytes(ar, idx)?;
            write_file(&efi_base.join("OC").join("Drivers").join(fname), &bytes)?;
            files.push(format!("OC/Drivers/{fname}"));
            if fname == "OpenCanopy.efi" {
                canopy = true;
            }
        }
    }

    // --- Sample.plist как fallback-конфиг (используется, только если своего нет) ---
    let sample_plist = match find_entry(ar, "docs/sample.plist") {
        Some(idx) => Some(entry_bytes(ar, idx)?),
        None => None,
    };

    Ok(OpenCoreInstall {
        files,
        bootx64,
        bootx64_note: warnings_note,
        canopy,
        sample_plist,
    })
}

/// Раскладка HfsPlus.efi + Resources из OcBinaryData.
/// Драйверы-записи пикера берутся из пакета OpenCorePkg той же версии;
/// OcBinaryData нужен только для HfsPlus (закрытый драйвер) и Resources.
/// `with_audio` — также положить аудиофайлы для AudioDxe (boot chime).
fn install_ocbinarydata<R: Read + Seek>(
    ar: &mut ZipArchive<R>,
    efi_base: &Path,
    with_audio: bool,
) -> Result<(Vec<String>, Vec<String>), String> {
    let drivers_dir = efi_base.join("OC").join("Drivers");
    std::fs::create_dir_all(&drivers_dir)
        .map_err(|e| format!("не создать {}: {e}", drivers_dir.display()))?;

    let mut files: Vec<String> = Vec::new();

    // HfsPlus обязателен — без него не увидеть раздел с macOS
    let hfs_idx = find_entry(ar, "drivers/hfsplus.efi")
        .ok_or("в архиве OcBinaryData нет Drivers/HfsPlus.efi — структура репозитория изменилась?")?;
    let hfs = entry_bytes(ar, hfs_idx)?;
    write_file(&drivers_dir.join("HfsPlus.efi"), &hfs)?;
    files.push("OC/Drivers/HfsPlus.efi".into());

    // Resources для графического пикера: Font, Label, Image (Tool не нужен —
    // не копируем, чтобы не раздувать EFI); Audio копируем только с AudioDxe
    let audio_wanted: HashSet<String> = if with_audio {
        AUDIO_FILES.iter().map(|s| s.to_lowercase()).collect()
    } else {
        HashSet::new()
    };
    let mut resources: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut written: u64 = 0;
    for i in 0..ar.len() {
        let mut f = ar.by_index(i).map_err(|e| format!("битый ZIP: {e}"))?;
        let name = f.name().replace('\\', "/");
        let lc = name.to_lowercase();
        let pos = if let Some(p) = lc.find("/resources/") {
            p + "/resources/".len()
        } else if lc.starts_with("resources/") {
            "resources/".len()
        } else {
            continue;
        };
        let rel = &name[pos..];
        if rel.is_empty() {
            continue;
        }
        let sub = rel.split('/').next().unwrap_or("").to_string();
        let sub_lc = sub.to_lowercase();
        let allowed = if sub_lc == "audio" {
            if !with_audio {
                continue;
            }
            // только базовые файлы звука, без всех языков
            let file_lc = rel.rsplit('/').next().unwrap_or("").to_lowercase();
            audio_wanted.contains(&file_lc)
        } else {
            matches!(sub_lc.as_str(), "font" | "label" | "image")
        };
        if !allowed {
            continue;
        }
        if rel.split('/').any(|c| c == "..") {
            continue; // zip-slip
        }
        let dest = efi_base.join("OC").join("Resources").join(rel);
        if f.is_dir() {
            std::fs::create_dir_all(&dest)
                .map_err(|e| format!("не создать {}: {e}", dest.display()))?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("не создать {}: {e}", parent.display()))?;
        }
        written += f.size();
        if written > MAX_RESOURCES_BYTES {
            return Err("Resources подозрительно большие (zip-bomb?)".into());
        }
        let mut out = std::fs::File::create(&dest)
            .map_err(|e| format!("не записать {}: {e}", dest.display()))?;
        std::io::copy(&mut f, &mut out)
            .map_err(|e| format!("ошибка записи {}: {e}", dest.display()))?;
        if seen.insert(sub.clone()) {
            resources.push(sub);
        }
    }
    resources.sort();

    Ok((files, resources))
}

/// Включить существующие и добавить недостающие записи UEFI/Drivers.
/// При установке OpenCanopy — переключить пикер на External.
/// Возвращает (изменён ли конфиг, сделан ли бэкап).
fn patch_uefi_drivers(
    config: &Path,
    drivers: &[String],
    picker_external: bool,
) -> Result<(bool, bool), String> {
    let mut root = Value::from_reader(std::io::BufReader::new(
        std::fs::File::open(config).map_err(|e| format!("не открыть {}: {e}", config.display()))?,
    ))
    .map_err(|e| format!("не разобрать {}: {e}", config.display()))?
    .into_dictionary()
    .ok_or_else(|| format!("{} — не словарь plist", config.display()))?;

    let Some(uefi) = root.get_mut("UEFI").and_then(|v| v.as_dictionary_mut()) else {
        return Err("в config.plist нет секции UEFI".into());
    };
    let drivers_arr = match uefi.get_mut("Drivers") {
        Some(v) if matches!(v, Value::Array(_)) => v,
        Some(_) => return Err("UEFI → Drivers в config.plist — не массив".into()),
        None => {
            uefi.insert("Drivers".into(), Value::Array(Vec::new()));
            uefi.get_mut("Drivers").ok_or("не создать UEFI → Drivers")?
        }
    };
    let entries = drivers_arr.as_array_mut().unwrap();

    let mut changed = false;
    for drv in drivers {
        let d_lc = drv.to_lowercase();
        let mut found = false;
        for e in entries.iter_mut() {
            let Some(d) = e.as_dictionary_mut() else { continue };
            let matches = d
                .get("Path")
                .and_then(|p| p.as_string())
                .map(|p| {
                    let p = p.replace('\\', "/").to_lowercase();
                    p == d_lc || p == format!("drivers/{d_lc}") || p.ends_with(&format!("/{d_lc}"))
                })
                .unwrap_or(false);
            if matches {
                if !d.get("Enabled").and_then(|b| b.as_boolean()).unwrap_or(false) {
                    d.insert("Enabled".into(), Value::Boolean(true));
                    changed = true;
                }
                found = true;
                break;
            }
        }
        if !found {
            let mut d = Dictionary::new();
            d.insert("Arguments".into(), Value::String(String::new()));
            d.insert("Comment".into(), Value::String(String::new()));
            d.insert("Enabled".into(), Value::Boolean(true));
            d.insert("HideVerbose".into(), Value::Boolean(false));
            d.insert("LoadEarly".into(), Value::Boolean(false));
            d.insert("Path".into(), Value::String(drv.clone()));
            entries.push(Value::Dictionary(d));
            changed = true;
        }
    }

    // Графический пикер: OpenCanopy требует External (PickerAttributes=17 уже в сэмпле)
    if picker_external {
        if let Some(boot) = root
            .get_mut("Misc")
            .and_then(|v| v.as_dictionary_mut())
            .and_then(|m| m.get_mut("Boot"))
            .and_then(|v| v.as_dictionary_mut())
        {
            if boot.get("PickerMode").and_then(|p| p.as_string()) != Some("External") {
                boot.insert("PickerMode".into(), Value::String("External".into()));
                changed = true;
            }
        }
    }

    if !changed {
        return Ok((false, false));
    }

    // резервная копия до записи
    let backup = config.with_extension("plist.bak-openefi");
    std::fs::copy(config, &backup)
        .map_err(|e| format!("не сделать копию {}: {e}", backup.display()))?;

    let mut buf = Vec::new();
    Value::Dictionary(root)
        .to_writer_xml(&mut buf)
        .map_err(|e| format!("не сериализовать {}: {e}", config.display()))?;
    std::fs::write(config, buf).map_err(|e| format!("не записать {}: {e}", config.display()))?;
    Ok((true, true))
}

/// Версия OpenCore из redirect-заголовка releases/latest (без GitHub API —
/// у того лимит 60 запросов/час, у прямых ссылок лимитов нет).
/// Location: https://github.com/acidanthera/OpenCorePkg/releases/tag/1.0.4
pub fn fetch_opencore_version(agent: &ureq::Agent) -> Result<String, String> {
    // redirects(0): 302 возвращается как обычный ответ с заголовком Location
    let resp = agent
        .get(OC_PKG_LATEST_URL)
        .call()
        .map_err(|_| "нет связи с GitHub (OpenCorePkg)".to_string())?;
    let location = resp
        .header("location")
        .ok_or("GitHub не вернул Location для releases/latest")?
        .to_string();
    let tag = location.rsplit('/').next().unwrap_or("").trim().to_string();
    if tag.is_empty() || !tag.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return Err(format!("не определить версию OpenCore из redirect: {location}"));
    }
    Ok(tag)
}

/// Установить OpenShell.efi из архива OpenCorePkg в EFI/OC/Tools.
/// Возвращает относительный путь или None, если в архиве нет Tools.
fn install_tools<R: Read + Seek>(
    ar: &mut ZipArchive<R>,
    efi_base: &Path,
) -> Result<Option<String>, String> {
    let Some(idx) = find_entry(ar, "x64/efi/oc/tools/openshell.efi") else {
        return Ok(None);
    };
    let bytes = entry_bytes(ar, idx)?;
    write_file(&efi_base.join("OC").join("Tools").join("OpenShell.efi"), &bytes)?;
    Ok(Some("OC/Tools/OpenShell.efi".into()))
}

/// Включить существующие и добавить недостающие записи Misc/Tools.
/// Возвращает (изменён ли конфиг, сделан ли бэкап).
fn patch_misc_tools(config: &Path, tools: &[String]) -> Result<(bool, bool), String> {
    let mut root = Value::from_reader(std::io::BufReader::new(
        std::fs::File::open(config).map_err(|e| format!("не открыть {}: {e}", config.display()))?,
    ))
    .map_err(|e| format!("не разобрать {}: {e}", config.display()))?
    .into_dictionary()
    .ok_or_else(|| format!("{} — не словарь plist", config.display()))?;

    let Some(misc) = root.get_mut("Misc").and_then(|v| v.as_dictionary_mut()) else {
        return Err("в config.plist нет секции Misc".into());
    };
    let tools_arr = match misc.get_mut("Tools") {
        Some(v) if matches!(v, Value::Array(_)) => v,
        Some(_) => return Err("Misc → Tools в config.plist — не массив".into()),
        None => {
            misc.insert("Tools".into(), Value::Array(Vec::new()));
            misc.get_mut("Tools").ok_or("не создать Misc → Tools")?
        }
    };
    let entries = tools_arr.as_array_mut().unwrap();

    let mut changed = false;
    for tool in tools {
        let t_lc = tool.to_lowercase();
        let name = tool.rsplit('/').next().unwrap_or(tool);
        let mut found = false;
        for e in entries.iter_mut() {
            let Some(d) = e.as_dictionary_mut() else { continue };
            let matches = d
                .get("Path")
                .and_then(|p| p.as_string())
                .map(|p| {
                    let p = p.replace('\\', "/").to_lowercase();
                    p == t_lc || p == format!("tools/{t_lc}") || p.ends_with(&format!("/{t_lc}"))
                })
                .unwrap_or(false);
            if matches {
                if !d.get("Enabled").and_then(|b| b.as_boolean()).unwrap_or(false) {
                    d.insert("Enabled".into(), Value::Boolean(true));
                    changed = true;
                }
                found = true;
                break;
            }
        }
        if !found {
            let mut d = Dictionary::new();
            d.insert("Arguments".into(), Value::String(String::new()));
            d.insert("Auxiliary".into(), Value::Boolean(true));
            d.insert("Comment".into(), Value::String(String::new()));
            d.insert("Enabled".into(), Value::Boolean(true));
            d.insert("Flavour".into(), Value::String("Auto".into()));
            d.insert("Name".into(), Value::String(name.to_string()));
            d.insert("Path".into(), Value::String(tool.clone()));
            d.insert("RealPath".into(), Value::Boolean(false));
            d.insert("TextMode".into(), Value::Boolean(false));
            entries.push(Value::Dictionary(d));
            changed = true;
        }
    }

    if !changed {
        return Ok((false, false));
    }

    // резервная копия до записи
    let backup = config.with_extension("plist.bak-openefi");
    std::fs::copy(config, &backup)
        .map_err(|e| format!("не сделать копию {}: {e}", backup.display()))?;

    let mut buf = Vec::new();
    Value::Dictionary(root)
        .to_writer_xml(&mut buf)
        .map_err(|e| format!("не сериализовать {}: {e}", config.display()))?;
    std::fs::write(config, buf).map_err(|e| format!("не записать {}: {e}", config.display()))?;
    Ok((true, true))
}

/// Включить аудио пикера (UEFI → Audio → AudioSupport) для AudioDxe
fn patch_audio_support(config: &Path) -> Result<(bool, bool), String> {
    let mut root = Value::from_reader(std::io::BufReader::new(
        std::fs::File::open(config).map_err(|e| format!("не открыть {}: {e}", config.display()))?,
    ))
    .map_err(|e| format!("не разобрать {}: {e}", config.display()))?
    .into_dictionary()
    .ok_or_else(|| format!("{} — не словарь plist", config.display()))?;

    let Some(uefi) = root.get_mut("UEFI").and_then(|v| v.as_dictionary_mut()) else {
        return Err("в config.plist нет секции UEFI".into());
    };
    if !matches!(uefi.get("Audio"), Some(Value::Dictionary(_))) {
        uefi.insert("Audio".into(), Value::Dictionary(Dictionary::new()));
    }
    let Some(audio) = uefi.get_mut("Audio").and_then(|v| v.as_dictionary_mut()) else {
        return Err("не создать UEFI → Audio".into());
    };
    let already = audio
        .get("AudioSupport")
        .and_then(|v| v.as_boolean())
        .unwrap_or(false);
    if already {
        return Ok((false, false));
    }
    audio.insert("AudioSupport".into(), Value::Boolean(true));

    let backup = config.with_extension("plist.bak-openefi");
    std::fs::copy(config, &backup)
        .map_err(|e| format!("не сделать копию {}: {e}", backup.display()))?;

    let mut buf = Vec::new();
    Value::Dictionary(root)
        .to_writer_xml(&mut buf)
        .map_err(|e| format!("не сериализовать {}: {e}", config.display()))?;
    std::fs::write(config, buf).map_err(|e| format!("не записать {}: {e}", config.display()))?;
    Ok((true, true))
}

// ---------------- Определение версии установленного OpenCore ----------------

/// Валидная ли строка похожа на версию OC («1.0.5», «0.9.7»)
fn plausible_oc_version(s: &str) -> Option<String> {
    let s = s.trim().trim_matches(char::is_control).trim();
    let mut parts = s.split('.');
    let first = parts.next()?.trim();
    let second = parts.next()?.trim();
    let third = parts.next()?.trim();
    if parts.next().is_some() {
        return None;
    }
    let (a, b, c) = (first.parse::<u32>().ok()?, second.parse::<u32>().ok()?, third.parse::<u32>().ok()?);
    // OpenCore был 0.5.x–0.9.x, сейчас 1.x — a ∈ 0..=9
    if a > 9 || b > 99 || c > 99 {
        return None;
    }
    Some(s.to_string())
}

/// Первая валидная версия в ASCII-тексте (лог-файл OpenCore)
fn first_version_in_text(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut start: Option<usize> = None;
    let mut buf = String::new();
    for (i, &b) in bytes.iter().enumerate() {
        let is_ver_char = b.is_ascii_digit() || b == b'.';
        if is_ver_char {
            if start.is_none() {
                start = Some(i);
                buf.clear();
            }
            buf.push(b as char);
        } else if start.is_some() {
            if let Some(v) = plausible_oc_version(&buf) {
                return Some(v);
            }
            start = None;
            buf.clear();
        }
        if buf.len() > 16 {
            start = None;
            buf.clear();
        }
    }
    plausible_oc_version(&buf)
}

/// Версия OpenCore из NVRAM-переменной (Linux efivars, macOS nvram)
fn version_from_nvram() -> Option<(String, String)> {
    match std::env::consts::OS {
        "linux" => {
            for name in [OC_VENDOR_GUID_EFIVAR, OC_APPLE_GUID_EFIVAR] {
                let p = Path::new("/sys/firmware/efi/efivars").join(name);
                if let Ok(data) = std::fs::read(&p) {
                    if data.len() > 4 {
                        // первые 4 байта — attributes efivarfs
                        let raw = String::from_utf8_lossy(&data[4..]);
                        let raw = raw.trim_end_matches(char::from(0));
                        if let Some(v) = plausible_oc_version(raw) {
                            return Some((v, "nvram".into()));
                        }
                    }
                }
            }
            None
        }
        "macos" => {
            let out = std::process::Command::new("nvram")
                .arg("opencore-version")
                .output()
                .ok()?;
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let text = text.split('\t').next_back().unwrap_or("").trim().to_string();
            plausible_oc_version(&text).map(|v| (v, "nvram".into()))
        }
        _ => None,
    }
}

/// Версия OpenCore из последнего лог-файла на ESP (opencore-*.txt)
fn version_from_logs(efi_base: &Path) -> Option<(String, String)> {
    let volume_root = efi_base.parent().unwrap_or(efi_base);
    let mut candidates: Vec<PathBuf> = Vec::new();
    for dir in [volume_root, &efi_base.join("OC")] {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_lowercase();
                if name.starts_with("opencore-") && name.ends_with(".txt") {
                    candidates.push(e.path());
                }
            }
        }
    }
    // свежий лог — последний по имени (в нём уже дата-время)
    candidates.sort();
    candidates.reverse();
    for log in candidates.iter().take(3) {
        if let Ok(data) = std::fs::read(log) {
            let head = &data[..data.len().min(64 * 1024)];
            let text = String::from_utf8_lossy(head);
            if let Some(v) = first_version_in_text(&text) {
                return Some((v, "log".into()));
            }
        }
    }
    None
}

/// Версия установленного OpenCore (best-effort): NVRAM → лог-файлы → None.
/// Определение по байтам OpenCore.efi невозможно — acidanthera не штампует
/// версию в бинарник (в RELEASE-сборках лежит шаблон REL-XXX-YYYY-MM-DD).
pub fn detect_installed_version(efi_base: &Path) -> (Option<String>, String) {
    if let Some((v, src)) = version_from_nvram() {
        return (Some(v), src);
    }
    if let Some((v, src)) = version_from_logs(efi_base) {
        return (Some(v), src);
    }
    (None, "none".into())
}

/// Статус установленного OpenCore для UI: версия + последняя на GitHub
pub fn detect_opencore_status(selected_dir: &Path) -> OpenCoreStatus {
    let efi_base = crate::downloader::resolve_efi_base(selected_dir);
    let oc = efi_base.join("OC");
    let installed = oc.join("OpenCore.efi").is_file() || oc.join("config.plist").is_file();
    let (version, version_source) = if installed {
        detect_installed_version(&efi_base)
    } else {
        (None, "none".into())
    };
    let latest = fetch_opencore_version(&http_agent_no_redirects()).ok();
    OpenCoreStatus {
        installed,
        version,
        latest,
        version_source,
    }
}

/// Основной сценарий: скачать OpenCorePkg + OcBinaryData и собрать полную EFI.
/// Используется и для первой установки, и для «Обновить OpenCore» (options
/// управляет Tools/AudioDxe). `progress` вызывается по мере прохождения этапов.
pub fn build_full_efi(
    selected_dir: &Path,
    options: FullEfiOptions,
    mut progress: impl FnMut(BuildProgress),
) -> Result<FullEfiReport, String> {
    let efi_base = crate::downloader::resolve_efi_base(selected_dir);
    let oc_dir = efi_base.join("OC");

    progress(BuildProgress {
        stage: "prepare".into(),
        message: "Проверка EFI-папки и резервное копирование…".into(),
        percent: 5,
    });
    let (backup_path, audit_warnings) = efi_safety(&efi_base);
    let mut warnings: Vec<String> = Vec::new();

    let agent = http_agent();
    let redirect_agent = http_agent_no_redirects();

    // --- 0. Версия установленного OpenCore (best-effort) и сравнение байт ---
    let (previous_version, _) = if oc_dir.join("OpenCore.efi").is_file() {
        detect_installed_version(&efi_base)
    } else {
        (None, "none".to_string())
    };

    // --- 1. OpenCorePkg: последний релиз (версия из redirect, без API-лимитов) ---
    progress(BuildProgress {
        stage: "oc_info".into(),
        message: "Определяю последнюю версию OpenCore…".into(),
        percent: 10,
    });
    let opencore_version = fetch_opencore_version(&redirect_agent)?;
    let download_base = format!(
        "https://github.com/{OC_PKG_REPO}/releases/download/{opencore_version}"
    );

    progress(BuildProgress {
        stage: "oc_download".into(),
        message: "Скачиваю OpenCorePkg…".into(),
        percent: 15,
    });
    let oc_zip = temp_zip("ocpkg");
    let rel_url = format!("{download_base}/OpenCore-{opencore_version}-RELEASE.zip");
    match download_to_file(&agent, &rel_url, &oc_zip) {
        Ok(_) => {}
        // RELEASE нет (нестандартный релиз) — пробуем DEBUG
        Err(e) if e.contains("404") => {
            let dbg_url = format!("{download_base}/OpenCore-{opencore_version}-DEBUG.zip");
            download_to_file(&agent, &dbg_url, &oc_zip)?;
        }
        Err(e) => return Err(e),
    }

    progress(BuildProgress {
        stage: "oc_extract".into(),
        message: "Распаковываю загрузчик OpenCore…".into(),
        percent: 35,
    });
    let mut oc_archive = open_zip(&oc_zip)?;

    // Совпадает ли установленный OpenCore.efi с новой версией (по байтам)?
    let was_up_to_date = match std::fs::read(oc_dir.join("OpenCore.efi")) {
        Ok(cur) => {
            let idx = find_entry(&mut oc_archive, "x64/efi/oc/opencore.efi")
                .ok_or("в архиве OpenCorePkg нет X64/EFI/OC/OpenCore.efi")?;
            Some(cur == entry_bytes(&mut oc_archive, idx)?)
        }
        Err(_) => None,
    };

    let oc_install = install_opencore_files(&mut oc_archive, &efi_base);

    // --- Опциональные инструменты (OpenShell) из того же архива ---
    let mut tools_installed: Vec<String> = Vec::new();
    if options.tools {
        match install_tools(&mut oc_archive, &efi_base) {
            Ok(Some(rel)) => tools_installed.push(rel),
            Ok(None) => warnings.push(
                "в архиве OpenCorePkg нет OpenShell.efi — структура релиза изменилась?".into(),
            ),
            Err(e) => warnings.push(format!("OpenShell не установлен: {e}")),
        }
    }

    // --- AudioDxe.efi — тоже из пакета OpenCorePkg (в OcBinaryData его нет) ---
    let mut audio_dxe_installed = false;
    if options.audio_dxe {
        if let Some(idx) = find_entry(&mut oc_archive, "x64/efi/oc/drivers/audiodxe.efi") {
            match entry_bytes(&mut oc_archive, idx) {
                Ok(bytes) => {
                    if let Err(e) = write_file(
                        &efi_base.join("OC").join("Drivers").join("AudioDxe.efi"),
                        &bytes,
                    ) {
                        warnings.push(format!("AudioDxe не установлен: {e}"));
                    } else {
                        audio_dxe_installed = true;
                    }
                }
                Err(e) => warnings.push(format!("AudioDxe не установлен: {e}")),
            }
        } else {
            warnings.push(
                "в архиве OpenCorePkg нет AudioDxe.efi — структура релиза изменилась?".into(),
            );
        }
    }

    let _ = std::fs::remove_file(&oc_zip);
    let oc_install = oc_install?;
    let mut files_installed = oc_install.files;
    files_installed.extend(tools_installed.iter().cloned());
    if audio_dxe_installed {
        files_installed.push("OC/Drivers/AudioDxe.efi".into());
    }

    if let Some(note) = &oc_install.bootx64_note {
        warnings.push(note.clone());
    }

    // --- 2. OcBinaryData: HfsPlus + Resources (нужен, если HfsPlus ещё нет
    //        или запрошен AudioDxe с аудиоресурсами) ---
    let hfs_exists = efi_base
        .join("OC")
        .join("Drivers")
        .join("HfsPlus.efi")
        .is_file();
    let need_ocbinary = !hfs_exists || options.audio_dxe;
    let mut audio_installed = false;
    let mut resources_installed: Vec<String> = Vec::new();
    if need_ocbinary {
        progress(BuildProgress {
            stage: "data_download".into(),
            message: "Скачиваю OcBinaryData (HfsPlus, Resources)…".into(),
            percent: 50,
        });
        let data_zip = temp_zip("ocbinary");
        download_to_file_limited(&agent, OC_BINARY_DATA_URL, &data_zip, MAX_OCBINARY_BYTES)?;

        progress(BuildProgress {
            stage: "data_extract".into(),
            message: "Распаковываю HfsPlus и Resources…".into(),
            percent: 65,
        });
        let mut data_archive = open_zip(&data_zip)?;
        let data_install = install_ocbinarydata(&mut data_archive, &efi_base, options.audio_dxe);
        let _ = std::fs::remove_file(&data_zip);
        let (data_files, res) = data_install?;
        // install_ocbinarydata кладёт в res имена подпапок Resources (Font/Label/Image/Audio):
        // «Audio» появляется там, только если аудиофайлы реально записаны на диск.
        let audio_files_copied = res.iter().any(|r| r.eq_ignore_ascii_case("Audio"));
        files_installed.extend(data_files);
        resources_installed = res;
        audio_installed = audio_dxe_installed && audio_files_copied;
    } else {
        progress(BuildProgress {
            stage: "data_extract".into(),
            message: "HfsPlus уже установлен — OcBinaryData пропущен".into(),
            percent: 65,
        });
    }

    // --- 3. config.plist: свой или Sample-фолбэк ---
    progress(BuildProgress {
        stage: "config".into(),
        message: "Настраиваю config.plist (драйверы, пикер)…".into(),
        percent: 80,
    });
    let config_path = oc_dir.join("config.plist");
    let mut config_source = "none".to_string();
    let mut config_patched = false;
    let mut config_backup = false;

    if config_path.is_file() {
        config_source = "existing".into();
    } else if let Some(sample) = &oc_install.sample_plist {
        write_file(&config_path, sample)?;
        config_source = "sample".into();
        warnings.push(
            "в EFI/OC не было config.plist — положен Sample.plist из OpenCorePkg: он ничего не \
             загружает, обязательно сгенерируйте свой конфиг (шаг «Конфиг»)"
                .into(),
        );
    } else {
        warnings.push(
            "config.plist не найден и в архиве OpenCorePkg нет Sample.plist — добавьте конфиг вручную"
                .into(),
        );
    }

    if config_path.is_file() {
        let mut drivers: Vec<String> = vec!["OpenRuntime.efi".into(), "HfsPlus.efi".into()];
        if oc_install.canopy {
            drivers.push("OpenCanopy.efi".into());
        }
        if files_installed.iter().any(|f| f.ends_with("ResetNvramEntry.efi")) {
            drivers.push("ResetNvramEntry.efi".into());
        }
        if files_installed.iter().any(|f| f.ends_with("ToggleSipEntry.efi")) {
            drivers.push("ToggleSipEntry.efi".into());
        }
        if audio_installed {
            drivers.push("AudioDxe.efi".into());
        }
        match patch_uefi_drivers(&config_path, &drivers, oc_install.canopy) {
            Ok((patched, backup)) => {
                config_patched = patched;
                config_backup = backup;
            }
            Err(e) => warnings.push(format!(
                "config.plist есть, но драйверы не зарегистрированы — включите их сами в UEFI → Drivers: {e}"
            )),
        }

        if audio_installed {
            // звук в пикере: AudioSupport + сами аудиофайлы
            match patch_audio_support(&config_path) {
                Ok(_) => {}
                Err(e) => warnings.push(format!(
                    "AudioSupport не включён — звук в пикере заработает после ручной правки UEFI → Audio: {e}"
                )),
            }
        }

        if !tools_installed.is_empty() {
            let tools: Vec<String> = tools_installed
                .iter()
                .map(|t| t.trim_start_matches("OC/").to_string())
                .collect();
            match patch_misc_tools(&config_path, &tools) {
                Ok(_) => {}
                Err(e) => warnings.push(format!(
                    "Tools не зарегистрированы в Misc → Tools — включите их сами: {e}"
                )),
            }
        }
    } else if !tools_installed.is_empty() {
        warnings.push(
            "config.plist не найден — OpenShell скопирован, но не зарегистрирован в Misc → Tools"
                .into(),
        );
    }

    progress(BuildProgress {
        stage: "done".into(),
        message: "Готово".into(),
        percent: 100,
    });

    Ok(FullEfiReport {
        efi_dir: efi_base.to_string_lossy().to_string(),
        oc_dir: oc_dir.to_string_lossy().to_string(),
        opencore_version: Some(opencore_version),
        previous_version,
        was_up_to_date,
        files_installed,
        bootx64: oc_install.bootx64,
        bootx64_note: oc_install.bootx64_note,
        config_source,
        config_path: if config_path.is_file() {
            Some(config_path.to_string_lossy().to_string())
        } else {
            None
        },
        config_patched,
        config_backup,
        canopy_enabled: oc_install.canopy,
        resources_installed,
        tools_installed,
        audio_installed,
        backup_path,
        audit_warnings,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;

    fn zip_bytes(build: impl FnOnce(&mut zip::ZipWriter<&mut std::io::Cursor<Vec<u8>>>)) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            build(&mut w);
            w.finish().unwrap();
        }
        buf.into_inner()
    }

    fn tmp(label: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("openefi-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn test_install_opencore_files_layout() {
        let efi = tmp("oc-layout");
        let data = zip_bytes(|w| {
            let opts = SimpleFileOptions::default();
            w.start_file("X64/EFI/BOOT/BOOTx64.efi", opts).unwrap();
            std::io::Write::write_all(w, b"NEWBOOT").unwrap();
            w.start_file("X64/EFI/BOOT/.contentFlavour", opts).unwrap();
            std::io::Write::write_all(w, b"flav").unwrap();
            w.start_file("X64/EFI/OC/OpenCore.efi", opts).unwrap();
            std::io::Write::write_all(w, b"OCNEW").unwrap();
            w.start_file("X64/EFI/OC/Drivers/OpenRuntime.efi", opts).unwrap();
            std::io::Write::write_all(w, b"RT").unwrap();
            w.start_file("X64/EFI/OC/Drivers/OpenCanopy.efi", opts).unwrap();
            std::io::Write::write_all(w, b"CP").unwrap();
            w.start_file("X64/EFI/OC/Drivers/ResetNvramEntry.efi", opts).unwrap();
            std::io::Write::write_all(w, b"RN").unwrap();
            w.start_file("IA32/EFI/OC/OpenCore.efi", opts).unwrap();
            std::io::Write::write_all(w, b"32bit").unwrap();
            w.start_file("Docs/Sample.plist", opts).unwrap();
            std::io::Write::write_all(w, b"<plist/>").unwrap();
        });
        let mut ar = ZipArchive::new(std::io::Cursor::new(&data)).unwrap();
        let inst = install_opencore_files(&mut ar, &efi).unwrap();

        assert_eq!(inst.bootx64, Bootx64Outcome::Installed);
        assert!(inst.canopy);
        assert!(inst.sample_plist.is_some());
        assert!(efi.join("BOOT").join("BOOTX64.EFI").exists());
        assert!(efi.join("BOOT").join(".contentFlavour").exists());
        assert!(efi.join("OC").join("OpenCore.efi").exists());
        assert!(efi.join("OC").join("Drivers").join("OpenRuntime.efi").exists());
        assert!(efi.join("OC").join("Drivers").join("OpenCanopy.efi").exists());
        assert!(efi.join("OC").join("Drivers").join("ResetNvramEntry.efi").exists());
        assert_eq!(
            std::fs::read(efi.join("OC").join("OpenCore.efi")).unwrap(),
            b"OCNEW"
        );
        assert_eq!(
            std::fs::read(efi.join("BOOT").join("BOOTX64.EFI")).unwrap(),
            b"NEWBOOT"
        );
        assert_eq!(inst.files.len(), 6, "boot, 1 маркер, OC, RT, Canopy, RN: {:?}", inst.files);
        std::fs::remove_dir_all(&efi).ok();
    }

    #[test]
    fn test_bootx64_foreign_not_touched() {
        let efi = tmp("oc-foreign");
        // чужой fallback-загрузчик (например, GRUB shim)
        std::fs::create_dir_all(efi.join("BOOT")).unwrap();
        std::fs::write(efi.join("BOOT").join("BOOTX64.EFI"), b"grub-shim").unwrap();

        let data = zip_bytes(|w| {
            let opts = SimpleFileOptions::default();
            w.start_file("X64/EFI/BOOT/BOOTx64.efi", opts).unwrap();
            std::io::Write::write_all(w, b"NEWBOOT").unwrap();
            w.start_file("X64/EFI/OC/OpenCore.efi", opts).unwrap();
            std::io::Write::write_all(w, b"OCNEW").unwrap();
            w.start_file("X64/EFI/OC/Drivers/OpenRuntime.efi", opts).unwrap();
            std::io::Write::write_all(w, b"RT").unwrap();
        });
        let mut ar = ZipArchive::new(std::io::Cursor::new(&data)).unwrap();
        let inst = install_opencore_files(&mut ar, &efi).unwrap();

        assert_eq!(inst.bootx64, Bootx64Outcome::SkippedForeign);
        assert!(inst.bootx64_note.is_some());
        // чужой файл не перезаписан, ядро OpenCore при этом установлено
        assert_eq!(
            std::fs::read(efi.join("BOOT").join("BOOTX64.EFI")).unwrap(),
            b"grub-shim"
        );
        assert!(efi.join("OC").join("OpenCore.efi").exists());
        assert!(!inst.files.iter().any(|f| f.starts_with("BOOT/")));
        std::fs::remove_dir_all(&efi).ok();
    }

    #[test]
    fn test_bootx64_old_opencore_updated() {
        let efi = tmp("oc-update");
        // старая установка OpenCore: BOOTX64 — копия старого OpenCore.efi
        std::fs::create_dir_all(efi.join("BOOT")).unwrap();
        std::fs::write(efi.join("BOOT").join("BOOTX64.EFI"), b"OLD").unwrap();
        std::fs::create_dir_all(efi.join("OC")).unwrap();
        std::fs::write(efi.join("OC").join("OpenCore.efi"), b"OLD").unwrap();

        let data = zip_bytes(|w| {
            let opts = SimpleFileOptions::default();
            w.start_file("X64/EFI/BOOT/BOOTx64.efi", opts).unwrap();
            std::io::Write::write_all(w, b"NEW").unwrap();
            w.start_file("X64/EFI/OC/OpenCore.efi", opts).unwrap();
            std::io::Write::write_all(w, b"NEW").unwrap();
            w.start_file("X64/EFI/OC/Drivers/OpenRuntime.efi", opts).unwrap();
            std::io::Write::write_all(w, b"RT").unwrap();
        });
        let mut ar = ZipArchive::new(std::io::Cursor::new(&data)).unwrap();
        let inst = install_opencore_files(&mut ar, &efi).unwrap();

        assert_eq!(inst.bootx64, Bootx64Outcome::Updated);
        assert_eq!(std::fs::read(efi.join("BOOT").join("BOOTX64.EFI")).unwrap(), b"NEW");
        assert_eq!(std::fs::read(efi.join("OC").join("OpenCore.efi")).unwrap(), b"NEW");
        std::fs::remove_dir_all(&efi).ok();
    }

    #[test]
    fn test_bootx64_same_version_skipped() {
        let efi = tmp("oc-same");
        std::fs::create_dir_all(efi.join("BOOT")).unwrap();
        std::fs::write(efi.join("BOOT").join("BOOTX64.EFI"), b"NEW").unwrap();

        let data = zip_bytes(|w| {
            let opts = SimpleFileOptions::default();
            w.start_file("X64/EFI/BOOT/BOOTx64.efi", opts).unwrap();
            std::io::Write::write_all(w, b"NEW").unwrap();
            w.start_file("X64/EFI/OC/OpenCore.efi", opts).unwrap();
            std::io::Write::write_all(w, b"OC").unwrap();
            w.start_file("X64/EFI/OC/Drivers/OpenRuntime.efi", opts).unwrap();
            std::io::Write::write_all(w, b"RT").unwrap();
        });
        let mut ar = ZipArchive::new(std::io::Cursor::new(&data)).unwrap();
        let inst = install_opencore_files(&mut ar, &efi).unwrap();
        assert_eq!(inst.bootx64, Bootx64Outcome::SkippedSame);
        assert!(inst.bootx64_note.is_none());
        std::fs::remove_dir_all(&efi).ok();
    }

    #[test]
    fn test_install_ocbinarydata_layout() {
        let efi = tmp("oc-data");
        let data = zip_bytes(|w| {
            let opts = SimpleFileOptions::default();
            w.start_file("OcBinaryData-master/Drivers/HfsPlus.efi", opts).unwrap();
            std::io::Write::write_all(w, b"HFS").unwrap();
            w.start_file("OcBinaryData-master/Drivers/OpenHfsPlus.efi", opts).unwrap();
            std::io::Write::write_all(w, b"OHFS").unwrap();
            w.start_file("OcBinaryData-master/Resources/Font/Roboto-Regular.ttf", opts).unwrap();
            std::io::Write::write_all(w, b"font").unwrap();
            w.start_file("OcBinaryData-master/Resources/Label/EFI/acpi.acil", opts).unwrap();
            std::io::Write::write_all(w, b"label").unwrap();
            w.start_file("OcBinaryData-master/Resources/Image/Acpi/Arrow.aci", opts).unwrap();
            std::io::Write::write_all(w, b"image").unwrap();
            w.start_file("OcBinaryData-master/Resources/Audio/beep.wav", opts).unwrap();
            std::io::Write::write_all(w, b"audio").unwrap();
            w.start_file("OcBinaryData-master/Docs/Readme.md", opts).unwrap();
            std::io::Write::write_all(w, b"docs").unwrap();
        });
        let mut ar = ZipArchive::new(std::io::Cursor::new(&data)).unwrap();
        let (files, resources) = install_ocbinarydata(&mut ar, &efi, false).unwrap();

        assert!(efi.join("OC").join("Drivers").join("HfsPlus.efi").exists());
        assert!(efi.join("OC").join("Resources").join("Font").join("Roboto-Regular.ttf").exists());
        assert!(efi.join("OC").join("Resources").join("Label").join("EFI").join("acpi.acil").exists());
        assert!(efi.join("OC").join("Resources").join("Image").join("Acpi").join("Arrow.aci").exists());
        assert!(
            !efi.join("OC").join("Resources").join("Audio").exists(),
            "Audio без AudioDxe не нужен"
        );
        assert_eq!(resources, vec!["Font".to_string(), "Image".to_string(), "Label".to_string()]);
        assert_eq!(files, vec!["OC/Drivers/HfsPlus.efi".to_string()]);
        std::fs::remove_dir_all(&efi).ok();
    }

    #[test]
    fn test_install_ocbinarydata_with_audio() {
        let efi = tmp("oc-hfs-audio");
        let data = zip_bytes(|w| {
            let opts = SimpleFileOptions::default();
            w.start_file("OcBinaryData-master/Drivers/HfsPlus.efi", opts).unwrap();
            std::io::Write::write_all(w, b"HFS").unwrap();
            w.start_file("OcBinaryData-master/Resources/Font/Roboto-Regular.ttf", opts).unwrap();
            std::io::Write::write_all(w, b"font").unwrap();
            w.start_file("OcBinaryData-master/Resources/Audio/AXEFIAudio_Boot.mp3", opts).unwrap();
            std::io::Write::write_all(w, b"boot-mp3").unwrap();
            w.start_file("OcBinaryData-master/Resources/Audio/AXEFIAudio_Click.mp3", opts).unwrap();
            std::io::Write::write_all(w, b"click-mp3").unwrap();
            w.start_file("OcBinaryData-master/Resources/Audio/AXEFIAudio_ru_Password.mp3", opts).unwrap();
            std::io::Write::write_all(w, b"ru-mp3").unwrap();
        });
        let mut ar = ZipArchive::new(std::io::Cursor::new(&data)).unwrap();
        let (files, resources) = install_ocbinarydata(&mut ar, &efi, true).unwrap();

        assert!(files.contains(&"OC/Drivers/HfsPlus.efi".to_string()));
        assert!(resources.contains(&"Audio".to_string()));
        let audio_dir = efi.join("OC").join("Resources").join("Audio");
        assert!(audio_dir.join("AXEFIAudio_Boot.mp3").exists());
        assert!(audio_dir.join("AXEFIAudio_Click.mp3").exists());
        // языковые файлы не копируем — EFI не раздувается
        assert!(!audio_dir.join("AXEFIAudio_ru_Password.mp3").exists());
        std::fs::remove_dir_all(&efi).ok();
    }

    #[test]
    fn test_install_ocbinarydata_requires_hfsplus() {
        let efi = tmp("oc-nohfs");
        let data = zip_bytes(|w| {
            let opts = SimpleFileOptions::default();
            w.start_file("OcBinaryData-master/Drivers/OpenHfsPlus.efi", opts).unwrap();
            std::io::Write::write_all(w, b"x").unwrap();
        });
        let mut ar = ZipArchive::new(std::io::Cursor::new(&data)).unwrap();
        let err = install_ocbinarydata(&mut ar, &efi, false).unwrap_err();
        assert!(err.contains("HfsPlus"), "внятная ошибка: {err}");
        std::fs::remove_dir_all(&efi).ok();
    }

    fn drivers_config_xml() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
  <key>UEFI</key>
  <dict>
    <key>ConnectDrivers</key><true/>
    <key>Drivers</key>
    <array>
      <dict>
        <key>Comment</key><string></string>
        <key>Enabled</key><true/>
        <key>Path</key><string>OpenRuntime.efi</string>
      </dict>
      <dict>
        <key>Comment</key><string></string>
        <key>Enabled</key><false/>
        <key>Path</key><string>ResetNvramEntry.efi</string>
      </dict>
      <dict>
        <key>Comment</key><string>HFS+ Driver</string>
        <key>Enabled</key><true/>
        <key>Path</key><string>Drivers/HfsPlus.efi</string>
      </dict>
    </array>
  </dict>
  <key>Misc</key>
  <dict>
    <key>Boot</key>
    <dict>
      <key>PickerAttributes</key><integer>17</integer>
      <key>PickerMode</key><string>Builtin</string>
    </dict>
  </dict>
</dict>
</plist>"#
            .into()
    }

    #[test]
    fn test_patch_uefi_drivers_enable_and_append() {
        let dir = tmp("oc-cfg1");
        let config = dir.join("config.plist");
        std::fs::write(&config, drivers_config_xml()).unwrap();

        let drivers = vec![
            "OpenRuntime.efi".to_string(),
            "HfsPlus.efi".to_string(),
            "ResetNvramEntry.efi".to_string(),
            "ToggleSipEntry.efi".to_string(),
            "OpenCanopy.efi".to_string(),
        ];
        let (changed, backup) = patch_uefi_drivers(&config, &drivers, true).unwrap();
        assert!(changed);
        assert!(backup);
        assert!(dir.join("config.plist.bak-openefi").exists(), "нужна резервная копия");

        let root = Value::from_reader(std::fs::File::open(&config).unwrap())
            .unwrap()
            .into_dictionary()
            .unwrap();
        let arr = root
            .get("UEFI")
            .and_then(|u| u.as_dictionary())
            .and_then(|u| u.get("Drivers"))
            .and_then(|d| d.as_array())
            .unwrap();
        let paths: Vec<(String, bool)> = arr
            .iter()
            .filter_map(|v| v.as_dictionary())
            .filter_map(|d| {
                let p = d.get("Path")?.as_string()?.to_string();
                let e = d.get("Enabled")?.as_boolean()?;
                Some((p, e))
            })
            .collect();
        assert_eq!(paths.len(), 5, "HfsPlus с подпапкой не дублируется: {paths:?}");
        assert!(paths.contains(&("OpenRuntime.efi".to_string(), true)));
        assert!(paths.contains(&("ResetNvramEntry.efi".to_string(), true)), "выключенная запись включена");
        assert!(paths.contains(&("Drivers/HfsPlus.efi".to_string(), true)), "подпапка не тронута");
        assert!(paths.contains(&("ToggleSipEntry.efi".to_string(), true)));
        assert!(paths.contains(&("OpenCanopy.efi".to_string(), true)));

        let picker = root
            .get("Misc").and_then(|m| m.as_dictionary())
            .and_then(|m| m.get("Boot")).and_then(|b| b.as_dictionary())
            .and_then(|b| b.get("PickerMode")).and_then(|p| p.as_string())
            .unwrap();
        assert_eq!(picker, "External", "OpenCanopy требует External-пикер");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_patch_uefi_drivers_no_changes_no_backup() {
        let dir = tmp("oc-cfg2");
        let config = dir.join("config.plist");
        std::fs::write(&config, drivers_config_xml()).unwrap();

        // всё уже включено, пикер не трогаем
        let (changed, backup) = patch_uefi_drivers(&config, &["OpenRuntime.efi".into()], false).unwrap();
        assert!(!changed);
        assert!(!backup);
        assert!(!dir.join("config.plist.bak-openefi").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_patch_uefi_drivers_no_uefi_section() {
        let dir = tmp("oc-cfg3");
        let config = dir.join("config.plist");
        std::fs::write(
            &config,
            r#"<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>ACPI</key><dict/></dict></plist>"#,
        )
        .unwrap();
        let err = patch_uefi_drivers(&config, &["OpenRuntime.efi".into()], false).unwrap_err();
        assert!(err.contains("UEFI"), "внятная ошибка: {err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[ignore] // сетевой тест: запускается вручную
    fn test_network_full_efi_build() {
        let d = tmp("oc-net");
        std::fs::create_dir_all(d.join("EFI")).unwrap(); // resolve_efi_base найдёт EFI
        let report =
            build_full_efi(&d, FullEfiOptions { tools: true, audio_dxe: false }, |_| {}).unwrap();
        assert!(report.opencore_version.is_some(), "версия OpenCore определена");
        assert!(d.join("EFI").join("OC").join("OpenCore.efi").exists());
        assert!(d.join("EFI").join("OC").join("Drivers").join("OpenRuntime.efi").exists());
        assert!(d.join("EFI").join("OC").join("Drivers").join("HfsPlus.efi").exists());
        assert!(d.join("EFI").join("BOOT").join("BOOTX64.EFI").exists());
        // Resources распакованы
        let res = d.join("EFI").join("OC").join("Resources");
        assert!(!report.resources_installed.is_empty(), "Resources установлены");
        assert!(res.join("Font").is_dir() || res.join("Label").is_dir() || res.join("Image").is_dir());
        // Sample-конфиг уложен и настроен
        assert_eq!(report.config_source, "sample");
        assert!(report.config_patched, "драйверы зарегистрированы");
        let cfg = std::fs::read_to_string(d.join("EFI").join("OC").join("config.plist")).unwrap();
        assert!(cfg.contains("OpenRuntime.efi"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_install_tools_from_archive() {
        let efi = tmp("oc-tools");
        let data = zip_bytes(|w| {
            let opts = SimpleFileOptions::default();
            w.start_file("X64/EFI/OC/Tools/OpenShell.efi", opts).unwrap();
            std::io::Write::write_all(w, b"shell").unwrap();
            w.start_file("X64/EFI/OC/Tools/CsrUtil.efi", opts).unwrap();
            std::io::Write::write_all(w, b"csr").unwrap();
        });
        let mut ar = ZipArchive::new(std::io::Cursor::new(&data)).unwrap();
        let rel = install_tools(&mut ar, &efi).unwrap();
        assert_eq!(rel.as_deref(), Some("OC/Tools/OpenShell.efi"));
        assert!(efi.join("OC").join("Tools").join("OpenShell.efi").exists());
        // CsrUtil не запрошен — не установлен
        assert!(!efi.join("OC").join("Tools").join("CsrUtil.efi").exists());
        std::fs::remove_dir_all(&efi).ok();
    }

    #[test]
    fn test_patch_misc_tools_append_and_enable() {
        let dir = tmp("oc-tools-cfg");
        let config = dir.join("config.plist");
        std::fs::write(&config, drivers_config_xml()).unwrap(); // есть Misc/Boot, нет Tools

        let (changed, backup) = patch_misc_tools(&config, &["Tools/OpenShell.efi".into()]).unwrap();
        assert!(changed);
        assert!(backup);
        let root = Value::from_reader(std::fs::File::open(&config).unwrap())
            .unwrap()
            .into_dictionary()
            .unwrap();
        let arr = root
            .get("Misc")
            .and_then(|m| m.as_dictionary())
            .and_then(|m| m.get("Tools"))
            .and_then(|t| t.as_array())
            .unwrap();
        assert_eq!(arr.len(), 1);
        let d = arr[0].as_dictionary().unwrap();
        assert_eq!(d.get("Path").and_then(|p| p.as_string()), Some("Tools/OpenShell.efi"));
        assert_eq!(d.get("Name").and_then(|p| p.as_string()), Some("OpenShell.efi"));
        assert_eq!(d.get("Enabled").and_then(|p| p.as_boolean()), Some(true));
        assert_eq!(d.get("Auxiliary").and_then(|p| p.as_boolean()), Some(true));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_patch_audio_support() {
        let dir = tmp("oc-audio-cfg");
        let config = dir.join("config.plist");
        std::fs::write(&config, drivers_config_xml()).unwrap();

        let (changed, _) = patch_audio_support(&config).unwrap();
        assert!(changed);
        let root = Value::from_reader(std::fs::File::open(&config).unwrap())
            .unwrap()
            .into_dictionary()
            .unwrap();
        let support = root
            .get("UEFI")
            .and_then(|u| u.as_dictionary())
            .and_then(|u| u.get("Audio"))
            .and_then(|a| a.as_dictionary())
            .and_then(|a| a.get("AudioSupport"))
            .and_then(|a| a.as_boolean());
        assert_eq!(support, Some(true));
        // повторно — уже включено, изменений нет
        let (changed2, backup2) = patch_audio_support(&config).unwrap();
        assert!(!changed2);
        assert!(!backup2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_plausible_oc_version() {
        assert_eq!(plausible_oc_version("1.0.5").as_deref(), Some("1.0.5"));
        assert_eq!(plausible_oc_version("0.9.7").as_deref(), Some("0.9.7"));
        assert_eq!(plausible_oc_version(" 0.7.7 ").as_deref(), Some("0.7.7"));
        assert_eq!(plausible_oc_version("12.0.1"), None, "OC не бывает 12.x");
        assert_eq!(plausible_oc_version("1.2"), None);
        assert_eq!(plausible_oc_version("1.2.3.4"), None);
        assert_eq!(plausible_oc_version("abc"), None);
    }

    #[test]
    fn test_first_version_in_text() {
        // формат лога OpenCore: первая версия в тексте — версия OC
        let log = "00:000 00:000 OC: Version: 1.0.5 (RELEASE)\n00:002 00:002 OC: ...";
        assert_eq!(first_version_in_text(log).as_deref(), Some("1.0.5"));
        let log2 = "OCABC: boot 2025-07-07\nOpenCore 0.9.7";
        assert_eq!(first_version_in_text(log2).as_deref(), Some("0.9.7"));
        // мусор, на который не должно срабатывать
        assert_eq!(first_version_in_text("99.99.99 nope 2025.07.07"), None);
    }

    #[test]
    fn test_version_from_logs() {
        let d = tmp("oc-logs");
        let efi = d.join("EFI");
        std::fs::create_dir_all(efi.join("OC")).unwrap();
        std::fs::write(
            d.join("opencore-2025-01-01-010101.txt"),
            "OC: Version: 0.9.9 (RELEASE)\n...",
        )
        .unwrap();
        std::fs::write(
            d.join("opencore-2025-07-07-070707.txt"),
            "OC: Version: 1.0.2 (RELEASE)\n...",
        )
        .unwrap();
        let (v, src) = version_from_logs(&efi).expect("версия найдена в логе");
        assert_eq!(v, "1.0.2", "берём самый свежий лог");
        assert_eq!(src, "log");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_detect_installed_version_without_sources() {
        let d = tmp("oc-nover");
        let efi = d.join("EFI");
        std::fs::create_dir_all(efi.join("OC")).unwrap();
        std::fs::write(efi.join("OC").join("OpenCore.efi"), b"bin").unwrap();
        let (v, src) = detect_installed_version(&efi);
        // в песочнице нет ни efivars с opencore-version, ни логов
        assert_eq!(src, "none");
        assert_eq!(v, None);
        std::fs::remove_dir_all(&d).ok();
    }
}
