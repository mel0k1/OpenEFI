//! Защита EFI: аудит папки перед установкой, обнаружение чужих загрузчиков
//! и автоматический бэкап EFI/OC перед любыми изменениями.
//!
//! Гарантии OpenEFI:
//! - пишем ТОЛЬКО внутрь EFI/OC (Kexts, ACPI, config.plist, bootstrap/OC);
//! - никогда не перезаписываем EFI/Boot/BOOTX64.EFI — это fallback-загрузчик
//!   Windows/дистрибутива Linux, и его порча ломает загрузку чужой ОС;
//! - не трогаем EFI/Microsoft, EFI/CLOVER, EFI/ubuntu и прочие папки;
//! - перед изменениями уже установленного EFI/OC снимаем полную копию.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::downloader::resolve_efi_base;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BootloaderEntry {
    /// Человекочитаемое имя («Windows Boot Manager», «GRUB», «Clover»…)
    pub name: String,
    /// Путь относительно корня ESP (например «EFI/Microsoft»)
    pub path: String,
    /// opencore | clover | windows | linux | refind | other
    pub kind: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EfiAudit {
    /// Разрешённая база EFI (например «/Volumes/ESP/EFI»)
    pub efi_dir: String,
    /// Каталог EFI найден
    pub found: bool,
    /// Похоже на EFI-раздел (есть типовые каталоги загрузчиков)
    pub looks_like_esp: bool,
    /// Установлен ли OpenCore (OpenCore.efi или config.plist в EFI/OC)
    pub opencore: bool,
    /// Сколько *.kext в OC/Kexts
    pub kexts_count: usize,
    /// Сколько *.aml в OC/ACPI
    pub acpi_count: usize,
    /// Есть ли OC/config.plist
    pub has_config: bool,
    pub bootloaders: Vec<BootloaderEntry>,
    /// Существует EFI/Boot/BOOTX64.EFI (возможно, fallback чужой ОС)
    pub shared_bootx64: bool,
    /// Предупреждения аудита (переводятся tb())
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupReport {
    pub backup_path: String,
    pub files: usize,
}

/// Каталоги известных загрузчиков внутри EFI/ (строчные имена)
const LINUX_BOOT_DIRS: &[&str] = &[
    "ubuntu", "debian", "fedora", "arch", "manjaro", "endeavouros", "opensuse", "kali", "mint",
    "elementary", "pop", "zorin", "gentoo", "void", "nixos", "rocky", "almalinux", "centos",
];

fn file_exists(p: &Path) -> bool {
    p.is_file()
}

/// Сколько каталогов с данным суффиксом (например .kext) лежит в папке
fn count_bundles(dir: &Path, suffix: &str) -> usize {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .to_lowercase()
                        .ends_with(suffix)
                })
                .count()
        })
        .unwrap_or(0)
}

