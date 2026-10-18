//! Подбор SSDT по железу — матрица из гайда Dortania
//! https://dortania.github.io/Getting-Started-With-ACPI/

use serde::Serialize;

use crate::hardware::HardwareReport;

const COMPILED_URL: &str = "https://github.com/dortania/Getting-Started-With-ACPI/tree/master/extra-files/compiled";
const GUIDE_URL: &str = "https://dortania.github.io/Getting-Started-With-ACPI/";

#[derive(Serialize, Clone, Debug)]
pub struct SsdtEntry {
    /// Имя без расширения, напр. "SSDT-PLUG-DRTNIA"
    pub name: String,
    /// Имя файла для EFI/OC/ACPI и ACPI/Add/Path
    pub file: String,
    pub description: String,
    /// Обязателен для данной конфигурации или опционален
    pub kind: String,
    pub url: String,
    /// Включена по умолчанию
    pub selected: bool,
}

fn ssdt(name: &str, description: &str, kind: &str) -> SsdtEntry {
    SsdtEntry {
        name: name.to_string(),
        file: format!("{name}.aml"),
        description: description.to_string(),
        kind: kind.to_string(),
        url: COMPILED_URL.to_string(),
        selected: kind != "Опциональна",
    }
}

/// Intel 8-е поколение (Coffee Lake) и новее — платформы с AWAC-RTC
fn intel_has_awac(codename: &str) -> bool {
    const MODERN: &[&str] = &[
        "Coffee Lake", "Whiskey Lake", "Amber Lake", "Cannon Lake", "Comet Lake",
        "Ice Lake", "Lakefield", "Rocket Lake", "Tiger Lake", "Alder Lake",
        "Raptor Lake", "Meteor Lake", "Arrow Lake", "Lunar Lake",
    ];
    MODERN.iter().any(|m| codename.contains(m))
}

/// Intel 300/400-серия чипсетов — потеряли native NVRAM (нужен PMC)
fn intel_needs_pmc(codename: &str) -> bool {
    codename.contains("Coffee Lake") || codename.contains("Comet Lake")
}

/// X99 / Haswell-E / Broadwell-E — UNC фиксы
fn intel_is_hedt_x99(codename: &str) -> bool {
    codename.contains("Haswell-E") || codename.contains("Broadwell-E")
}

