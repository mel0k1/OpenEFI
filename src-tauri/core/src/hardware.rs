use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use sysinfo::System;

use crate::cpu_codename;
use crate::db::PciDatabase;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CpuInfo {
    pub manufacturer: String,
    pub name: String,
    pub codename: Option<String>,
    pub cores: usize,
    pub threads: usize,
    pub avx2: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviceInfo {
    pub name: String,
    pub vendor_id: String,
    pub device_id: String,
    pub kind: String,
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GpuInfo {
    pub name: String,
    pub vendor_id: String,
    pub device_id: String,
    pub device_type: String,
    pub codename: Option<String>,
    pub max_darwin: Option<u32>,
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HardwareReport {
    pub os: String,
    pub platform: String,
    pub board_name: String,
    pub cpu: CpuInfo,
    pub memory_gb: f64,
    pub gpus: Vec<GpuInfo>,
    pub network: Vec<DeviceInfo>,
    pub audio: Vec<DeviceInfo>,
    pub bluetooth: Vec<DeviceInfo>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PciDevice {
    pub vendor: String,
    pub device: String,
    pub class: u32,
    pub name_hint: Option<String>,
    /// PnP-класс устройства (только Windows: "Display", "Net", "Media"...)
    pub pnp_class: Option<String>,
}

pub fn scan() -> HardwareReport {
    let mut warnings = Vec::new();
    let os = std::env::consts::OS.to_string();

    let mut sys = System::new();
    sys.refresh_cpu_all();
    sys.refresh_memory();
    std::thread::sleep(std::time::Duration::from_millis(300));
    sys.refresh_cpu_all();

    let cpu_name = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_default();
    let physical_cores = System::physical_core_count().unwrap_or(sys.cpus().len());
    let threads = sys.cpus().len();

    let (manufacturer, name): (String, String) = if cpu_name.to_uppercase().contains("RYZEN")
        || cpu_name.to_uppercase().contains("EPYC")
        || cpu_name.to_uppercase().contains("ATHLON")
        || cpu_name.to_uppercase().contains("THREADRIPPER")
    {
        ("AMD".to_string(), cpu_name.clone())
    } else {
        ("Intel".to_string(), cpu_name.clone())
    };

    let codename = if manufacturer == "AMD" {
        cpu_codename::amd_codename(&cpu_name).map(str::to_string)
    } else {
        cpu_codename::intel_codename(&cpu_name).map(str::to_string)
    };

    let pci_devices = match scan_pci(&os) {
        Ok(d) => d,
        Err(e) => {
            warnings.push(format!("Не удалось получить список PCI-устройств: {e}"));
            Vec::new()
        }
    };

    let usb_devices = scan_usb(&os).unwrap_or_default();
    let board = board_name(&os).unwrap_or_else(|| "Unknown".into());
    let platform = detect_platform(&os, &board);

    let gpus = classify_gpus(&pci_devices, &codename, &manufacturer);
    let network = classify_network(&pci_devices, &usb_devices);
    let audio = classify_audio(&pci_devices, &os);
    let bluetooth = classify_bluetooth(&usb_devices);

    let memory_bytes = sys.total_memory();
    let memory_gb = (memory_bytes as f64 / 1024.0 / 1024.0 / 1024.0 * 10.0).round() / 10.0;

    HardwareReport {
        os,
        platform,
        board_name: board,
        cpu: CpuInfo {
            manufacturer,
            name,
            codename,
            cores: physical_cores,
            threads,
            avx2: cpu_codename::has_avx2(&cpu_name),
        },
        memory_gb,
        gpus,
        network,
        audio,
        bluetooth,
        warnings,
    }
}

fn read_trim(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn scan_pci(os: &str) -> Result<Vec<PciDevice>, String> {
    match os {
        "linux" => scan_pci_linux(),
        "windows" => scan_pci_windows(),
        "macos" => scan_pci_macos(),
        _ => Err("неизвестная ОС".into()),
    }
}

fn scan_pci_linux() -> Result<Vec<PciDevice>, String> {
    let mut devices = Vec::new();
    let dir = Path::new("/sys/bus/pci/devices");
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        let vendor = match read_trim(&path.join("vendor")) {
            Some(v) => v.trim_start_matches("0x").to_string(),
            None => continue,
        };
        let device = read_trim(&path.join("device"))
            .unwrap_or_default()
            .trim_start_matches("0x")
            .to_string();
        let class = read_trim(&path.join("class"))
            .and_then(|c| u32::from_str_radix(c.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0);
        let name_hint = read_trim(&path.join("uevent"))
            .and_then(|u| {
                u.lines().find(|l| l.starts_with("PCI_ID")).map(|l| l.to_string())
            });
        devices.push(PciDevice {
            pnp_class: None,
            vendor,
            device,
            class,
            name_hint,
        });
    }
    Ok(devices)
}

fn scan_pci_windows() -> Result<Vec<PciDevice>, String> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_PnPEntity | Where-Object { $_.DeviceID -like 'PCI\\VEN_*' } | ForEach-Object { [string]$_.PNPClass + '|' + [string]$_.Name + '|' + $_.DeviceID }",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some((pnp_class, name, vendor, device)) = parse_windows_pnp_line(line) {
            devices.push(PciDevice {
                vendor,
                device,
                class: 0,
                name_hint: name,
                pnp_class: pnp_class.map(|c| c.to_lowercase()),
            });
        }
    }
    Ok(devices)
}

/// Разбор строки формата "PNPClass|Name|DeviceID", полученной из Win32_PnPEntity.
/// PNPClass может быть пустым; DeviceID обязан содержать VEN_/DEV_.
fn parse_windows_pnp_line(line: &str) -> Option<(Option<String>, Option<String>, String, String)> {
    if line.is_empty() {
        return None;
    }
    let mut parts = line.splitn(3, '|');
    let pnp_class = parts.next().unwrap_or("").trim();
    let name = parts.next().unwrap_or("").trim();
    let device_id = parts.next().unwrap_or("").trim().to_lowercase();
    let vendor = extract_hex_field(&device_id, "ven_")?;
    let device = extract_hex_field(&device_id, "dev_")?;
    if vendor.is_empty() || device.is_empty() {
        return None;
    }
    let class = if pnp_class.is_empty() { None } else { Some(pnp_class.to_string()) };
    let hint = if name.is_empty() || name == device_id { None } else { Some(name.to_string()) };
    Some((class, hint, vendor, device))
}

fn scan_pci_macos() -> Result<Vec<PciDevice>, String> {
    let output = Command::new("system_profiler")
        .args(["SPPCIDataType", "-json"])
        .output()
        .map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let mut devices = Vec::new();
    if let Some(items) = value
        .get("SPPCIDataType")
        .and_then(|v| v.as_array())
    {
        for item in items {
            let vendor = item
                .get("spdisplays_vendor-id")
                .or_else(|| item.get("spdisplays_vendor_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim_start_matches("0x")
                .to_lowercase();
            let device = item
                .get("spdisplays_device-id")
                .or_else(|| item.get("spdisplays_device_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim_start_matches("0x")
                .to_lowercase();
            if vendor.is_empty() {
                continue;
            }
            devices.push(PciDevice {
            pnp_class: None,
                vendor,
                device,
                class: 0,
                name_hint: item.get("_name").and_then(|n| n.as_str()).map(str::to_string),
            });
        }
    }
    Ok(devices)
}

fn scan_usb(os: &str) -> Result<Vec<PciDevice>, String> {
    match os {
        "linux" => {
            let mut devices = Vec::new();
            let dir = Path::new("/sys/bus/usb/devices");
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.contains(':') {
                        continue;
                    }
                    let path = entry.path();
                    let vendor = match read_trim(&path.join("idVendor")) {
                        Some(v) => v.to_lowercase(),
                        None => continue,
                    };
                    let device = read_trim(&path.join("idProduct"))
                        .unwrap_or_default()
                        .to_lowercase();
                    devices.push(PciDevice {
            pnp_class: None,
                        vendor,
                        device,
                        class: 0,
                        name_hint: read_trim(&path.join("product")),
                    });
                }
            }
            Ok(devices)
        }
        "windows" => {
            let output = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Get-CimInstance Win32_PnPEntity | Where-Object { $_.DeviceID -like 'USB\\*' } | Select-Object -ExpandProperty DeviceID",
                ])
                .output()
                .map_err(|e| e.to_string())?;
            let text = String::from_utf8_lossy(&output.stdout);
            let mut devices = Vec::new();
            for line in text.lines() {
                let lower = line.trim().to_lowercase();
                let vendor = extract_hex_field(&lower, "vid_").unwrap_or_default();
                let device = extract_hex_field(&lower, "pid_").unwrap_or_default();
                if vendor.is_empty() {
                    continue;
                }
                devices.push(PciDevice {
            pnp_class: None,
                    vendor,
                    device,
                    class: 0,
                    name_hint: Some(line.trim().to_string()),
                });
            }
            Ok(devices)
        }
        "macos" => {
            let output = Command::new("system_profiler")
                .args(["SPUSBDataType", "-json"])
                .output()
                .map_err(|e| e.to_string())?;
            let value: serde_json::Value =
                serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
            let mut devices = Vec::new();
            fn walk(node: &serde_json::Value, devices: &mut Vec<PciDevice>) {
                if let Some(items) = node.as_array() {
                    for item in items {
                        let vendor = item
                            .get("vendor_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim_start_matches("0x")
                            .to_lowercase();
                        let device = item
                            .get("product_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim_start_matches("0x")
                            .to_lowercase();
                        if !vendor.is_empty() {
                            devices.push(PciDevice {
            pnp_class: None,
                                vendor,
                                device,
                                class: 0,
                                name_hint: item.get("_name").and_then(|n| n.as_str()).map(str::to_string),
                            });
                        }
                        if let Some(children) = item.get("_items") {
                            walk(children, devices);
                        }
                    }
                }
            }
            if let Some(root) = value.get("SPUSBDataType") {
                walk(root, &mut devices);
            }
            Ok(devices)
        }
        _ => Ok(Vec::new()),
    }
}

fn extract_hex_field(text: &str, key: &str) -> Option<String> {
    let pos = text.find(key)?;
    let rest = &text[pos + key.len()..];
    let hex: String = rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
    if hex.is_empty() {
        None
    } else {
        Some(hex)
    }
}

fn board_name(os: &str) -> Option<String> {
    match os {
        "linux" => read_trim(Path::new("/sys/devices/virtual/dmi/id/board_name")),
        "windows" => {
            let output = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "(Get-CimInstance Win32_BaseBoard).Manufacturer + ' ' + (Get-CimInstance Win32_BaseBoard).Product",
                ])
                .output()
                .ok()?;
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        "macos" => {
            let output = Command::new("sysctl")
                .args(["-n", "hw.model"])
                .output()
                .ok()?;
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        _ => None,
    }
}

fn detect_platform(os: &str, board: &str) -> String {
    let upper = board.to_uppercase();
    match os {
        "linux" => {
            if let Some(ct) = read_trim(Path::new("/sys/class/dmi/id/chassis_type")) {
                let t: u32 = ct.parse().unwrap_or(0);
                if matches!(t, 8 | 9 | 10 | 14 | 30 | 31 | 32) {
                    return "Laptop".into();
                }
                if t != 0 {
                    return "Desktop".into();
                }
            }
            "Desktop".into()
        }
        "windows" => {
            let output = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "(Get-CimInstance Win32_ComputerSystem).PCSystemType",
                ])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_default();
            match output.as_str() {
                "2" => "Laptop".into(),
                _ => "Desktop".into(),
            }
        }
        "macos" => {
            if upper.starts_with("MACBOOK") {
                "Laptop".into()
            } else {
                "Desktop".into()
            }
        }
        _ => "Desktop".into(),
    }
}

fn is_amd_apu_name(name: &str) -> bool {
    const APU_MARKERS: &[&str] = &[
        "Raven", "Picasso", "Renoir", "Lucienne", "Cezanne", "Barcel", "Rembrandt",
        "Mendocino", "Phoenix", "Hawk Point", "Strix", "Vega", "Radeon 610M",
        "Radeon 680M", "Radeon 780M", "Radeon 880M", "Radeon 890M", "Graphics",
        "Apu",
    ];
    APU_MARKERS.iter().any(|m| name.to_lowercase().contains(&m.to_lowercase()))
}

fn classify_gpus(pci: &[PciDevice], cpu_codename: &Option<String>, cpu_manufacturer: &str) -> Vec<GpuInfo> {
    let db = PciDatabase::global();
    let mut gpus = Vec::new();
    for dev in pci {
        let class_major = dev.class >> 16;
        let is_display_pci = class_major == 0x03;
        let is_display_pnp = dev.pnp_class.as_deref() == Some("display");
        let is_display_hint = dev
            .name_hint
            .as_ref()
            .map(|h| {
                let l = h.to_lowercase();
                l.contains("vga") || l.contains("display") || l.contains("graphics")
                    || l.contains("radeon") || l.contains("geforce")
            })
            .unwrap_or(false);
        if !is_display_pci && !is_display_pnp && !is_display_hint {
            continue;
        }
        let _vendor_name = db.vendor_name(&dev.vendor).unwrap_or("Unknown").to_string();
        let device_name = db
            .device_name(&dev.vendor, &dev.device)
            .unwrap_or(dev.name_hint.as_deref().unwrap_or("Unknown GPU"))
            .to_string();

        let (device_type, codename) = if dev.vendor == "8086" {
            ("Integrated GPU".to_string(), cpu_codename.clone())
        } else if dev.vendor == "1002" || dev.vendor == "1022" {
            let igpu = is_amd_apu_name(&device_name) || is_amd_apu_name(cpu_manufacturer);
            let t = if igpu { "Integrated GPU" } else { "Discrete GPU" };
            (t.to_string(), Some(device_name.clone()))
        } else if dev.vendor == "10de" {
            ("Discrete GPU".to_string(), Some(device_name.clone()))
        } else {
            ("Discrete GPU".to_string(), Some(device_name.clone()))
        };

        let (max_darwin, note) = gpu_macos_limits(&dev.vendor, &device_name, &device_type);
        gpus.push(GpuInfo {
            name: device_name,
            vendor_id: dev.vendor.clone(),
            device_id: dev.device.clone(),
            device_type,
            codename,
            max_darwin,
            note,
        });
    }
    gpus.sort_by_key(|g| g.device_type == "Integrated GPU");
    gpus
}

fn gpu_macos_limits(vendor: &str, name: &str, device_type: &str) -> (Option<u32>, Option<String>) {
    let n = name.to_lowercase();
    if vendor == "10de" {
        if n.contains("gk") {
            return (
                Some(19),
                Some("Kepler: нативно до Catalina 10.15; новее — только OCLP".into()),
            );
        }
        if n.contains("gm ") || n.contains("gm1") || n.contains("gm2") || n.contains("gm3")
            || n.contains("gp1") || n.contains("gp2") || n.contains("gp3") || n.contains("gp4")
            || n.contains("maxwell") || n.contains("pascal")
        {
            return (
                Some(21),
                Some("Maxwell/Pascal: нативно до Monterey 12; новее — только OCLP".into()),
            );
        }
        return (
            Some(0),
            Some("NVIDIA Turing/Ampere/Ada не поддерживаются macOS вовсе — понадобится AMD GPU".into()),
        );
    }
    if vendor == "1002" || vendor == "1022" {
        if n.contains("navi 3") || n.contains("navi3") {
            return (Some(23), Some("RDNA 3: нативно с Ventura 13".into()));
        }
        if n.contains("navi 2") || n.contains("navi2") {
            return (Some(24), Some("RDNA 2: нативно до Sequoia 15, нужен NootRX/WhateverGreen".into()));
        }
        if n.contains("polaris") || n.contains("lexa") || n.contains("ellesmere") || n.contains("baffin") {
            return (
                Some(22),
                Some("Polaris: нативно до Ventura 13; в Sonoma 14+ — только через OCLP".into()),
            );
        }
        return (None, None);
    }
    if vendor == "8086" && device_type == "Integrated GPU" {
        return (None, None);
    }
    (None, None)
}

pub const MAX_DARWIN_BY_INTEL_IGPU: &[(&str, u32)] = &[
    ("Sandy Bridge", 19), ("Sandy Bridge-E", 19), ("Ivy Bridge", 19), ("Ivy Bridge-E", 19),
    ("Haswell", 20), ("Haswell-H", 20), ("Haswell-ULT", 20), ("Haswell-ULX", 20),
    ("Haswell-E", 20), ("Haswell-EP", 20), ("Haswell-EX", 20),
    ("Broadwell", 20), ("Broadwell-H", 20), ("Broadwell-U", 20), ("Broadwell-Y", 20), ("Broadwell-E", 20),
    ("Skylake", 21), ("Skylake-X", 21), ("Kaby Lake", 21), ("Kaby Lake-H", 21),
    ("Kaby Lake-G", 21), ("Kaby Lake-X", 21), ("Amber Lake-Y", 21),
    ("Coffee Lake-S", 21), ("Coffee Lake-E", 21), ("Coffee Lake-H", 21), ("Coffee Lake-U", 21),
    ("Cannon Lake-U", 21), ("Whiskey Lake-U", 21), ("Comet Lake-S", 21), ("Comet Lake-H", 21),
    ("Comet Lake-U", 21), ("Comet Lake-W", 21), ("Ice Lake-U", 21), ("Ice Lake-SP", 21),
    ("Lakefield", 21),
    ("Tiger Lake-H", 24), ("Tiger Lake-B", 24), ("Tiger Lake-UP3", 24),
    ("Tiger Lake-H35", 24), ("Tiger Lake-UP4", 24),
    ("Rocket Lake-S", 22), ("Rocket Lake-E", 22),
    ("Alder Lake-S", 24), ("Alder Lake-HX", 24), ("Alder Lake-H", 24), ("Alder Lake-P", 24),
    ("Alder Lake-U", 24), ("Alder Lake-N", 24),
    ("Raptor Lake-S", 24), ("Raptor Lake-E", 24), ("Raptor Lake-HX", 24), ("Raptor Lake-H", 24),
    ("Raptor Lake-PX", 24), ("Raptor Lake-P", 24), ("Raptor Lake-U", 24),
    ("Meteor Lake-H", 24), ("Meteor Lake-U", 24),
    ("Arrow Lake-S", 25), ("Arrow Lake-H", 25), ("Arrow Lake-HX", 25), ("Arrow Lake-U", 25),
    ("Lunar Lake", 25),
];

fn classify_network(pci: &[PciDevice], usb: &[PciDevice]) -> Vec<DeviceInfo> {
    let db = PciDatabase::global();
    let ids = crate::db::device_ids();
    let mut out = Vec::new();

    for dev in pci {
        let class_major = dev.class >> 16;
        let class_sub = (dev.class >> 8) & 0xff;
        let is_eth = class_major == 0x02 && class_sub == 0x00;
        let is_wireless = class_major == 0x02 && (class_sub == 0x80 || class_sub == 0x20);
        let is_net_pnp = dev.pnp_class.as_deref() == Some("net");
        let hinted = dev
            .name_hint
            .as_ref()
            .map(|h| {
                let l = h.to_lowercase();
                l.contains("ethernet") || l.contains("network") || l.contains("wi-fi")
                    || l.contains("wireless") || l.contains("wifi") || l.contains("wlan")
                    || l.contains("802.11")
            })
            .unwrap_or(false);
        if !is_eth && !is_wireless && !is_net_pnp && !hinted {
            continue;
        }
        let full_id = format!("{}-{}", dev.vendor.to_uppercase(), dev.device.to_uppercase());
        let (kind, note) = if is_wireless || in_list(&full_id, &ids.list("IntelWiFiIDs"))
            || in_list(&full_id, &ids.list("BroadcomWiFiIDs"))
            || in_list(&full_id, &ids.list("AtherosWiFiIDs"))
            || in_list(&full_id, &ids.list("rtw88WiFiIDs"))
        {
            let note = if in_list(&full_id, &ids.list("AtherosWiFiIDs")) {
                Some("Atheros Wi-Fi работает только через OCLP (macOS до El Capitan нативно)".into())
            } else {
                None
            };
            ("Wi-Fi".to_string(), note)
        } else {
            ("Ethernet".to_string(), None)
        };
        let name = db
            .device_name(&dev.vendor, &dev.device)
            .unwrap_or(dev.name_hint.as_deref().unwrap_or("Сетевой адаптер"));
        out.push(DeviceInfo {
            name: name.to_string(),
            vendor_id: dev.vendor.clone(),
            device_id: dev.device.clone(),
            kind,
            note,
        });
    }

    for dev in usb {
        let full_id = format!("{}-{}", dev.vendor.to_uppercase(), dev.device.to_uppercase());
        if in_list(&full_id, &ids.list("WirelessUSBIDs")) {
            let name = dev.name_hint.clone().unwrap_or_else(|| "USB Wi-Fi адаптер".into());
            out.push(DeviceInfo {
                name,
                vendor_id: dev.vendor.clone(),
                device_id: dev.device.clone(),
                kind: "Wi-Fi (USB)".into(),
                note: Some("USB Wi-Fi адаптеры не поддерживаются macOS — нужен встроенный модуль".into()),
            });
        }
    }
    out
}

fn classify_audio(pci: &[PciDevice], os: &str) -> Vec<DeviceInfo> {
    let db = PciDatabase::global();
    let ids = crate::db::device_ids();
    let mut out = Vec::new();
    let mut codec_id: Option<String> = None;

    if os == "linux" {
        if let Ok(asound) = std::fs::read_dir("/proc/asound") {
            for entry in asound.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("card") {
                    continue;
                }
                if let Ok(codecs) = std::fs::read_dir(entry.path()) {
                    for codec in codecs.flatten() {
                        let cname = codec.file_name().to_string_lossy().to_string();
                        if !cname.starts_with("codec#") {
                            continue;
                        }
                        if let Ok(text) = std::fs::read_to_string(codec.path()) {
                            for line in text.lines() {
                                if let Some(rest) = line.strip_prefix("Vendor Id:") {
                                    let raw = rest.trim().trim_start_matches("0x");
                                    if let Ok(v) = u32::from_str_radix(raw, 16) {
                                        let hi = (v >> 16) as u16;
                                        let lo = v as u16;
                                        codec_id =
                                            Some(format!("{hi:04X}-{lo:04X}"));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for dev in pci {
        let class_major = dev.class >> 16;
        let class_sub = (dev.class >> 8) & 0xff;
        let is_audio = (class_major == 0x04 && class_sub == 0x03)
            || (class_major == 0x04 && class_sub == 0x01);
        let audio_pnp = matches!(dev.pnp_class.as_deref(), Some("media") | Some("audio"));
        let hinted = dev
            .name_hint
            .as_ref()
            .map(|h| h.to_lowercase().contains("audio"))
            .unwrap_or(false);
        if !is_audio && !audio_pnp && !hinted {
            continue;
        }
        let name = db
            .device_name(&dev.vendor, &dev.device)
            .unwrap_or("HDA-контроллер");
        let note = match &codec_id {
            Some(cid) if cid.starts_with(&dev.vendor.to_uppercase()) => {
                if ids.codecs.contains(cid) {
                    Some(format!("Кодек {cid} поддерживается AppleALC"))
                } else {
                    Some(format!("Кодек {cid}: поддержки в AppleALC нет, звук может не завестись"))
                }
            }
            _ => Some("Рекомендуется AppleALC; при проблемах задать alcid в boot-args".into()),
        };
        out.push(DeviceInfo {
            name: name.to_string(),
            vendor_id: dev.vendor.clone(),
            device_id: dev.device.clone(),
            kind: "Audio".into(),
            note,
        });
    }

    if out.is_empty() {
        if let Some(cid) = codec_id {
            out.push(DeviceInfo {
                name: format!("HDA кодек {cid}"),
                vendor_id: String::new(),
                device_id: String::new(),
                kind: "Audio".into(),
                note: Some("Определён через /proc/asound".into()),
            });
        }
    }
    out
}

fn classify_bluetooth(usb: &[PciDevice]) -> Vec<DeviceInfo> {
    let ids = crate::db::device_ids();
    let mut out = Vec::new();
    for dev in usb {
        let full_id = format!("{}-{}", dev.vendor.to_uppercase(), dev.device.to_uppercase());
        let bt_lists = [
            ids.list("IntelBluetoothIDs"),
            ids.list("BroadcomBluetoothIDs"),
            ids.list("AtherosBluetoothIDs"),
            ids.list("RealtekBluetoothIDs"),
        ];
        if bt_lists.iter().any(|l| in_list(&full_id, l)) {
            out.push(DeviceInfo {
                name: dev.name_hint.clone().unwrap_or_else(|| "Bluetooth".into()),
                vendor_id: dev.vendor.clone(),
                device_id: dev.device.clone(),
                kind: "Bluetooth".into(),
                note: None,
            });
        }
    }
    out
}

pub fn in_list(id: &str, list: &[String]) -> bool {
    list.iter().any(|x| x.eq_ignore_ascii_case(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_windows_pnp_line() {
        // Реальный формат Win32_PnPEntity: PNPClass|Name|DeviceID
        let (class, name, vendor, device) = parse_windows_pnp_line(
            "Display|Radeon RX 550 Series|PCI\\VEN_1002&DEV_699F&SUBSYS_C57017AA&REV_C1\\4&1B4B2C3&0&0008",
        )
        .unwrap();
        assert_eq!(class.as_deref(), Some("Display"));
        assert_eq!(name.as_deref(), Some("Radeon RX 550 Series"));
        assert_eq!(vendor, "1002");
        assert_eq!(device, "699f");

        // пустой PNPClass
        let (class, name, vendor, device) = parse_windows_pnp_line(
            "|Some Device|PCI\\VEN_10EC&DEV_8168&SUBSYS_85051043&REV_15\\4&1C9E2F3&0&00E5",
        )
        .unwrap();
        assert_eq!(class, None);
        assert_eq!(name.as_deref(), Some("Some Device"));
        assert_eq!(vendor, "10ec");
        assert_eq!(device, "8168");

        // без VEN_/DEV_ — отбрасывается
        assert!(parse_windows_pnp_line("Display|Screen|USB\\VID_1234&PID_5678").is_none());
        assert!(parse_windows_pnp_line("").is_none());
    }

    #[test]
    fn test_classify_gpus_windows_rx550() {
        // Симуляция Windows-скана: class=0, но pnp_class=Display
        let dev = PciDevice {
            vendor: "1002".into(),
            device: "699f".into(),
            class: 0,
            name_hint: Some("Radeon RX 550 Series".into()),
            pnp_class: Some("display".into()),
        };
        let gpus = classify_gpus(&[dev], &Some("Coffee Lake-S".into()), "GenuineIntel");
        assert_eq!(gpus.len(), 1, "RX 550 должна определяться как GPU");
        let gpu = &gpus[0];
        assert_eq!(gpu.device_type, "Discrete GPU");
        assert!(gpu.name.to_lowercase().contains("radeon"), "имя: {}", gpu.name);
        assert_eq!(gpu.max_darwin, Some(22), "Polaris нативно до Ventura (darwin 22)");
    }

    #[test]
    fn test_classify_network_and_audio_windows() {
        let net = PciDevice {
            vendor: "10ec".into(),
            device: "8168".into(),
            class: 0,
            name_hint: Some("Realtek Gaming GbE Family Controller".into()),
            pnp_class: Some("net".into()),
        };
        let audio = PciDevice {
            vendor: "8086".into(),
            device: "a0c8".into(),
            class: 0,
            name_hint: Some("Intel Smart Sound Technology".into()),
            pnp_class: Some("media".into()),
        };
        let gpu = PciDevice {
            vendor: "8086".into(),
            device: "9bc4".into(),
            class: 0,
            name_hint: Some("Intel(R) UHD Graphics 630".into()),
            pnp_class: Some("display".into()),
        };
        let pci = vec![net, audio, gpu];
        let gpus = classify_gpus(&pci, &Some("Comet Lake-S".into()), "GenuineIntel");
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].device_type, "Integrated GPU");

        let net_out = classify_network(&pci, &[]);
        assert_eq!(net_out.len(), 1, "сеть по pnp_class=Net должна находиться");
        assert_eq!(net_out[0].kind, "Ethernet");

        let audio_out = classify_audio(&pci, "windows");
        assert_eq!(audio_out.len(), 1, "аудио по pnp_class=Media должно находиться");
    }
}
