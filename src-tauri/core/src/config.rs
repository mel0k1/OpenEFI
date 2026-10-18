//! Генерация config.plist для OpenCore на базе эталонного Sample.plist из OpenCorePkg.
//! Заполняем ACPI/Add (SSDT), Kernel/Add (кексты в правильном порядке),
//! DeviceProperties (iGPU framebuffer), NVRAM boot-args и PlatformInfo/Generic (SMBIOS).

use plist::{Dictionary, Value};

use crate::hardware::HardwareReport;
use crate::kexts;
use crate::ssdts::SsdtEntry;
use crate::smbios::SmbiosData;

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .filter_map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok())
        .collect()
}

const SAMPLE: &str = include_str!("sample_config.plist");

/// Входные данные для генерации config.plist
pub struct ConfigInput<'a> {
    pub report: &'a HardwareReport,
    pub kexts: &'a [kexts::KextEntry],
    pub ssdts: &'a [SsdtEntry],
    pub smbios: &'a SmbiosData,
    pub darwin: u32,
}

/// Топологическая сортировка кекстов по зависимостям (Lilu/VirtualSMC раньше плагинов).
/// Возвращает (имя, min_darwin, max_darwin).
fn ordered_kexts(selected: &[kexts::KextEntry]) -> Vec<(String, Option<u32>, Option<u32>)> {
    let mut order: Vec<(String, Option<u32>, Option<u32>)> = Vec::new();
    let mut placed: Vec<String> = Vec::new();

    fn deps_ok(k: &kexts::KextEntry, placed: &[String]) -> bool {
        k.requires.iter().all(|r| placed.iter().any(|p| p == r))
    }

    // Стабильный алгоритм: проходим список, пока всё не разместим
    let mut pending: Vec<kexts::KextEntry> = selected.to_vec();
    while !pending.is_empty() {
        let mut progressed = false;
        pending.retain(|k| {
            if deps_ok(k, &placed) {
                let def = kexts::kext_bounds(&k.name);
                order.push((k.name.clone(), def.0, def.1));
                placed.push(k.name.clone());
                progressed = true;
                false
            } else {
                true
            }
        });
        if !progressed {
            // Циклические зависимости — добавляем остаток как есть
            for k in pending {
                let def = kexts::kext_bounds(&k.name);
                order.push((k.name.clone(), def.0, def.1));
            }
            break;
        }
    }
    order
}

fn darwin_kernel_str(d: Option<u32>) -> String {
    match d {
        Some(v) if v > 0 && v < 100 => format!("{v}.00"),
        _ => String::new(),
    }
}

/// AAPL,ig-platform-id для Intel iGPU (десктоп), по гайду Dortania
fn intel_igpu_platform_id(codename: &str) -> Option<[u8; 4]> {
    match codename {
        "Coffee Lake-S" => Some([0x07, 0x00, 0x9B, 0x3E]), // 0x3E9B0007
        "Comet Lake-S" => Some([0x00, 0x00, 0x9B, 0x3E]),  // 0x3E9B0000
        "Kaby Lake" | "Kaby Lake-S" => Some([0x00, 0x00, 0x16, 0x59]), // 0x59160000
        "Skylake" | "Skylake-S" => Some([0x00, 0x00, 0x12, 0x19]), // 0x19120000
        "Haswell" => Some([0x03, 0x00, 0x22, 0x0D]),       // 0x0D220003
        _ => None,
    }
}