pub fn recommend(report: &HardwareReport) -> Vec<SsdtEntry> {
    let mut out = Vec::new();
    let is_intel = report.cpu.manufacturer == "Intel";
    let is_laptop = report.platform.contains("Laptop");
    let codename = report.cpu.codename.clone().unwrap_or_default();

    // SSDT-PLUG — управление питанием CPU. Только Intel (у AMD нет XCPM).
    if is_intel {
        out.push(ssdt(
            "SSDT-PLUG-DRTNIA",
            "Управление питанием процессора (XCPM) — обязательна на всех Intel",
            "Обязательна",
        ));
    }

    // SSDT-EC-USBX — фейковый контроллер EC + питание USB 3.0
    if is_laptop {
        out.push(ssdt(
            "SSDT-EC-USBX-LAPTOP",
            "Фейковый EC + питание USB для ноутбуков — обязательна",
            "Обязательна",
        ));
    } else {
        out.push(ssdt(
            "SSDT-EC-USBX-DESKTOP",
            "Фейковый EC + питание USB для десктопов — обязательна",
            "Обязательна",
        ));
    }

    // SSDT-AWAC — RTC AWAC на новых платформах
    if is_intel {
        if intel_has_awac(&codename) {
            out.push(ssdt(
                "SSDT-AWAC-DRTNIA",
                "Возврат классического RTC на чипсетах 300-й серии и новее — обязательна",
                "Обязательна",
            ));
        }
    } else {
        out.push(ssdt(
            "SSDT-AWAC-DRTNIA",
            "RTC-фикс: нужна для чипсетов AMD B550/A520/X570",
            "Рекомендуется",
        ));
    }

    // SSDT-PMC — восстановление NVRAM на 300/400-сериях
    if is_intel && intel_needs_pmc(&codename) {
        out.push(ssdt(
            "SSDT-PMC",
            "Восстановление native NVRAM (чипсеты Z390/B360 и др.); не нужна для B365/H310/C246",
            "Обязательна",
        ));
    }

    // SSDT-XOSI — ноутбуки, I2C-тачпады (VoodooI2C)
    if is_laptop {
        out.push(ssdt(
            "SSDT-XOSI",
            "Спуф O SMBus/OSI для I2C-тачпадов на ноутбуках (с VoodooI2C)",
            "Рекомендуется",
        ));
    }

    // SSDT-UNC — X99
    if is_intel && intel_is_hedt_x99(&codename) {
        out.push(ssdt(
            "SSDT-UNC",
            "Фиксы моста UNC на X99 (Haswell-E/Broadwell-E)",
            "Обязательна",
        ));
    }

    // SSDT-DMAR — если пользователь включил VT-d и не отключает в BIOS
    out.push(ssdt(
        "SSDT-DMAR",
        "Отключение DMAR/VT-d без захода в BIOS — только если VT-d включён",
        "Опциональна",
    ));

    let _ = GUIDE_URL;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::{CpuInfo, HardwareReport};

    fn report(manufacturer: &str, codename: Option<&str>, platform: &str) -> HardwareReport {
        HardwareReport {
            os: "windows".into(),
            platform: platform.into(),
            board_name: "Test Board".into(),
            cpu: CpuInfo {
                manufacturer: manufacturer.into(),
                name: "Test CPU".into(),
                codename: codename.map(str::to_string),
                cores: 6,
                threads: 12,
                avx2: true,
            },
            memory_gb: 16.0,
            gpus: vec![],
            network: vec![],
            audio: vec![],
            bluetooth: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn test_coffee_lake_desktop() {
        let r = report("Intel", Some("Coffee Lake-S"), "Desktop");
        let ssdts = recommend(&r);
        let names: Vec<&str> = ssdts.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"SSDT-PLUG-DRTNIA"));
        assert!(names.contains(&"SSDT-EC-USBX-DESKTOP"));
        assert!(names.contains(&"SSDT-AWAC-DRTNIA"));
        assert!(names.contains(&"SSDT-PMC"));
        assert!(!names.contains(&"SSDT-XOSI"), "XOSI только для ноутбуков");
    }

    #[test]
    fn test_haswell_desktop() {
        let r = report("Intel", Some("Haswell"), "Desktop");
        let ssdts = recommend(&r);
        let names: Vec<&str> = ssdts.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"SSDT-PLUG-DRTNIA"));
        assert!(!names.contains(&"SSDT-AWAC-DRTNIA"), "Haswell не имеет AWAC");
        assert!(!names.contains(&"SSDT-PMC"));
    }

    #[test]
    fn test_amd_desktop() {
        let r = report("AMD", Some("Vermeer"), "Desktop");
        let ssdts = recommend(&r);
        let names: Vec<&str> = ssdts.iter().map(|s| s.name.as_str()).collect();
        assert!(!names.contains(&"SSDT-PLUG-DRTNIA"), "AMD не нуждается в PLUG");
        assert!(names.contains(&"SSDT-EC-USBX-DESKTOP"));
        assert!(names.contains(&"SSDT-AWAC-DRTNIA"));
    }

    #[test]
    fn test_intel_laptop() {
        let r = report("Intel", Some("Whiskey Lake-U"), "Laptop");
        let ssdts = recommend(&r);
        let names: Vec<&str> = ssdts.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"SSDT-EC-USBX-LAPTOP"));
        assert!(names.contains(&"SSDT-XOSI"));
        assert!(!names.contains(&"SSDT-EC-USBX-DESKTOP"));
    }

    #[test]
    fn test_files_are_aml() {
        let r = report("Intel", Some("Coffee Lake-S"), "Desktop");
        for s in recommend(&r) {
            assert!(s.file.ends_with(".aml"));
            assert!(s.file.starts_with("SSDT-"));
            assert_eq!(s.selected, s.kind != "Опциональна", "{}: selected соответствует kind", s.name);
        }
    }
}
