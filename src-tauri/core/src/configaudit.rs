//! Аудит существующего config.plist: сверяем Kernel/Add и ACPI/Add с реальными
//! файлами на диске (EFI/OC), ловим битые пути, «мёртвые» записи и незарегистрированные
//! бандлы, проверяем временный UTBDefault против сгенерированного UTBMap.
//!
//! Аудит только читает — ничего не правит.

use std::path::{Path, PathBuf};

use plist::Value;
use serde::{Deserialize, Serialize};

use crate::downloader::resolve_efi_base;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuditIssue {
    /// error | warning | info
    pub severity: String,
    /// Семантический код (перевод ключом на фронте)
    pub code: String,
    /// Параметры для подстановки в перевод
    #[serde(default)]
    pub params: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KextAudit {
    /// BundlePath как в конфиге (например «Kexts/Lilu.kext»)
    pub bundle: String,
    pub enabled: bool,
    /// Бандл найден на диске
    pub on_disk: bool,
    /// Версия из Info.plist бандла (CFBundleShortVersionString)
    pub version: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AcpiAudit {
    /// Path как в конфиге (например «SSDT-PLUG-DRTNIA.aml»)
    pub path: String,
    pub enabled: bool,
    pub on_disk: bool,
}

/// Кекст на диске, которого нет в Kernel/Add
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrphanKext {
    pub bundle: String,
    /// Зарегистрирован ли (пусть и выключенной) записью в Kernel/Add
    pub registered: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigAudit {
    /// Путь к проверенному config.plist
    pub config_path: String,
    /// Конфиг найден и разобран
    pub found: bool,
    pub kernel: Vec<KextAudit>,
    pub acpi: Vec<AcpiAudit>,
    /// .kext на диске без записи в Kernel/Add
    pub orphans: Vec<OrphanKext>,
    /// .aml на диске без записи в ACPI/Add
    pub orphan_tables: Vec<String>,
    pub issues: Vec<AuditIssue>,
    /// Счётчики для сводки
    pub kexts_total: usize,
    pub kexts_enabled: usize,
    pub tables_total: usize,
}

/// Версия кекта из Contents/Info.plist (CFBundleShortVersionString → CFBundleVersion)
pub fn read_bundle_version(info_plist: &Path) -> Option<String> {
    let value = Value::from_reader(std::io::BufReader::new(
        std::fs::File::open(info_plist).ok()?,
    ))
    .ok()?;
    let dict = value.as_dictionary()?;
    dict.get("CFBundleShortVersionString")
        .and_then(|v| v.as_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            dict.get("CFBundleVersion")
                .and_then(|v| v.as_string())
                .filter(|s| !s.is_empty())
        })
        .map(str::to_string)
}

/// Кандидаты относительного пути бандла от OC: как записано в конфиге плюс
/// типовые префиксы каталогов (регистр разделителя не важен — FAT).
fn bundle_candidates(p: &str) -> Vec<String> {
    let p = p.trim().replace('\\', "/").trim_start_matches("/ ").to_string();
    let mut out = vec![p.clone()];
    let lc = p.to_lowercase();
    if !lc.starts_with("kexts/") {
        out.push(format!("Kexts/{}", p));
    }
    out
}

/// Кандидаты относительного пути таблицы от OC
fn acpi_candidates(p: &str) -> Vec<String> {
    let p = p.trim().replace('\\', "/").trim_start_matches("/ ").to_string();
    let mut out = vec![p.clone()];
    let lc = p.to_lowercase();
    if !lc.starts_with("acpi/") {
        out.push(format!("ACPI/{}", p));
    }
    out
}

fn find_case_insensitive(dir: &Path, name: &str) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    rd.flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

/// Существует ли бандл/файл по относительному пути от OC (регистр не важен — FAT)
fn exists_rel(oc_dir: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = oc_dir.to_path_buf();
    let parts: Vec<&str> = rel.split('/').filter(|s| !s.is_empty()).collect();
    for (i, part) in parts.iter().enumerate() {
        let found = find_case_insensitive(&cur, part)?;
        if i + 1 == parts.len() {
            return Some(found);
        }
        cur = found;
    }
    None
}

/// Аудит config.plist по выбранной пользователем папке (том / EFI / OC)
pub fn audit_config(selected: &Path) -> Result<ConfigAudit, String> {
    let efi_base = resolve_efi_base(selected);
    let oc_dir = efi_base.join("OC");
    let config = oc_dir.join("config.plist");
    if !config.is_file() {
        return Ok(ConfigAudit {
            config_path: config.to_string_lossy().to_string(),
            found: false,
            kernel: vec![],
            acpi: vec![],
            orphans: vec![],
            orphan_tables: vec![],
            issues: vec![AuditIssue {
                severity: "error".into(),
                code: "config.not_found".into(),
                params: vec![],
            }],
            kexts_total: 0,
            kexts_enabled: 0,
            tables_total: 0,
        });
    }

    let root = Value::from_reader(std::io::BufReader::new(
        std::fs::File::open(&config).map_err(|e| format!("не открыть {}: {e}", config.display()))?,
    ))
    .map_err(|e| format!("не разобрать {}: {e}", config.display()))?
    .into_dictionary()
    .ok_or_else(|| format!("{} — не словарь plist", config.display()))?;

    let mut issues: Vec<AuditIssue> = Vec::new();

    // --- Kernel/Add ---
    let mut kernel: Vec<KextAudit> = Vec::new();
    let mut registered_bundles: Vec<String> = Vec::new();
    if let Some(add) = root
        .get("Kernel")
        .and_then(|v| v.as_dictionary())
        .and_then(|k| k.get("Add"))
        .and_then(|v| v.as_array())
    {
        for entry in add {
            let Some(d) = entry.as_dictionary() else {
                continue;
            };
            let bundle = d
                .get("BundlePath")
                .and_then(|v| v.as_string())
                .unwrap_or("")
                .to_string();
            if bundle.is_empty() {
                issues.push(AuditIssue {
                    severity: "warning".into(),
                    code: "kernel.empty_entry".into(),
                    params: vec![],
                });
                continue;
            }
            let enabled = d
                .get("Enabled")
                .and_then(|v| v.as_boolean())
                .unwrap_or(false);
            let disk_path = bundle_candidates(&bundle)
                .iter()
                .find_map(|rel| exists_rel(&oc_dir, rel))
                .filter(|p| p.is_dir());
            let on_disk = disk_path.is_some();
            let version = disk_path
                .and_then(|p| read_bundle_version(&p.join("Contents").join("Info.plist")));
            if !on_disk {
                issues.push(AuditIssue {
                    severity: if enabled { "error" } else { "warning" }.into(),
                    code: "kernel.missing".into(),
                    params: vec![bundle.clone()],
                });
            }
            registered_bundles.push(bundle.rsplit('/').next().unwrap_or(&bundle).to_string());
            kernel.push(KextAudit {
                bundle,
                enabled,
                on_disk,
                version,
            });
        }
    } else {
        issues.push(AuditIssue {
            severity: "warning".into(),
            code: "kernel.no_add".into(),
            params: vec![],
        });
    }

    // --- ACPI/Add ---
    let mut acpi: Vec<AcpiAudit> = Vec::new();
    let mut registered_tables: Vec<String> = Vec::new();
    if let Some(add) = root
        .get("ACPI")
        .and_then(|v| v.as_dictionary())
        .and_then(|k| k.get("Add"))
        .and_then(|v| v.as_array())
    {
        for entry in add {
            let Some(d) = entry.as_dictionary() else {
                continue;
            };
            let path = d
                .get("Path")
                .and_then(|v| v.as_string())
                .unwrap_or("")
                .to_string();
            if path.is_empty() {
                continue;
            }
            let enabled = d
                .get("Enabled")
                .and_then(|v| v.as_boolean())
                .unwrap_or(false);
            let on_disk = acpi_candidates(&path)
                .iter()
                .find_map(|rel| exists_rel(&oc_dir, rel))
                .map(|p| p.is_file())
                .unwrap_or(false);
            if !on_disk {
                issues.push(AuditIssue {
                    severity: if enabled { "error" } else { "warning" }.into(),
                    code: "acpi.missing".into(),
                    params: vec![path.clone()],
                });
            }
            registered_tables.push(path.rsplit('/').next().unwrap_or(&path).to_string());
            acpi.push(AcpiAudit {
                path,
                enabled,
                on_disk,
            });
        }
    }

    // --- Бандлы на диске без записи в конфиге ---
    let mut orphans: Vec<OrphanKext> = Vec::new();
    let kexts_dir = oc_dir.join("Kexts");
    if let Ok(rd) = std::fs::read_dir(&kexts_dir) {
        for e in rd.flatten() {
            if !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            if !name.to_lowercase().ends_with(".kext") {
                continue;
            }
            let registered = registered_bundles
                .iter()
                .any(|b| b.eq_ignore_ascii_case(&name));
            if !registered {
                orphans.push(OrphanKext {
                    bundle: name.clone(),
                    registered: false,
                });
                issues.push(AuditIssue {
                    severity: "warning".into(),
                    code: "kernel.orphan".into(),
                    params: vec![name],
                });
            }
        }
    }
    let acpi_dir = oc_dir.join("ACPI");
    let mut orphan_tables: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&acpi_dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.to_lowercase().ends_with(".aml") {
                continue;
            }
            if !registered_tables.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
                orphan_tables.push(name.clone());
                issues.push(AuditIssue {
                    severity: "warning".into(),
                    code: "acpi.orphan".into(),
                    params: vec![name],
                });
            }
        }
    }

    // --- UTBDefault vs UTBMap ---
    let utb_default_enabled = kernel
        .iter()
        .any(|k| k.enabled && k.bundle.to_lowercase().contains("utbdefault"));
    let utb_map_present = kernel
        .iter()
        .any(|k| k.bundle.to_lowercase().contains("utbmap.kext"));
    let utb_map_on_disk = exists_rel(&oc_dir, "Kexts/UTBMap.kext")
        .map(|p| p.is_dir())
        .unwrap_or(false);
    if utb_default_enabled && utb_map_present {
        issues.push(AuditIssue {
            severity: "error".into(),
            code: "utb.conflict".into(),
            params: vec![],
        });
    } else if utb_default_enabled {
        issues.push(AuditIssue {
            severity: "info".into(),
            code: "utb.default_active".into(),
            params: vec![],
        });
    }
    if !utb_map_present && utb_map_on_disk {
        issues.push(AuditIssue {
            severity: "warning".into(),
            code: "utb.map_unregistered".into(),
            params: vec![],
        });
    }

    let kexts_enabled = kernel.iter().filter(|k| k.enabled).count();

    Ok(ConfigAudit {
        config_path: config.to_string_lossy().to_string(),
        found: true,
        kexts_total: kernel.len(),
        kexts_enabled,
        tables_total: acpi.len(),
        kernel,
        acpi,
        orphans,
        orphan_tables,
        issues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use plist::Dictionary;

    fn plist_bytes(kernel_bundles: &[(&str, bool)], acpi_paths: &[(&str, bool)]) -> Vec<u8> {
        let mut kernel_add: Vec<Value> = Vec::new();
        for (b, en) in kernel_bundles {
            let mut d = Dictionary::new();
            d.insert("BundlePath".into(), Value::String(b.to_string()));
            d.insert("Enabled".into(), Value::Boolean(*en));
            d.insert(
                "ExecutablePath".into(),
                Value::String("Contents/MacOS/x".into()),
            );
            d.insert("PlistPath".into(), Value::String("Contents/Info.plist".into()));
            kernel_add.push(Value::Dictionary(d));
        }
        let mut acpi_add: Vec<Value> = Vec::new();
        for (p, en) in acpi_paths {
            let mut d = Dictionary::new();
            d.insert("Comment".into(), Value::String(String::new()));
            d.insert("Enabled".into(), Value::Boolean(*en));
            d.insert("Path".into(), Value::String(p.to_string()));
            acpi_add.push(Value::Dictionary(d));
        }
        let mut kernel = Dictionary::new();
        kernel.insert("Add".into(), Value::Array(kernel_add));
        let mut acpi = Dictionary::new();
        acpi.insert("Add".into(), Value::Array(acpi_add));
        let mut root = Dictionary::new();
        root.insert("ACPI".into(), Value::Dictionary(acpi));
        root.insert("Kernel".into(), Value::Dictionary(kernel));
        let mut buf = Vec::new();
        Value::Dictionary(root).to_writer_xml(&mut buf).unwrap();
        buf
    }

    fn tmp(label: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("openefi-cfgaud-{}-{}", label, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write_kext(dir: &Path, name: &str, version: &str) {
        let d = dir.join("Kexts").join(name);
        std::fs::create_dir_all(d.join("Contents/MacOS")).unwrap();
        let mut info = Dictionary::new();
        info.insert(
            "CFBundleShortVersionString".into(),
            Value::String(version.into()),
        );
        info.insert("CFBundleIdentifier".into(), Value::String("test".into()));
        let mut buf = Vec::new();
        Value::Dictionary(info).to_writer_xml(&mut buf).unwrap();
        std::fs::write(d.join("Contents/Info.plist"), buf).unwrap();
        std::fs::write(d.join("Contents/MacOS").join(name.trim_end_matches(".kext")), b"bin").unwrap();
    }

    #[test]
    fn test_audit_full_match() {
        let tmp = tmp("ok");
        let oc = tmp.join("EFI").join("OC");
        std::fs::create_dir_all(oc.join("ACPI")).unwrap();
        write_kext(&oc, "Lilu.kext", "1.7.0");
        write_kext(&oc, "VirtualSMC.kext", "1.3.6");
        std::fs::write(oc.join("ACPI/SSDT-PLUG-DRTNIA.aml"), b"ssdt").unwrap();
        std::fs::write(
            oc.join("config.plist"),
            plist_bytes(
                &[
                    ("Kexts/Lilu.kext", true),
                    ("Kexts/VirtualSMC.kext", true),
                ],
                &[("SSDT-PLUG-DRTNIA.aml", true)],
            ),
        )
        .unwrap();

        let a = audit_config(&tmp).unwrap();
        assert!(a.found);
        assert_eq!(a.kernel.len(), 2);
        assert!(a.kernel.iter().all(|k| k.on_disk));
        assert_eq!(a.kernel[0].version.as_deref(), Some("1.7.0"));
        assert_eq!(a.acpi.len(), 1);
        assert!(a.acpi[0].on_disk);
        assert!(a.orphans.is_empty());
        assert!(a.orphan_tables.is_empty());
        assert!(a.issues.is_empty(), "чистый конфиг без замечаний: {:?}", a.issues);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_audit_missing_and_orphans() {
        let tmp = tmp("broken");
        let oc = tmp.join("EFI").join("OC");
        std::fs::create_dir_all(oc.join("ACPI")).unwrap();
        // на диске есть Lilu и осиротевший USBToolBox, в конфиге — Lilu + отсутствующий VoodooI2C
        write_kext(&oc, "Lilu.kext", "1.6.9");
        write_kext(&oc, "USBToolBox.kext", "1.1.1");
        std::fs::write(oc.join("ACPI/SSDT-EC-USBX.aml"), b"ssdt").unwrap();
        std::fs::write(
            oc.join("config.plist"),
            plist_bytes(
                &[
                    ("Kexts/Lilu.kext", true),
                    ("Kexts/VoodooI2C.kext", false),
                ],
                &[("SSDT-NOPE.aml", false)],
            ),
        )
        .unwrap();

        let a = audit_config(&tmp).unwrap();
        // VoodooI2C выключен и отсутствует → warning; SSDT-NOPE выключена → warning
        assert!(a.issues.iter().any(|i| i.code == "kernel.missing" && i.severity == "warning"));
        assert!(a.issues.iter().any(|i| i.code == "acpi.missing" && i.severity == "warning"));
        assert_eq!(a.orphans.len(), 1);
        assert_eq!(a.orphans[0].bundle, "USBToolBox.kext");
        assert_eq!(a.orphan_tables.len(), 1);
        assert_eq!(a.orphan_tables[0], "SSDT-EC-USBX.aml");
        assert_eq!(a.kernel[1].version, None, "нет бандла — нет версии");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_audit_missing_enabled_is_error() {
        let tmp = tmp("err");
        let oc = tmp.join("EFI").join("OC");
        std::fs::create_dir_all(oc.join("ACPI")).unwrap();
        std::fs::write(
            oc.join("config.plist"),
            plist_bytes(&[("Kexts/Gone.kext", true)], &[]),
        )
        .unwrap();
        let a = audit_config(&tmp).unwrap();
        assert!(a
            .issues
            .iter()
            .any(|i| i.code == "kernel.missing" && i.severity == "error"));
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_audit_utb_default_and_map() {
        let tmp = tmp("utb");
        let oc = tmp.join("EFI").join("OC");
        std::fs::create_dir_all(oc.join("ACPI")).unwrap();
        write_kext(&oc, "UTBDefault.kext", "1.1.2");
        write_kext(&oc, "USBToolBox.kext", "1.1.2");
        // UTBMap лежит на диске, но не зарегистрирован; включён UTBDefault
        write_kext(&oc, "UTBMap.kext", "1.1.2");
        std::fs::write(
            oc.join("config.plist"),
            plist_bytes(
                &[
                    ("Kexts/USBToolBox.kext", true),
                    ("Kexts/UTBDefault.kext", true),
                ],
                &[],
            ),
        )
        .unwrap();
        let a = audit_config(&tmp).unwrap();
        assert!(a.issues.iter().any(|i| i.code == "utb.default_active"));
        assert!(a.issues.iter().any(|i| i.code == "utb.map_unregistered"));
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_audit_utb_conflict() {
        let tmp = tmp("utbc");
        let oc = tmp.join("EFI").join("OC");
        std::fs::create_dir_all(oc.join("ACPI")).unwrap();
        write_kext(&oc, "UTBDefault.kext", "1.1.2");
        write_kext(&oc, "UTBMap.kext", "1.1.2");
        std::fs::write(
            oc.join("config.plist"),
            plist_bytes(
                &[
                    ("Kexts/UTBDefault.kext", true),
                    ("Kexts/UTBMap.kext", true),
                ],
                &[],
            ),
        )
        .unwrap();
        let a = audit_config(&tmp).unwrap();
        assert!(a.issues.iter().any(|i| i.code == "utb.conflict" && i.severity == "error"));
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_audit_no_config() {
        let tmp = tmp("empty");
        std::fs::create_dir_all(tmp.join("EFI").join("OC")).unwrap();
        let a = audit_config(&tmp).unwrap();
        assert!(!a.found);
        assert!(a.issues.iter().any(|i| i.code == "config.not_found"));
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_read_bundle_version_fallbacks() {
        let tmp = tmp("ver");
        let d = tmp.join("X.kext").join("Contents");
        std::fs::create_dir_all(&d).unwrap();
        // только CFBundleVersion
        let mut info = Dictionary::new();
        info.insert("CFBundleVersion".into(), Value::String("2.0".into()));
        let mut buf = Vec::new();
        Value::Dictionary(info).to_writer_xml(&mut buf).unwrap();
        std::fs::write(d.join("Info.plist"), buf).unwrap();
        assert_eq!(read_bundle_version(&d.join("Info.plist")).as_deref(), Some("2.0"));
        // пустая версия → None
        let mut info = Dictionary::new();
        info.insert("CFBundleShortVersionString".into(), Value::String("".into()));
        let mut buf = Vec::new();
        Value::Dictionary(info).to_writer_xml(&mut buf).unwrap();
        std::fs::write(d.join("Info.plist"), buf).unwrap();
        assert_eq!(read_bundle_version(&d.join("Info.plist")), None);
        std::fs::remove_dir_all(&tmp).ok();
    }
}