pub fn generate_config(input: &ConfigInput) -> Result<Vec<u8>, String> {
    let mut root = Value::from_reader(std::io::Cursor::new(SAMPLE))
        .map_err(|e| format!("не удалось разобрать Sample.plist: {e}"))?
        .into_dictionary()
        .ok_or("Sample.plist — не словарь")?;

    // Убираем предупреждения сэмпла
    for key in ["#WARNING - 1", "#WARNING - 2", "#WARNING - 3", "#WARNING - 4"] {
        root.remove(key);
    }

    // --- ACPI / Add ---
    let mut acpi_add = Vec::new();
    for s in input.ssdts {
        let mut d = Dictionary::new();
        d.insert("Comment".into(), Value::String(s.description.clone()));
        d.insert("Enabled".into(), Value::Boolean(true));
        d.insert("Path".into(), Value::String(s.file.clone()));
        acpi_add.push(Value::Dictionary(d));
    }
    if let Some(acpi) = root.get_mut("ACPI").and_then(|v| v.as_dictionary_mut()) {
        acpi.insert("Add".into(), Value::Array(acpi_add));
    }

    // --- Booter / Quirks (SetupVirtualMap=false на новых платформах) ---
    let codename = input.report.cpu.codename.clone().unwrap_or_default();
    if let Some(booter) = root.get_mut("Booter").and_then(|v| v.as_dictionary_mut()) {
        if let Some(quirks) = booter.get_mut("Quirks").and_then(|v| v.as_dictionary_mut()) {
            let modern = ["Comet Lake", "Alder Lake", "Raptor Lake", "Meteor Lake", "Arrow Lake"]
                .iter()
                .any(|m| codename.contains(m));
            if modern {
                quirks.insert("SetupVirtualMap".into(), Value::Boolean(false));
            }
        }
    }

    // --- DeviceProperties: очищаем placeholder сэмпла, добавляем Intel iGPU framebuffer ---
    let mut dp_add = Dictionary::new();
    if input.report.cpu.manufacturer == "Intel" {
        let is_desktop = !input.report.platform.contains("Laptop");
        let has_igpu = input
            .report
            .gpus
            .iter()
            .any(|g| g.vendor_id == "8086" && g.device_type == "Integrated GPU");
        if is_desktop && has_igpu {
            if let Some(pid) = intel_igpu_platform_id(&codename) {
                let mut igpu_props = Dictionary::new();
                igpu_props.insert("AAPL,ig-platform-id".into(), Value::Data(pid.to_vec()));
                dp_add.insert("PciRoot(0x0)/Pci(0x2,0x0)".into(), Value::Dictionary(igpu_props));
            }
        }
    }
    if let Some(dp) = root
        .get_mut("DeviceProperties")
        .and_then(|v| v.as_dictionary_mut())
    {
        dp.insert("Add".into(), Value::Dictionary(dp_add));
    }

    // --- Kernel / Add ---
    // Системные драйверы (non_efi) в EFI не попадают — исключаем из Kernel/Add
    let efi_kexts: Vec<kexts::KextEntry> = input
        .kexts
        .iter()
        .filter(|k| k.selected && !k.non_efi)
        .cloned()
        .collect();
    let mut kernel_add = Vec::new();
    for (name, min, max) in ordered_kexts(&efi_kexts) {
        let mut d = Dictionary::new();
        d.insert("Arch".into(), Value::String("x86_64".into()));
        // Кексты лежат в EFI/OC/Kexts, путь в BundlePath задаётся от папки OC
        d.insert("BundlePath".into(), Value::String(format!("Kexts/{name}.kext")));
        d.insert("Comment".into(), Value::String(String::new()));
        d.insert("Enabled".into(), Value::Boolean(true));
        d.insert(
            "ExecutablePath".into(),
            Value::String(format!("Contents/MacOS/{name}")),
        );
        d.insert("MaxKernel".into(), Value::String(darwin_kernel_str(max)));
        d.insert("MinKernel".into(), Value::String(darwin_kernel_str(min)));
        d.insert(
            "PlistPath".into(),
            Value::String("Contents/Info.plist".into()),
        );
        kernel_add.push(Value::Dictionary(d));
    }
    if let Some(kernel) = root.get_mut("Kernel").and_then(|v| v.as_dictionary_mut()) {
        kernel.insert("Add".into(), Value::Array(kernel_add));

        // XhciPortLimit: в macOS 11.3+ работает некорректно
        if let Some(quirks) = kernel.get_mut("Quirks").and_then(|v| v.as_dictionary_mut()) {
            quirks.insert(
                "XhciPortLimit".into(),
                Value::Boolean(input.darwin < 20),
            );
        }
    }

    // --- NVRAM / boot-args ---
    if let Some(nvram) = root.get_mut("NVRAM").and_then(|v| v.as_dictionary_mut()) {
        if let Some(add) = nvram.get_mut("Add").and_then(|v| v.as_dictionary_mut()) {
            if let Some(guide) = add
                .get_mut("7C436110-AB2A-4BBB-A880-FE41995C9F82")
                .and_then(|v| v.as_dictionary_mut())
            {
                guide.insert(
                    "boot-args".into(),
                    Value::String("-v keepsyms=1".into()),
                );
            }
        }
    }

    // --- PlatformInfo / Generic ---
    if let Some(pi) = root
        .get_mut("PlatformInfo")
        .and_then(|v| v.as_dictionary_mut())
    {
        pi.insert("UpdateSMBIOSMode".into(), Value::String("Create".into()));
        if let Some(generic) = pi.get_mut("Generic").and_then(|v| v.as_dictionary_mut()) {
            generic.insert(
                "SystemProductName".into(),
                Value::String(input.smbios.model.clone()),
            );
            generic.insert(
                "SystemSerialNumber".into(),
                Value::String(input.smbios.serial.clone()),
            );
            generic.insert("MLB".into(), Value::String(input.smbios.mlb.clone()));
            generic.insert(
                "SystemUUID".into(),
                Value::String(input.smbios.uuid.clone().to_uppercase()),
            );
            generic.insert("ROM".into(), Value::Data(hex_to_bytes(&input.smbios.rom)));
        }
    }

    let mut buf = Vec::new();
    Value::Dictionary(root)
        .to_writer_xml(&mut buf)
        .map_err(|e| format!("не удалось сериализовать config.plist: {e}"))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::{CpuInfo, GpuInfo};
    use crate::ssdts;

    fn sample_report() -> HardwareReport {
        HardwareReport {
            os: "windows".into(),
            platform: "Desktop".into(),
            board_name: "Z390 AORUS".into(),
            cpu: CpuInfo {
                manufacturer: "Intel".into(),
                name: "Intel Core i5-9400".into(),
                codename: Some("Coffee Lake-S".into()),
                cores: 6,
                threads: 6,
                avx2: true,
            },
            memory_gb: 16.0,
            gpus: vec![GpuInfo {
                name: "Intel UHD 630".into(),
                vendor_id: "8086".into(),
                device_id: "3e92".into(),
                device_type: "Integrated GPU".into(),
                codename: Some("Coffee Lake-S".into()),
                max_darwin: None,
                note: None,
            }],
            network: vec![],
            audio: vec![],
            bluetooth: vec![],
            warnings: vec![],
        }
    }

    fn parse_xml(bytes: &[u8]) -> Dictionary {
        Value::from_reader(std::io::Cursor::new(bytes))
            .unwrap()
            .into_dictionary()
            .unwrap()
    }

    fn path<'a>(root: &'a Dictionary, keys: &[&str]) -> &'a Value {
        let mut cur = root.get(keys[0]).unwrap_or_else(|| panic!("нет ключа {}", keys[0]));
        for k in &keys[1..] {
            cur = cur
                .as_dictionary()
                .unwrap_or_else(|| panic!("{} — не словарь", keys.join("/")))
                .get(*k)
                .unwrap_or_else(|| panic!("нет ключа {} в {}", k, keys.join("/")));
        }
        cur
    }

    #[test]
    fn test_generate_config_coffee_lake() {
        let report = sample_report();
        let kexts_sel: Vec<kexts::KextEntry> = {
            let mut all = kexts::match_kexts(&report, 22);
            all.retain(|k| k.selected);
            all
        };
        let ssdts_sel = ssdts::recommend(&report);
        let smbios = crate::smbios::generate("iMac19,1");
        let input = ConfigInput {
            report: &report,
            kexts: &kexts_sel,
            ssdts: &ssdts_sel,
            smbios: &smbios,
            darwin: 22,
        };
        let bytes = generate_config(&input).unwrap();
        let root = parse_xml(&bytes);

        // Секции на месте
        for section in ["ACPI", "Booter", "DeviceProperties", "Kernel", "Misc", "NVRAM", "PlatformInfo", "UEFI"] {
            assert!(root.contains_key(section), "нет секции {section}");
        }
        // Предупреждения сэмпла удалены
        assert!(!root.keys().any(|k| k.starts_with("#WARNING")));

        // ACPI.Add содержит SSDT-PLUG
        let acpi_add = path(&root, &["ACPI", "Add"]).as_array().unwrap();
        assert!(!acpi_add.is_empty());
        let paths: Vec<&str> = acpi_add
            .iter()
            .filter_map(|v| v.as_dictionary())
            .filter_map(|d| d.get("Path").and_then(|p| p.as_string()))
            .collect();
        assert!(paths.contains(&"SSDT-PLUG-DRTNIA.aml"));

        // Kernel.Add: Lilu первым, VirtualSMC до SMCProcessor
        let bundles: Vec<&str> = path(&root, &["Kernel", "Add"])
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_dictionary())
            .filter_map(|d| d.get("BundlePath").and_then(|b| b.as_string()))
            .collect();
        assert!(!bundles.is_empty());
        assert_eq!(bundles[0], "Kexts/Lilu.kext", "Lilu должен быть первым: {bundles:?}");
        let vs = bundles.iter().position(|b| *b == "Kexts/VirtualSMC.kext").unwrap();
        let sp = bundles.iter().position(|b| *b == "Kexts/SMCProcessor.kext").unwrap();
        assert!(vs < sp, "VirtualSMC раньше плагинов");

        // DeviceProperties: ig-platform-id 0x3E9B0007
        let dp = path(&root, &["DeviceProperties", "Add", "PciRoot(0x0)/Pci(0x2,0x0)", "AAPL,ig-platform-id"])
            .as_data()
            .unwrap();
        assert_eq!(dp, &[0x07, 0x00, 0x9B, 0x3E]);

        // PlatformInfo
        let generic = path(&root, &["PlatformInfo", "Generic"]).as_dictionary().unwrap();
        assert_eq!(generic.get("SystemProductName").unwrap().as_string().unwrap(), "iMac19,1");
        assert!(!generic.get("SystemSerialNumber").unwrap().as_string().unwrap().is_empty());
        assert_eq!(generic.get("ROM").unwrap().as_data().unwrap().len(), 6);

        // boot-args
        assert_eq!(
            path(&root, &["NVRAM", "Add", "7C436110-AB2A-4BBB-A880-FE41995C9F82", "boot-args"])
                .as_string()
                .unwrap(),
            "-v keepsyms=1"
        );

        // XhciPortLimit false для darwin 22
        assert_eq!(path(&root, &["Kernel", "Quirks", "XhciPortLimit"]).as_boolean(), Some(false));
    }

    #[test]
    fn test_kext_order_dependencies() {
        let report = sample_report();
        let mut all = kexts::match_kexts(&report, 22);
        all.retain(|k| k.selected);
        let order = ordered_kexts(&all);
        let names: Vec<&str> = order.iter().map(|(n, _, _)| n.as_str()).collect();
        let lilu = names.iter().position(|n| *n == "Lilu").unwrap();
        let wEG = names.iter().position(|n| *n == "WhateverGreen").unwrap();
        assert!(lilu < wEG, "Lilu раньше WhateverGreen: {names:?}");
    }

    #[test]
    fn test_amd_config_no_igpu_props() {
        let mut report = sample_report();
        report.cpu.manufacturer = "AMD".into();
        report.cpu.codename = Some("Vermeer".into());
        report.gpus.clear();
        let kexts_sel: Vec<kexts::KextEntry> = {
            let mut all = kexts::match_kexts(&report, 22);
            all.retain(|k| k.selected);
            all
        };
        let ssdts_sel = ssdts::recommend(&report);
        let smbios = crate::smbios::generate("iMacPro1,1");
        let input = ConfigInput {
            report: &report,
            kexts: &kexts_sel,
            ssdts: &ssdts_sel,
            smbios: &smbios,
            darwin: 22,
        };
        let bytes = generate_config(&input).unwrap();
        let root = parse_xml(&bytes);
        // Для AMD без iGPU DeviceProperties/Add должен остаться пустым
        let dp_add = path(&root, &["DeviceProperties", "Add"]).as_dictionary().unwrap();
        assert!(dp_add.is_empty(), "не должно быть iGPU-свойств на AMD");
        // PLUG не должно быть в ACPI
        let paths: Vec<&str> = path(&root, &["ACPI", "Add"])
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_dictionary())
            .filter_map(|d| d.get("Path").and_then(|p| p.as_string()))
            .collect();
        assert!(!paths.iter().any(|p| p.contains("PLUG")));
    }

    #[test]
    fn test_non_efi_kext_excluded_from_config() {
        let report = sample_report();
        let mut all = kexts::match_kexts(&report, 22);
        // помечаем NVIDIA Driver выбранным (системный драйвер)
        for k in &mut all {
            if k.non_efi {
                k.selected = true;
            }
        }
        assert!(all.iter().any(|k| k.non_efi && k.selected), "в каталоге должен быть non-EFI драйвер");
        let ssdts_sel = ssdts::recommend(&report);
        let smbios = crate::smbios::generate("iMac19,1");
        let input = ConfigInput {
            report: &report,
            kexts: &all,
            ssdts: &ssdts_sel,
            smbios: &smbios,
            darwin: 22,
        };
        let bytes = generate_config(&input).unwrap();
        let root = parse_xml(&bytes);
        let bundles: Vec<&str> = path(&root, &["Kernel", "Add"])
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_dictionary())
            .filter_map(|d| d.get("BundlePath").and_then(|b| b.as_string()))
            .collect();
        assert!(
            !bundles.iter().any(|b| b.contains("NVIDIA Driver")),
            "системный драйвер не должен попадать в Kernel/Add: {bundles:?}"
        );
    }
}
