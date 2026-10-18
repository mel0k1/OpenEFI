use serde::{Deserialize, Serialize};

use crate::cpu_codename;
use crate::db::device_ids;
use crate::hardware::{HardwareReport, in_list};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KextEntry {
    pub name: String,
    pub description: String,
    pub category: String,
    pub required: bool,
    pub selected: bool,
    pub status: String,
    pub requires: Vec<String>,
    pub repo: Option<String>,
}

struct KextDef {
    name: &'static str,
    description: &'static str,
    category: &'static str,
    required: bool,
    min_darwin: u32,
    max_darwin: u32,
    requires: &'static [&'static str],
    repo: Option<&'static str>,
}

const fn k(
    name: &'static str,
    description: &'static str,
    category: &'static str,
    required: bool,
    min_darwin: u32,
    max_darwin: u32,
    requires: &'static [&'static str],
    repo: Option<&'static str>,
) -> KextDef {
    KextDef {
        name,
        description,
        category,
        required,
        min_darwin,
        max_darwin,
        requires,
        repo,
    }
}

const LATEST: u32 = 9999;

static KEXTS: &[KextDef] = &[
    k("Lilu", "Патч-движок ядра: базовый слой для остальных плагинов", "Обязательные", true, 0, LATEST, &[], Some("acidanthera/Lilu")),
    k("VirtualSMC", "Эмулятор SMC — без него macOS не запустится", "Обязательные", true, 0, LATEST, &["Lilu"], Some("acidanthera/VirtualSMC")),
    k("SMCProcessor", "Датчики температуры CPU", "Плагины VirtualSMC", false, 0, LATEST, &["Lilu", "VirtualSMC"], Some("acidanthera/VirtualSMC")),
    k("SMCSuperIO", "Мониторинг датчиков и управление оборотами вентиляторов", "Плагины VirtualSMC", false, 0, LATEST, &["Lilu", "VirtualSMC"], Some("acidanthera/VirtualSMC")),
    k("SMCBatteryManager", "Статус и уровень заряда батареи", "Плагины VirtualSMC", false, 0, LATEST, &["Lilu", "VirtualSMC"], Some("acidanthera/VirtualSMC")),
    k("SMCLightSensor", "Датчик внешнего освещения", "Плагины VirtualSMC", false, 0, LATEST, &["Lilu", "VirtualSMC"], Some("acidanthera/VirtualSMC")),
    k("SMCDellSensors", "Датчики вентиляторов на ноутбуках Dell", "Плагины VirtualSMC", false, 0, LATEST, &["Lilu", "VirtualSMC"], Some("acidanthera/VirtualSMC")),
    k("SMCRadeonSensors", "Датчики температуры GPU AMD", "Плагины VirtualSMC", false, 18, LATEST, &["Lilu", "VirtualSMC"], Some("ChefKissInc/SMCRadeonSensors")),
    k("WhateverGreen", "Фиксы видеокарт Intel/NVIDIA/AMD", "Графика", false, 0, LATEST, &["Lilu"], Some("acidanthera/WhateverGreen")),
    k("NootedRed", "Поддержка iGPU AMD (Vega APUs)", "Графика", false, 17, LATEST, &["Lilu"], Some("ChefKissInc/NootedRed")),
    k("NootRX", "Поддержка dGPU AMD RDNA 2 (RX 6000)", "Графика", false, 20, 24, &["Lilu"], Some("ChefKissInc/NootRX")),
    k("AppleALC", "Нативный звук на unsupported кодеках", "Аудио", false, 0, LATEST, &["Lilu"], Some("acidanthera/AppleALC")),
    k("AirportBrcmFixup", "Фиксы для не-нативных Broadcom Wi-Fi карт", "Wi-Fi", false, 0, LATEST, &[], Some("acidanthera/AirportBrcmFixup")),
    k("IOSkywalkFamily", "Наследуемый стек Wi-Fi для Sonoma/Tahoe", "Wi-Fi", false, 23, LATEST, &[], Some("laobamac/IOSkywalkFamily")),
    k("AirPortBrcmNIC-Tahoe", "Broadcom Wi-Fi на macOS Tahoe", "Wi-Fi", false, 25, LATEST, &["IOSkywalkFamily"], Some("laobamac")),
    k("AirportItlwm", "Intel Wi-Fi с нативным интерфейсом macOS", "Wi-Fi", false, 18, 23, &[], Some("OpenIntelWireless/itlwm")),
    k("itlwm", "Intel Wi-Fi через приложение HeliPort", "Wi-Fi", false, 18, LATEST, &[], Some("OpenIntelWireless/itlwm")),
    k("IntelBluetoothFirmware", "Прошивка для Intel Bluetooth", "Bluetooth", false, 17, LATEST, &[], Some("OpenIntelWireless/IntelBluetoothFirmware")),
    k("IntelBTPatcher", "Фиксы багов Intel Bluetooth", "Bluetooth", false, 17, LATEST, &["IntelBluetoothFirmware"], Some("OpenIntelWireless/IntelBluetoothFirmware")),
    k("BlueToolFixup", "Поддержка сторонних Bluetooth на Monterey+", "Bluetooth", false, 21, LATEST, &[], Some("acidanthera/BrcmPatchRAM")),
    k("BrcmFirmwareData", "PatchRAM прошивки для Broadcom Bluetooth", "Bluetooth", false, 0, LATEST, &[], Some("acidanthera/BrcmPatchRAM")),
    k("BrcmPatchRAM2", "PatchRAM обновления для Broadcom USB Bluetooth", "Bluetooth", false, 0, LATEST, &[], Some("acidanthera/BrcmPatchRAM")),
    k("BrcmPatchRAM3", "PatchRAM обновления для Broadcom USB Bluetooth (10.15+)", "Bluetooth", false, 19, LATEST, &["BlueToolFixup"], Some("acidanthera/BrcmPatchRAM")),
    k("BrcmBluetoothInjector", "Переключатель Broadcom Bluetooth на старых версиях", "Bluetooth", false, 0, 21, &[], Some("acidanthera/BrcmPatchRAM")),
    k("RealtekBluetoothFirmware", "Прошивка для Realtek Bluetooth", "Bluetooth", false, 18, LATEST, &[], Some("karylaze/RealtekBluetoothFirmware")),
    k("IntelMausiEthernet", "Драйвер Intel Ethernet (Mausi)", "Ethernet", false, 0, LATEST, &[], Some("Mieze/IntelMausiEthernet")),
    k("AppleIGC", "Драйвер Intel 2.5G Ethernet (i225/i226)", "Ethernet", false, 19, LATEST, &[], Some("SongXiaoXi/AppleIGC")),
    k("AppleIGB", "Драйвер Intel IGB серверных контроллеров", "Ethernet", false, 0, LATEST, &[], Some("SongXiaoXi/AppleIGB")),
    k("AtherosE2200Ethernet", "Драйвер Atheros E2200", "Ethernet", false, 0, LATEST, &[], Some("Mieze/AtherosE2200Ethernet")),
    k("RealtekRTL8111", "Драйвер Realtek RTL8111/8168", "Ethernet", false, 0, LATEST, &[], Some("Mieze/RTL8111_driver_for_OS_X")),
    k("RealtekRTL8100", "Драйвер Realtek RTL8100", "Ethernet", false, 0, LATEST, &[], Some("Mieze/RealtekRTL8100")),
    k("RealtekRTL8125", "Драйвер Realtek 2.5G RTL8125", "Ethernet", false, 17, LATEST, &[], Some("Mieze/RealtekRTL8125")),
    k("CatalinaBCM5701Ethernet", "Драйвер Broadcom BCM57XX Ethernet", "Ethernet", false, 0, LATEST, &[], None),
    k("RestrictEvents", "Блокировка лишних процессов, разблокировка функций (cpuname, memtab)", "Дополнительно", false, 0, LATEST, &["Lilu"], Some("acidanthera/RestrictEvents")),
    k("AppleMCEReporterDisabler", "Отключение AppleMCEReporter — против паники на AMD и многосокетных системах", "Дополнительно", false, 19, LATEST, &[], None),
    k("CpuTopologyRebuild", "Перестройка топологии ядер Intel (E/P-core) для 6+ ядер", "Дополнительно", false, 18, 23, &["Lilu"], Some("b00t0x/CpuTopologyRebuild")),
    k("CpuTscSync", "Синхронизация TSC на Intel HEDT", "Дополнительно", false, 17, LATEST, &["Lilu"], Some("acidanthera/CpuTscSync")),
    k("AmdTscSync", "Синхронизация TSC на AMD", "Дополнительно", false, 0, LATEST, &[], Some("lvs1974/AmdTscSync")),
    k("ForgedInvariant", "Plug & play синхронизация TSC на AMD и Intel", "Дополнительно", false, 17, LATEST, &["Lilu"], Some("ChefKissInc/ForgedInvariant")),
    k("CryptexFixup", "Патчи для установки Rosetta на системах без AVX2", "Дополнительно", false, 22, LATEST, &["Lilu"], Some("khronokernel/CryptexFixup")),
    k("NVMeFix", "Фиксы совместимости и энергопотребления NVMe SSD", "Дополнительно", false, 19, LATEST, &["Lilu"], Some("acidanthera/NVMeFix")),
    k("BrightnessKeys", "Обработка клавиш яркости без DSDT-патчей", "Дополнительно", false, 17, LATEST, &[], Some("acidanthera/BrightnessKeys")),
    k("CPUFriend", "Динамическое управление питанием CPU (нужен CPUFriendDataProvider)", "Дополнительно", false, 17, LATEST, &["Lilu"], Some("acidanthera/CPUFriend")),
    k("FeatureUnlock", "Разблокировка Sidecar, Continuity и прочего на unsupported железе", "Дополнительно", false, 19, LATEST, &["Lilu"], Some("acidanthera/FeatureUnlock")),
    k("NoTouchID", "Убирает лаги аутентификации на системах с Touch ID board-ids", "Дополнительно", false, 0, LATEST, &["Lilu"], Some("al3xtjames/NoTouchID")),
    k("ECEnabler", "Чтение полей Embedded Controller длиннее 1 байта (батарея)", "Дополнительно", false, 19, LATEST, &[], Some("1Revenger1/ECEnabler")),
    k("HibernationFixup", "Фиксы гибернации и спящего режима", "Дополнительно", false, 17, LATEST, &["Lilu"], Some("acidanthera/HibernationFixup")),
    k("USBToolBox", "Инструмент маппинга USB-портов", "USB", false, 17, LATEST, &["Lilu"], Some("USBToolBox/kext")),
    k("UTBDefault", "Временный кекст: все USB-порты без лимита (заменить своим UTBMap)", "USB", false, 17, LATEST, &["USBToolBox"], Some("USBToolBox/kext")),
    k("XHCI-unsupported", "Поддержка нестандартных xHCI-контроллеров", "USB", false, 0, LATEST, &[], None),
    k("GenericUSBXHCI", "Фиксы USB 3.0 на некоторых AMD APU", "USB", false, 0, LATEST, &[], Some("RehabMan/OS-X-Generic-USBXHCI")),
    k("SATA-unsupported", "Поддержка редких SATA-контроллеров", "Хранилище", false, 0, LATEST, &[], None),
    k("CtlnaAHCIPort", "Улучшенная поддержка отдельных SATA-контроллеров", "Хранилище", false, 0, LATEST, &[], None),
    k("RealtekCardReader", "Драйвер ридеров карт Realtek (PCIe/USB)", "Карт-ридеры", false, 17, LATEST, &[], Some("0xFireWolf/RealtekCardReader")),
    k("RealtekCardReaderFriend", "Отображение ридера Realtek в Системной информации", "Карт-ридеры", false, 18, LATEST, &["RealtekCardReader"], Some("0xFireWolf/RealtekCardReaderFriend")),
    k("VoodooPS2Controller", "Клавиатуры, тачпады и мыши PS/2", "Ввод", false, 0, LATEST, &["Lilu"], Some("acidanthera/VoodooPS2")),
    k("VoodooI2C", "Поддержка I2C-контроллеров и тачпадов", "Ввод", false, 17, LATEST, &[], Some("VoodooI2C/VoodooI2C")),
    k("VoodooI2CHID", "Сателлит для HID I2C / ELAN1200+ устройств", "Ввод", false, 17, LATEST, &["VoodooI2C"], Some("VoodooI2C/VoodooI2C")),
    k("VoodooRMI", "Тачпады Synaptics по SMBus/I2C", "Ввод", false, 18, LATEST, &["VoodooInput"], Some("VoodooRMI/VoodooRMI")),
    k("AsusSMC", "ALS, подсветка клавиатуры и Fn-клавиши на ноутбуках ASUS", "Брендовые", false, 0, LATEST, &["Lilu", "VirtualSMC"], Some("hieplpvip/AsusSMC")),
    k("YogaSMC", "Сенсоры и фирменные функции Lenovo IdeaPad/Yoga/ThinkPad", "Брендовые", false, 0, LATEST, &["Lilu", "VirtualSMC"], Some("zhen-zen/YogaSMC")),
    k("BigSurface", "Комплексная поддержка устройств Microsoft Surface", "Брендовые", false, 20, LATEST, &["Lilu"], Some("VoodooI2C/BigSurface")),
];