/// Аудит EFI-папки: что уже стоит и чего OpenEFI не тронет
pub fn audit(selected: &Path) -> EfiAudit {
    let efi_base = resolve_efi_base(selected);
    // «найден EFI» = разрешённая база существует И была выбрана осознанно:
    // имя EFI/OC у выбранной папки или внутри неё есть каталог EFI
    let sel_name_lc = selected
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let resolved_explicitly =
        sel_name_lc == "efi" || sel_name_lc == "oc" || selected.join("EFI").is_dir();
    let found = efi_base.is_dir() && resolved_explicitly;

    let oc = efi_base.join("OC");
    let opencore = file_exists(&oc.join("OpenCore.efi")) || file_exists(&oc.join("config.plist"));
    let has_config = file_exists(&oc.join("config.plist"));
    let kexts_count = count_bundles(&oc.join("Kexts"), ".kext");
    let acpi_count = count_bundles(&oc.join("ACPI"), ".aml");

    // список загрузчиков + поиск Boot/BOOTX64.EFI (регистр не важен: FAT нечувствителен,
    // но на некоторых Linux-монтированиях сверяемся напрямую)
    let mut bootloaders: Vec<BootloaderEntry> = Vec::new();
    let mut shared_bootx64 = false;
    if found {
        if let Ok(rd) = std::fs::read_dir(&efi_base) {
            for e in rd.flatten() {
                if !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let dir_name = e.file_name().to_string_lossy().to_string();
                let name_lc = dir_name.to_lowercase();
                let rel = format!("EFI/{dir_name}");
                let known = match name_lc.as_str() {
                    "oc" => Some(BootloaderEntry {
                        name: "OpenCore".into(),
                        path: rel,
                        kind: "opencore".into(),
                    }),
                    "clover" => Some(BootloaderEntry {
                        name: "Clover".into(),
                        path: rel,
                        kind: "clover".into(),
                    }),
                    "microsoft" => Some(BootloaderEntry {
                        name: "Windows Boot Manager".into(),
                        path: rel,
                        kind: "windows".into(),
                    }),
                    "systemd" => Some(BootloaderEntry {
                        name: "systemd-boot".into(),
                        path: rel,
                        kind: "linux".into(),
                    }),
                    "refind" => Some(BootloaderEntry {
                        name: "rEFInd".into(),
                        path: rel,
                        kind: "refind".into(),
                    }),
                    "refit" => Some(BootloaderEntry {
                        name: "rEFIt".into(),
                        path: rel,
                        kind: "refind".into(),
                    }),
                    n if LINUX_BOOT_DIRS.contains(&n) => Some(BootloaderEntry {
                        name: "GRUB (Linux)".into(),
                        path: rel,
                        kind: "linux".into(),
                    }),
                    "boot" => {
                        // fallback-загрузчик: EFI/Boot/BOOTX64.EFI
                        shared_bootx64 = std::fs::read_dir(e.path())
                            .map(|rd| {
                                rd.flatten().any(|f| {
                                    f.file_name()
                                        .to_string_lossy()
                                        .eq_ignore_ascii_case("bootx64.efi")
                                })
                            })
                            .unwrap_or(false);
                        None
                    }
                    // прочие папки: смотрим, есть ли внутри .efi
                    n if !n.starts_with('.') => {
                        let has_efi = std::fs::read_dir(e.path())
                            .map(|rd| {
                                rd.flatten().any(|f| {
                                    f.file_name()
                                        .to_string_lossy()
                                        .to_lowercase()
                                        .ends_with(".efi")
                                })
                            })
                            .unwrap_or(false);
                        has_efi.then(|| BootloaderEntry {
                            name: dir_name,
                            path: rel,
                            kind: "other".into(),
                        })
                    }
                    _ => None,
                };
                if let Some(b) = known {
                    bootloaders.push(b);
                }
            }
        }
    }

    // похож ли раздел на ESP: есть типовые каталоги
    let looks_like_esp = found
        && (oc.is_dir()
            || efi_base.join("Microsoft").is_dir()
            || efi_base.join("Boot").is_dir()
            || efi_base.join("CLOVER").is_dir()
            || bootloaders.len() > 1);

    let mut warnings = Vec::new();
    if !found {
        warnings.push("в выбранной папке нет каталога EFI — возможно, выбран не тот диск".into());
    } else {
        if !looks_like_esp {
            warnings.push("выбранная папка не похожа на EFI-раздел".into());
        }
        if !opencore {
            warnings.push(
                "OpenCore не найден в EFI/OC — кексты положить можно, но загрузчик придётся установить отдельно (OpenCorePkg)".into(),
            );
        }
        if shared_bootx64 && !opencore {
            warnings.push(
                "в EFI/Boot лежит BOOTX64.EFI чужого загрузчика — OpenEFI никогда не перезаписывает этот файл".into(),
            );
        }
    }

    EfiAudit {
        efi_dir: efi_base.to_string_lossy().to_string(),
        found,
        looks_like_esp,
        opencore,
        kexts_count,
        acpi_count,
        has_config,
        bootloaders,
        shared_bootx64,
        warnings,
    }
}

/// Предупреждения аудита одной строкой набора — для отчётов скачивания
pub fn audit_warnings(selected: &Path) -> Vec<String> {
    audit(selected).warnings
}

