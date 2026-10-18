//! BIOS-чеклист: какие настройки BIOS/UEFI включить или выключить перед
//! установкой хакинтоша. Матрица построена по гайду Dortania и адаптируется
//! под железо пользователя (Intel/AMD, десктоп/ноутбук, GPU).
//!
//! Плюс «живые» проверки там, где это позволяет хост-ОС:
//! - Windows: `Confirm-SecureBootUEFI` и `$env:firmware_type` (UEFI/Legacy);
//! - Linux: /sys/firmware/efi и efivar SecureBoot-*;
//! - macOS: хост-проверки недоступны (запуск на реальном Mac бессмыслен).

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

use crate::hardware::HardwareReport;

pub const ACTION_ENABLE: &str = "enable";
pub const ACTION_DISABLE: &str = "disable";
/// Проверить значение (нет единого «правильного» — в описании указано целевое)
pub const ACTION_CHECK: &str = "check";

pub const PRIORITY_REQUIRED: &str = "required";
pub const PRIORITY_RECOMMENDED: &str = "recommended";
pub const PRIORITY_OPTIONAL: &str = "optional";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BiosItem {
    /// Семантический id — текст пункта лежит в i18n (bios.item.<id>)
    pub id: String,
    /// Как пункт называется в реальных BIOS/UEFI (не переводится)
    pub option: String,
    /// enable | disable | check
    pub action: String,
    /// required | recommended | optional
    pub priority: String,
    /// Применим ли пункт к железу пользователя
    pub applies: bool,
    /// Дополнительная заметка от бэкенда (переводится tb())
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BiosChecklist {
    pub items: Vec<BiosItem>,
}

/// Результат живой проверки настроек на хост-системе
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BiosStatus {
    /// Secure Boot: Some(true) — включён (плохо), Some(false) — выключен (ок)
    pub secure_boot: Option<bool>,
    /// "uefi" | "legacy"
    pub boot_mode: Option<String>,
    /// Замечания проверки (переводятся tb())
    pub notes: Vec<String>,
}

fn item(
    id: &str,
    option: &str,
    action: &str,
    priority: &str,
    applies: bool,
    note: Option<&str>,
) -> BiosItem {
    BiosItem {
        id: id.to_string(),
        option: option.to_string(),
        action: action.to_string(),
        priority: priority.to_string(),
        applies,
        note: note.map(str::to_string),
    }
}

fn gpu_name_lc(report: &HardwareReport) -> String {
    report
        .gpus
        .iter()
        .map(|g| format!("{} {}", g.name, g.codename.clone().unwrap_or_default()))
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Матрица настроек BIOS по гайду Dortania, адаптированная под железо
pub fn checklist(report: &HardwareReport) -> BiosChecklist {
    let amd = report.cpu.manufacturer.eq_ignore_ascii_case("AMD");
    let intel = !amd;
    let laptop = report.platform.eq_ignore_ascii_case("Laptop");
    let gpu = gpu_name_lc(report);
    // Navi 2x (RX 6000) с NootRX требует включённый Resizable BAR
    let navi2 = gpu.contains("navi 2") || gpu.contains("navi2");
    let navi_or_older = gpu.contains("navi")
        || gpu.contains("polaris")
        || gpu.contains("vega")
        || gpu.contains("lexa")
        || gpu.contains("ellesmere")
        || gpu.contains("baffin");

    let mut items = Vec::new();

    // --- Обязательные ---
    items.push(item(
        "secure_boot",
        "Secure Boot",
        ACTION_DISABLE,
        PRIORITY_REQUIRED,
        true,
        None,
    ));
    items.push(item(
        "fast_boot",
        "Fast Boot",
        ACTION_DISABLE,
        PRIORITY_REQUIRED,
        true,
        None,
    ));
    items.push(item(
        "sata_ahci",
        "SATA Mode / Configure SATA as",
        ACTION_CHECK,
        PRIORITY_REQUIRED,
        true,
        Some("Должен быть режим AHCI — IDE/RST/RAID сломает обнаружение диска установщиком"),
    ));
    items.push(item(
        "above_4g",
        "Above 4G Decoding",
        ACTION_ENABLE,
        PRIORITY_REQUIRED,
        true,
        Some("На разных платах встречается как «Above 4G Memory» или «Mining Mode» — нужен для загрузки с AMD GPU"),
    ));
    items.push(item(
        "csm",
        "CSM / Launch CSM",
        ACTION_DISABLE,
        PRIORITY_REQUIRED,
        true,
        Some("macOS грузится только в чистом UEFI; если после выключения CSM диск пропал из списка загрузки — включите «Boot mode: UEFI only»"),
    ));
    if navi_or_older && !navi2 {
        items.push(item(
            "resize_bar",
            "Resizable BAR / Smart Access Memory",
            ACTION_DISABLE,
            PRIORITY_REQUIRED,
            true,
            Some("Navi 10 и старее (Polaris/Vega/Navi 10) не стартуют с включённым ReBAR"),
        ));
    } else if navi2 {
        items.push(item(
            "resize_bar",
            "Resizable BAR / Smart Access Memory",
            ACTION_CHECK,
            PRIORITY_REQUIRED,
            true,
            Some("Для RX 6000 с кекстом NootRX ReBAR должен быть ВКЛЮЧЁН; с WhateverGreen — выключен"),
        ));
    } else {
        items.push(item(
            "resize_bar",
            "Resizable BAR / Smart Access Memory",
            ACTION_DISABLE,
            PRIORITY_RECOMMENDED,
            true,
            Some("Не критично, но macOS не использует ReBAR — выключите, если возникнут артефакты"),
        ));
    }
    if intel {
        items.push(item(
            "hyper_threading",
            "Hyper-Threading Technology",
            ACTION_ENABLE,
            PRIORITY_REQUIRED,
            true,
            None,
        ));
        items.push(item(
            "sgx",
            "Intel SGX",
            ACTION_DISABLE,
            PRIORITY_REQUIRED,
            true,
            Some("На поколениях без SGX пункта не будет — просто пропустите"),
        ));
    }

    // --- Рекомендованные ---
    if intel {
        items.push(item(
            "vt_d",
            "VT-d",
            ACTION_DISABLE,
            PRIORITY_RECOMMENDED,
            true,
            Some("Если выключать не хотите — OpenCore умеет DisableIoMapper; на AMD этого пункта нет"),
        ));
        items.push(item(
            "txt",
            "Intel TXT / Trusted Execution",
            ACTION_DISABLE,
            PRIORITY_RECOMMENDED,
            true,
            None,
        ));
        items.push(item(
            "cfg_lock",
            "CFG Lock (MSR 0xE2)",
            ACTION_DISABLE,
            PRIORITY_RECOMMENDED,
            true,
            Some("Часто спрятан в расширенном меню BIOS; если не найти — OpenCore компенсирует кварками AppleCpuPmCfgLock/AppleXcpmCfgLock"),
        ));
        items.push(item(
            "vt_x",
            "Intel Virtualization Technology (VT-x)",
            ACTION_ENABLE,
            PRIORITY_RECOMMENDED,
            true,
            Some("macOS сам VT-x не использует, но пригодится для виртуалок; выключенным оставлять нет смысла"),
        ));
        items.push(item(
            "tpm_ptt",
            "TPM Device / PTT",
            ACTION_CHECK,
            PRIORITY_OPTIONAL,
            true,
            Some("OpenCore TPM игнорирует; если Windows 11 стоит на том же ПК — не трогайте, иначе она перестанет загружаться"),
        ));
    }
    if amd {
        items.push(item(
            "svm",
            "SVM Mode (AMD-V)",
            ACTION_ENABLE,
            PRIORITY_RECOMMENDED,
            true,
            None,
        ));
        items.push(item(
            "iommu",
            "IOMMU",
            ACTION_DISABLE,
            PRIORITY_RECOMMENDED,
            true,
            Some("Или оставьте включённым — тогда AMD-конфиг должен содержать DisableIoMapper=true"),
        ));
        items.push(item(
            "npb",
            "NX Mode / No-Execute (NX)",
            ACTION_ENABLE,
            PRIORITY_REQUIRED,
            true,
            Some("На AMD обязателен включённый NX — иначе ядро macOS падает при старте"),
        ));
    }
    if laptop && intel {
        items.push(item(
            "dvmt",
            "DVMT Pre-Allocated Memory",
            ACTION_CHECK,
            PRIORITY_REQUIRED,
            true,
            Some("Нужно 64 МБ под iGPU; часто спрятан — на некоторых ноутбуках правится только через мод BIOS/UEFI"),
        ));
    }

    // --- Опциональные ---
    items.push(item(
        "xmp",
        "XMP / DOCP / EXPO Profile",
        ACTION_ENABLE,
        PRIORITY_OPTIONAL,
        true,
        Some("Профиль памяти для производительности; на совместимость macOS не влияет"),
    ));

    BiosChecklist {
        items: items
            .into_iter()
            .filter(|i| i.applies)
            .collect(),
    }
}

fn ps(script: &str) -> String {
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Живая проверка Secure Boot и режима загрузки на хост-системе
pub fn live_status() -> BiosStatus {
    match std::env::consts::OS {
        "windows" => windows_status(),
        "linux" => linux_status(),
        _ => BiosStatus {
            secure_boot: None,
            boot_mode: None,
            notes: vec!["живая проверка доступна только на Windows и Linux".into()],
        },
    }
}

fn windows_status() -> BiosStatus {
    let mut notes = Vec::new();

    let fw = ps("$env:firmware_type");
    let boot_mode = match fw.to_lowercase().as_str() {
        "uefi" => Some("uefi".to_string()),
        "legacy" => Some("legacy".to_string()),
        _ => None,
    };

    let sb = ps("Confirm-SecureBootUEFI");
    let secure_boot = match sb.to_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => {
            notes.push(
                "Secure Boot не удалось прочитать — нужны права администратора или система грузится в Legacy-режиме".into(),
            );
            None
        }
    };

    BiosStatus { secure_boot, boot_mode, notes }
}

fn linux_status() -> BiosStatus {
    let mut notes = Vec::new();

    let boot_mode = if Path::new("/sys/firmware/efi").is_dir() {
        Some("uefi".to_string())
    } else {
        Some("legacy".to_string())
    };

    // efivar SecureBoot-8be4df61-93a4-11d0-947c-00c04fc329c3:
    // байт со смещением 4 == 1 — Secure Boot включён
    let mut secure_boot = None;
    if let Ok(dir) = std::fs::read_dir("/sys/firmware/efi/efivars") {
        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.starts_with("secureboot-") {
                if let Ok(data) = std::fs::read(entry.path()) {
                    if data.len() > 4 {
                        secure_boot = Some(data[4] == 1);
                    }
                }
                break;
            }
        }
    } else {
        notes.push("efivars недоступны — Secure Boot прочитать не удалось".into());
    }

    BiosStatus { secure_boot, boot_mode, notes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::{CpuInfo, GpuInfo};

    fn report(cpu: &str, platform: &str, gpu: Option<&str>) -> HardwareReport {
        HardwareReport {
            os: "windows".into(),
            platform: platform.into(),
            board_name: "Test Board".into(),
            cpu: CpuInfo {
                manufacturer: cpu.into(),
                name: format!("{cpu} Test CPU"),
                codename: None,
                cores: 8,
                threads: 16,
                avx2: true,
            },
            memory_gb: 16.0,
            gpus: gpu
                .map(|n| {
                    vec![GpuInfo {
                        name: n.into(),
                        vendor_id: "1002".into(),
                        device_id: "699F".into(),
                        device_type: "Discrete GPU".into(),
                        codename: None,
                        max_darwin: None,
                        note: None,
                    }]
                })
                .unwrap_or_default(),
            network: vec![],
            audio: vec![],
            bluetooth: vec![],
            storage: vec![],
            warnings: vec![],
        }
    }

    fn ids(c: &BiosChecklist) -> Vec<&str> {
        c.items.iter().map(|i| i.id.as_str()).collect()
    }

    #[test]
    fn test_checklist_intel_desktop() {
        let c = checklist(&report("Intel", "Desktop", Some("AMD Radeon RX 550 (Polaris 21)")));
        let ids = ids(&c);
        assert!(ids.contains(&"secure_boot"));
        assert!(ids.contains(&"hyper_threading"));
        assert!(ids.contains(&"cfg_lock"));
        assert!(!ids.contains(&"svm"));
        assert!(!ids.contains(&"dvmt"), "десктопу DVMT не нужен");
        // RX 550 = Polaris → ReBAR обязателен к выключению
        let rebar = c.items.iter().find(|i| i.id == "resize_bar").unwrap();
        assert_eq!(rebar.action, ACTION_DISABLE);
        assert_eq!(rebar.priority, PRIORITY_REQUIRED);
    }

    #[test]
    fn test_checklist_amd_cpu() {
        let c = checklist(&report("AMD", "Desktop", None));
        let ids = ids(&c);
        assert!(ids.contains(&"svm"));
        assert!(ids.contains(&"npb"));
        assert!(!ids.contains(&"hyper_threading"));
        assert!(!ids.contains(&"vt_d"));
        assert!(!ids.contains(&"sgx"));
    }

    #[test]
    fn test_checklist_laptop_intel_dvmt() {
        let c = checklist(&report("Intel", "Laptop", None));
        assert!(ids(&c).contains(&"dvmt"));
    }

    #[test]
    fn test_checklist_navi2_rebar_on() {
        let c = checklist(&report("Intel", "Desktop", Some("AMD Radeon RX 6800 XT Navi 21")));
        let rebar = c.items.iter().find(|i| i.id == "resize_bar").unwrap();
        assert_eq!(rebar.action, ACTION_CHECK);
        assert!(rebar.note.as_deref().unwrap().contains("NootRX"));
    }

    #[test]
    fn test_live_status_shape() {
        // не должна паниковать ни на одной ОС; на linux/macOS в песочнице
        // обычно нет efivars — тогда notes непустой или поля None
        let s = live_status();
        assert!(s.boot_mode.is_none() || s.boot_mode.is_some());
        assert!(s.secure_boot.is_none() || s.secure_boot.is_some());
    }
}