/// Границы поддержки macOS (darwin) для кекта по имени
pub fn kext_bounds(name: &str) -> (Option<u32>, Option<u32>) {
    match find(name) {
        Some(def) => (
            Some(def.min_darwin).filter(|v| *v > 0),
            Some(def.max_darwin).filter(|v| *v < LATEST),
        ),
        None => (None, None),
    }
}

pub fn kext_catalog() -> Vec<KextEntry> {
    KEXTS.iter().map(|k| KextEntry {
        name: k.name.to_string(),
        description: k.description.to_string(),
        category: k.category.to_string(),
        required: k.required,
        selected: k.required,
        status: "available".into(),
        requires: k.requires.iter().map(|s| s.to_string()).collect(),
        repo: k.repo.map(str::to_string),
    }).collect()
}

fn find(name: &str) -> Option<&'static KextDef> {
    KEXTS.iter().find(|k| k.name == name)
}

pub fn match_kexts(report: &HardwareReport, macos_darwin: u32) -> Vec<KextEntry> {
    let ids = device_ids();
    let mut selected: Vec<String> = vec![
        "Lilu".into(),
        "VirtualSMC".into(),
        "USBToolBox".into(),
        "UTBDefault".into(),
    ];

    let is_intel = report.cpu.manufacturer == "Intel";
    let is_laptop = report.platform.contains("Laptop");
    let board_upper = report.board_name.to_uppercase();

    if is_intel {
        selected.push("SMCProcessor".into());
        selected.push("SMCSuperIO".into());
    } else {
        selected.push("SMCRadeonSensors".into());
        selected.push("AmdTscSync".into());
    }

    if is_laptop && !board_upper.contains("SURFACE") {
        selected.push("SMCBatteryManager".into());
        selected.push("SMCLightSensor".into());
        selected.push("BrightnessKeys".into());
        if board_upper.contains("DELL") {
            selected.push("SMCDellSensors".into());
        }
        if board_upper.contains("ASUS") {
            selected.push("AsusSMC".into());
        }
    } else if is_laptop {
        selected.push("BigSurface".into());
    }

    if !report.cpu.avx2 && macos_darwin >= 22 {
        selected.push("CryptexFixup".into());
    }

    if !is_intel && macos_darwin >= 21 || report.cpu.threads > 32 && macos_darwin >= 19 {
        selected.push("AppleMCEReporterDisabler".into());
    }

    if is_intel {
        let c = report.cpu.codename.as_deref().unwrap_or("");
        let idx = cpu_codename::INTEL_GENERATIONS.iter().position(|g| *g == c);
        if let Some(pos) = idx {
            if pos <= 19 && report.cpu.cores > 6 && !c.contains("Lunar") && !c.contains("Meteor") {
                selected.push("CpuTopologyRebuild".into());
            }
        }
        if is_intel_hedt(&report.cpu.name, c) {
            selected.push("CpuTscSync".into());
        }
    }

    let gpu_needs_oclp = report.gpus.iter().any(|g| {
        g.max_darwin.is_some() && g.max_darwin.unwrap() < macos_darwin
    });

    for gpu in &report.gpus {
        if gpu.device_type == "Integrated GPU" {
            if gpu.vendor_id == "1002" || gpu.vendor_id == "1022" {
                selected.push("NootedRed".into());
                if is_laptop {
                    selected.push("ForgedInvariant".into());
                }
            } else {
                selected.push("WhateverGreen".into());
            }
        } else {
            let n = gpu.name.to_lowercase();
            if n.contains("navi 22") || n.contains("6700") || n.contains("6800") {
                selected.push("NootRX".into());
            } else {
                selected.push("WhateverGreen".into());
            }
        }
    }

    let has_supported_codec = report.audio.iter().any(|a| {
        let full = format!("{}-{}", a.vendor_id.to_uppercase(), a.device_id.to_uppercase());
        ids.codecs.iter().any(|c| c.eq_ignore_ascii_case(&full))
    });
    if !report.audio.is_empty() {
        selected.push("AppleALC".into());
        if has_supported_codec && macos_darwin >= 25 {
            selected.push("AppleALC".into());
        }
    }

    for net in &report.network {
        let full = format!("{}-{}", net.vendor_id.to_uppercase(), net.device_id.to_uppercase());
        if in_list(&full, &ids.list("BroadcomWiFiIDs")) {
            selected.push("AirportBrcmFixup".into());
            if macos_darwin >= 23 {
                selected.push("IOSkywalkFamily".into());
            }
            if macos_darwin >= 25 {
                selected.push("AirPortBrcmNIC-Tahoe".into());
            }
        } else if in_list(&full, &ids.list("IntelWiFiIDs")) {
            if macos_darwin >= 23 {
                selected.push("itlwm".into());
            } else {
                selected.push("AirportItlwm".into());
            }
        } else if in_list(&full, &ids.list("AtherosWiFiIDs")) {
            selected.push("AMFIPass".into());
        } else if in_list(&full, &ids.list("rtw88WiFiIDs")) {
            selected.push("AMFIPass".into());
        } else if in_list(&full, &ids.list("IntelI22XIDs")) {
            selected.push("AppleIGC".into());
        } else if in_list(&full, &ids.list("AtherosE2200IDs")) {
            selected.push("AtherosE2200Ethernet".into());
        } else if in_list(&full, &ids.list("IntelMausiIDs")) {
            selected.push("IntelMausiEthernet".into());
        } else if in_list(&full, &ids.list("RealtekRTL8100IDs")) {
            selected.push("RealtekRTL8100".into());
        } else if in_list(&full, &ids.list("RealtekRTL8111IDs")) {
            selected.push("RealtekRTL8111".into());
        } else if in_list(&full, &ids.list("RealtekRTL8125IDs"))
            || in_list(&full, &ids.list("RealtekRTL812xIDs"))
        {
            selected.push("RealtekRTL8125".into());
        } else if in_list(&full, &ids.list("AppleIGBIDs"))
            || in_list(&full, &ids.list("IntelX500IDs"))
        {
            selected.push("AppleIGB".into());
        } else if in_list(&full, &ids.list("BroadcomBCM57XXIDs")) {
            selected.push("CatalinaBCM5701Ethernet".into());
        }
    }

    for bt in &report.bluetooth {
        let full = format!("{}-{}", bt.vendor_id.to_uppercase(), bt.device_id.to_uppercase());
        if in_list(&full, &ids.list("IntelBluetoothIDs")) {
            selected.push("IntelBluetoothFirmware".into());
            selected.push("IntelBTPatcher".into());
            selected.push("BlueToolFixup".into());
        } else if in_list(&full, &ids.list("BroadcomBluetoothIDs")) {
            selected.push("BrcmFirmwareData".into());
            selected.push("BrcmPatchRAM2".into());
            selected.push("BrcmPatchRAM3".into());
            if macos_darwin < 21 {
                selected.push("BrcmBluetoothInjector".into());
            }
            selected.push("BlueToolFixup".into());
        } else if in_list(&full, &ids.list("RealtekBluetoothIDs")) {
            selected.push("RealtekBluetoothFirmware".into());
            selected.push("BlueToolFixup".into());
        } else if in_list(&full, &ids.list("AtherosBluetoothIDs")) {
            selected.push("BlueToolFixup".into());
        }
    }

    if report.board_name.to_uppercase().contains("NUC") {
        // NUC-системы обычно имеют PCIe-ридер Realtek
    }

    let mut entries: Vec<KextEntry> = KEXTS
        .iter()
        .map(|k| KextEntry {
            name: k.name.to_string(),
            description: k.description.to_string(),
            category: k.category.to_string(),
            required: k.required,
            selected: k.required,
            status: String::new(),
            requires: k.requires.iter().map(|s| s.to_string()).collect(),
            repo: k.repo.map(str::to_string),
        })
        .collect();

    for e in &mut entries {
        let def = find(&e.name).unwrap();
        let compatible = macos_darwin >= def.min_darwin && macos_darwin <= def.max_darwin;
        if selected.iter().any(|s| s == &e.name) {
            e.selected = true;
            e.status = if compatible {
                "recommended".into()
            } else {
                "needs_attention".into()
            };
        } else {
            e.selected = false;
            e.status = if compatible { "available".into() } else { "incompatible".into() };
        }
    }

    // вытягиваем зависимости выбранных кекстов
    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..entries.len() {
            if !entries[i].selected {
                continue;
            }
            for dep in entries[i].requires.clone() {
                if let Some(pos) = entries.iter().position(|t| t.name == dep) {
                    if !entries[pos].selected {
                        entries[pos].selected = true;
                        entries[pos].status = "dependency".into();
                        changed = true;
                    }
                }
            }
        }
    }

    let _ = gpu_needs_oclp;
    entries
}

fn is_intel_hedt(processor_name: &str, codename: &str) -> bool {
    const IDX_START: usize = 66;
    let pos = cpu_codename::INTEL_GENERATIONS.iter().position(|g| *g == codename);
    match pos {
        Some(p) if p < 45 => false,
        Some(p) if p < IDX_START => {
            codename.ends_with("-X") || codename.ends_with("-P") || codename.ends_with("-W")
                || codename.ends_with("-E") || codename.ends_with("-EP") || codename.ends_with("-EX")
        }
        Some(_) => processor_name.to_uppercase().contains("XEON"),
        None => false,
    }
}