/// Полный бэкап EFI/OC → EFI/OC.bak-openefi-<unix_ts>
/// Возвращает отчёт с путём и числом скопированных файлов.
pub fn backup_oc(selected: &Path) -> Result<BackupReport, String> {
    let efi_base = resolve_efi_base(selected);
    let oc = efi_base.join("OC");
    if !oc.is_dir() {
        return Err("OpenCore не найден в EFI/OC — бэкапить нечего".into());
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dst = efi_base.join(format!("OC.bak-openefi-{ts}"));
    let files = count_files_recursive(&oc);
    copy_dir_recursive(&oc, &dst)?;
    Ok(BackupReport {
        backup_path: dst.to_string_lossy().to_string(),
        files,
    })
}

fn count_files_recursive(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| {
                    if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        count_files_recursive(&e.path())
                    } else {
                        1
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}

/// Рекурсивное копирование папки (назначение не должно существовать)
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if dst.exists() {
        return Err(format!("{} уже существует — подождите секунду и повторите", dst.display()));
    }
    std::fs::create_dir_all(dst).map_err(|e| format!("не создать {}: {e}", dst.display()))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("не читать {}: {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("не читать {}: {e}", src.display()))?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to)
                .map_err(|e| format!("не скопировать {}: {e}", entry.path().display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_detects_bootloaders_and_oc() {
        let tmp = std::env::temp_dir().join(format!("openefi-audit1-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        let efi = tmp.join("volume").join("EFI");
        let oc = efi.join("OC");
        std::fs::create_dir_all(oc.join("Kexts/Lilu.kext/Contents")).unwrap();
        std::fs::create_dir_all(oc.join("ACPI")).unwrap();
        std::fs::write(oc.join("OpenCore.efi"), b"boot").unwrap();
        std::fs::write(oc.join("config.plist"), b"<plist/>").unwrap();
        std::fs::write(oc.join("ACPI/SSDT-PLUG.aml"), b"SSDT").unwrap();
        std::fs::create_dir_all(efi.join("Microsoft/Boot")).unwrap();
        std::fs::write(efi.join("Microsoft/Boot/bootmgfw.efi"), b"win").unwrap();
        std::fs::create_dir_all(efi.join("ubuntu")).unwrap();
        std::fs::write(efi.join("ubuntu/grubx64.efi"), b"grub").unwrap();
        std::fs::create_dir_all(efi.join("Boot")).unwrap();
        std::fs::write(efi.join("Boot/BOOTX64.EFI"), b"fallback").unwrap();

        let a = audit(&tmp.join("volume"));
        assert!(a.found);
        assert!(a.looks_like_esp);
        assert!(a.opencore);
        assert!(a.has_config);
        assert_eq!(a.kexts_count, 1);
        assert_eq!(a.acpi_count, 1);
        assert!(a.shared_bootx64);
        assert!(a.warnings.is_empty(), "с установленным OC предупреждений нет");

        let kinds: Vec<&str> = a.bootloaders.iter().map(|b| b.kind.as_str()).collect();
        assert!(kinds.contains(&"opencore"));
        assert!(kinds.contains(&"windows"));
        assert!(kinds.contains(&"linux"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_audit_empty_folder_warns_no_oc() {
        let tmp = std::env::temp_dir().join(format!("openefi-audit2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("volume")).unwrap();

        let a = audit(&tmp.join("volume"));
        assert!(!a.found, "EFI-каталога нет");
        assert!(!a.opencore);
        assert!(a
            .warnings
            .iter()
            .any(|w| w.contains("нет каталога EFI") || w.contains("OpenCore не найден")));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_backup_oc_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("openefi-bak1-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        let efi = tmp.join("volume").join("EFI");
        let oc = efi.join("OC");
        std::fs::create_dir_all(oc.join("Kexts/Lilu.kext/Contents/MacOS")).unwrap();
        std::fs::write(oc.join("config.plist"), b"original").unwrap();
        std::fs::write(oc.join("Kexts/Lilu.kext/Contents/MacOS/lilu"), b"x").unwrap();

        let rep = backup_oc(&tmp.join("volume")).unwrap();
        assert!(rep.files >= 2, "в бэкапе конфиг + кекст, получено {}", rep.files);
        let bak = std::path::PathBuf::from(&rep.backup_path);
        assert!(bak.is_dir());
        assert_eq!(
            std::fs::read(bak.join("config.plist")).unwrap(),
            b"original".to_vec()
        );

        // портим оригинал — бэкап untouched
        std::fs::write(oc.join("config.plist"), b"broken").unwrap();
        assert_eq!(
            std::fs::read(bak.join("config.plist")).unwrap(),
            b"original".to_vec()
        );

        // без OC — ошибка
        let empty = tmp.join("other");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(backup_oc(&empty).is_err());

        std::fs::remove_dir_all(&tmp).ok();
    }
}
