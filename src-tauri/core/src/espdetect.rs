//! Автопоиск EFI-раздела (ESP, GPT-тип c12a7328-f81f-11d2-ba4b-00a0c93ec93b)
//! вместо ручного выбора папки:
//! - Windows: Get-Partition по GptType + монтирование Add-PartitionAccessPath;
//! - Linux: lsblk по PARTTYPE, монтирование через udisksctl (без root на desktop);
//! - macOS: diskutil list -plist, монтирование diskutil mount.
//!
//! Поиск ничего не меняет в системе; монтирование — единственная операция,
//! требующая прав (Windows: администратор, Linux: активный desktop-сеанс).

use serde::{Deserialize, Serialize};

pub const ESP_GPT_TYPE: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EfiPartition {
    /// Идентификатор для монтирования: «disk 1, partition 2» (Win),
    /// «/dev/nvme0n1p1» (Linux), «disk0s1» (macOS)
    pub device: String,
    pub disk: Option<u32>,
    pub partition: Option<u32>,
    /// Точка монтирования с завершающим разделителем, если смонтирован
    pub mounted_at: Option<String>,
    pub size_mb: Option<u64>,
}

fn ps(script: &str) -> String {
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

// ---------------- Windows ----------------

#[derive(Deserialize, Debug, Clone)]
struct WinPartition {
    #[serde(rename = "DiskNumber", default)]
    disk_number: Option<u32>,
    #[serde(rename = "PartitionNumber", default)]
    partition_number: Option<u32>,
    #[serde(rename = "DriveLetter", default)]
    drive_letter: Option<String>,
    #[serde(rename = "Size", default)]
    size: Option<u64>,
}

fn win_json_to_parts(text: &str) -> Vec<WinPartition> {
    serde_json::from_str::<Vec<WinPartition>>(text).unwrap_or_else(|_| {
        serde_json::from_str::<WinPartition>(text)
            .map(|p| vec![p])
            .unwrap_or_default()
    })
}

fn windows_partitions() -> Vec<EfiPartition> {
    // ConvertTo-Json -InputObject сохраняет массив даже для одного элемента
    let out = ps(&format!(
        "$p = Get-Partition | Where-Object {{ $_.GptType -eq '{{{}}}' }}; ConvertTo-Json -Compress -InputObject @($p | Select-Object DiskNumber,PartitionNumber,DriveLetter,Size)",
        ESP_GPT_TYPE
    ));
    if out.is_empty() {
        return vec![];
    }
    win_json_to_parts(&out)
        .into_iter()
        .map(|p| {
            let device = match (p.disk_number, p.partition_number) {
                (Some(d), Some(n)) => format!("disk {d} partition {n}"),
                _ => "unknown".into(),
            };
            let mounted_at = p
                .drive_letter
                .filter(|l| !l.is_empty())
                .map(|l| format!("{l}\\"));
            EfiPartition {
                device,
                disk: p.disk_number,
                partition: p.partition_number,
                mounted_at,
                size_mb: p.size.map(|s| s / 1024 / 1024),
            }
        })
        .collect()
}

fn windows_mount(disk: u32, part: u32) -> Result<String, String> {
    let out = ps(&format!(
        "$p = Add-PartitionAccessPath -DiskNumber {disk} -PartitionNumber {part} -AssignDriveLetter -ErrorAction SilentlyContinue; if (-not $p) {{ $p = Get-Partition -DiskNumber {disk} -PartitionNumber {part} }}; if ($p -and $p.DriveLetter) {{ Write-Output $p.DriveLetter }} else {{ Write-Output 'FAIL' }}"
    ));
    let letter = out.trim().trim_end_matches('\\').to_string();
    if letter.is_empty() || letter == "FAIL" || letter.chars().count() > 2 {
        return Err("не удалось назначить букву EFI-разделу — запустите OpenEFI от имени администратора".into());
    }
    Ok(format!("{letter}\\"))
}

// ---------------- Linux ----------------

#[derive(Deserialize, Debug, Clone)]
struct LsblkDevice {
    name: String,
    #[serde(rename = "parttype", default)]
    parttype: Option<String>,
    #[serde(rename = "mountpoints", default)]
    mountpoints: Option<serde_json::Value>,
    #[serde(default)]
    children: Option<Vec<LsblkDevice>>,
}

fn collect_linux(devs: &[LsblkDevice], out: &mut Vec<EfiPartition>) {
    for d in devs {
        let is_esp = d
            .parttype
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case(ESP_GPT_TYPE))
            .unwrap_or(false);
        if is_esp {
            let mounted = match &d.mountpoints {
                Some(serde_json::Value::Array(arr)) => arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .next()
                    .map(str::to_string),
                Some(serde_json::Value::String(s)) => Some(s.clone()),
                _ => None,
            };
            out.push(EfiPartition {
                device: format!("/dev/{}", d.name),
                disk: None,
                partition: None,
                mounted_at: mounted.map(|m| {
                    if m.ends_with('/') {
                        m
                    } else {
                        format!("{m}/")
                    }
                }),
                size_mb: None,
            });
        }
        if let Some(ch) = &d.children {
            collect_linux(ch, out);
        }
    }
}

fn linux_json_to_parts(text: &str) -> Vec<EfiPartition> {
    let parsed: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_str(text).unwrap_or_default();
    let Some(serde_json::Value::Array(devs)) = parsed.get("blockdevices") else {
        return vec![];
    };
    let parsed: Vec<LsblkDevice> = devs
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();
    let mut out = Vec::new();
    collect_linux(&parsed, &mut out);
    out
}

fn linux_partitions() -> Vec<EfiPartition> {
    // mountpoints есть в новых lsblk, в старых — mountpoint; пробуем оба варианта
    let out = std::process::Command::new("lsblk")
        .args(["-J", "-o", "NAME,PARTTYPE,MOUNTPOINTS"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .filter(|s| s.contains("blockdevices"))
        .or_else(|| {
            std::process::Command::new("lsblk")
                .args(["-J", "-o", "NAME,PARTTYPE,MOUNTPOINT"])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        })
        .unwrap_or_default();
    if out.is_empty() {
        return vec![];
    }
    linux_json_to_parts(&out)
}

fn linux_mount(device: &str) -> Result<String, String> {
    // udisksctl есть на большинстве desktop-дистрибутивов и не требует root
    let out = std::process::Command::new("udisksctl")
        .args(["mount", "-b", device])
        .output()
        .map_err(|_| "udisksctl не найден — смонтируйте EFI-раздел вручную (например, sudo mount)".to_string())?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    if out.status.success() {
        if let Some(pos) = text.find(" at ") {
            let m = text[pos + 4..].trim().trim_end_matches('.').to_string();
            if !m.is_empty() {
                return Ok(if m.ends_with('/') { m } else { format!("{m}/") });
            }
        }
    }
    Err(format!(
        "не смонтировать {}: {} — попробуйте вручную (sudo mount)",
        device,
        String::from_utf8_lossy(&out.stderr).trim()
    ))
}

// ---------------- macOS ----------------

fn macos_partitions() -> Vec<EfiPartition> {
    let out = std::process::Command::new("diskutil")
        .args(["list", "-plist"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    if out.is_empty() {
        return vec![];
    }
    let Ok(value) = plist::Value::from_reader(std::io::Cursor::new(out.as_bytes())) else {
        return vec![];
    };
    let Some(all) = value
        .as_dictionary()
        .and_then(|d| d.get("AllDisksAndPartitions"))
        .and_then(|v| v.as_array())
    else {
        return vec![];
    };
    let mut parts: Vec<EfiPartition> = Vec::new();
    for disk in all {
        let Some(disk_dict) = disk.as_dictionary() else {
            continue;
        };
        let dev = disk_dict.get("DeviceIdentifier").and_then(|v| v.as_string());
        let Some(parts_arr) = disk_dict.get("Partitions").and_then(|v| v.as_array()) else {
            continue;
        };
        for p in parts_arr {
            let Some(pd) = p.as_dictionary() else {
                continue;
            };
            let content = pd.get("Content").and_then(|v| v.as_string());
            if content.map(|c| c.eq_ignore_ascii_case("EFI")) != Some(true) {
                continue;
            }
            let id = pd.get("DeviceIdentifier").and_then(|v| v.as_string());
            let size = pd.get("Size").and_then(|v| v.as_real()).map(|s| (s / 1024.0 / 1024.0) as u64);
            let id = match (id, dev) {
                (Some(id), _) => id.to_string(),
                (None, Some(dev)) => format!("{dev}-part"),
                _ => continue,
            };
            // точку монтирования даёт diskutil info (дополнительный вызов)
            let mounted_at = diskutil_mount_point(&id);
            parts.push(EfiPartition {
                device: id,
                disk: None,
                partition: None,
                mounted_at,
                size_mb: size,
            });
        }
    }
    parts
}

fn diskutil_mount_point(id: &str) -> Option<String> {
    let out = std::process::Command::new("diskutil")
        .args(["info", "-plist", id])
        .output()
        .ok()?;
    let value = plist::Value::from_reader(std::io::Cursor::new(out.stdout)).ok()?;
    let m = value
        .as_dictionary()?
        .get("MountPoint")
        .and_then(|v| v.as_string())?
        .to_string();
    (!m.is_empty()).then(|| {
        if m.ends_with('/') {
            m
        } else {
            format!("{m}/")
        }
    })
}

fn macos_mount(device: &str) -> Result<String, String> {
    let out = std::process::Command::new("diskutil")
        .args(["mount", device])
        .output()
        .map_err(|e| format!("не запустить diskutil: {e}"))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if out.status.success() {
        if let Some(pos) = text.find(" on ") {
            let m = text[pos + 4..].trim().to_string();
            if let Some(pos2) = m.find('(') {
                let m = m[..pos2].trim().to_string();
                if !m.is_empty() {
                    return Ok(if m.ends_with('/') { m } else { format!("{m}/") });
                }
            }
            return Ok(if m.ends_with('/') { m } else { format!("{m}/") });
        }
    }
    Err(format!("не смонтировать {}: {}", device, text.trim()))
}

// ---------------- Публичный API ----------------

/// Найти все EFI-разделы на машине (без изменений в системе)
pub fn find_esp() -> Vec<EfiPartition> {
    match std::env::consts::OS {
        "windows" => windows_partitions(),
        "linux" => linux_partitions(),
        "macos" => macos_partitions(),
        _ => vec![],
    }
}

/// Смонтировать EFI-раздел и вернуть точку монтирования
pub fn mount_esp(device: &str) -> Result<String, String> {
    match std::env::consts::OS {
        "windows" => {
            // device вида «disk 1 partition 2»
            let parts: Vec<&str> = device.split_whitespace().collect();
            if parts.len() == 4 && parts[0] == "disk" && parts[2] == "partition" {
                let disk = parts[1]
                    .parse::<u32>()
                    .map_err(|_| "некорректный идентификатор диска".to_string())?;
                let part = parts[3]
                    .parse::<u32>()
                    .map_err(|_| "некорректный идентификатор раздела".to_string())?;
                windows_mount(disk, part)
            } else {
                Err("некорректный идентификатор раздела (ожидается «disk N partition N»)".into())
            }
        }
        "linux" => linux_mount(device),
        "macos" => macos_mount(device),
        other => Err(format!("автомонтирование не поддерживается на {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_win_json_single_and_array() {
        let single = r#"{"DiskNumber":0,"PartitionNumber":1,"DriveLetter":null,"Size":268435456}"#;
        assert_eq!(win_json_to_parts(single).len(), 1);
        let arr = r#"[{"DiskNumber":0,"PartitionNumber":1,"DriveLetter":"Z","Size":268435456},{"DiskNumber":1,"PartitionNumber":2,"DriveLetter":null,"Size":524288000}]"#;
        let parts = win_json_to_parts(arr);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].drive_letter.as_deref(), Some("Z"));
        assert_eq!(parts[1].disk_number, Some(1));
    }

    #[test]
    fn test_win_partition_mapping() {
        let arr = r#"[{"DiskNumber":0,"PartitionNumber":1,"DriveLetter":null,"Size":268435456}]"#;
        let mapped: Vec<EfiPartition> = win_json_to_parts(arr)
            .into_iter()
            .map(|p| {
                let device = match (p.disk_number, p.partition_number) {
                    (Some(d), Some(n)) => format!("disk {d} partition {n}"),
                    _ => "unknown".into(),
                };
                let mounted_at = p
                    .drive_letter
                    .filter(|l| !l.is_empty())
                    .map(|l| format!("{l}\\"));
                EfiPartition {
                    device,
                    disk: p.disk_number,
                    partition: p.partition_number,
                    mounted_at,
                    size_mb: p.size.map(|s| s / 1024 / 1024),
                }
            })
            .collect();
        assert_eq!(mapped[0].device, "disk 0 partition 1");
        assert_eq!(mapped[0].mounted_at, None);
        assert_eq!(mapped[0].size_mb, Some(256));
    }

    #[test]
    fn test_linux_lsblk_parse() {
        let json = r#"{"blockdevices":[{"name":"nvme0n1","parttype":null,"mountpoints":[null],"children":[{"name":"nvme0n1p1","parttype":"c12a7328-f81f-11d2-ba4b-00a0c93ec93b","mountpoints":["/boot/efi"]},{"name":"nvme0n1p2","parttype":"ebd0a0a2-b9e5-4433-87c0-68b6b72699c7","mountpoints":null}]},{"name":"sda","children":[{"name":"sda1","parttype":"C12A7328-F81F-11D2-BA4B-00A0C93EC93B","mountpoints":[]}]}]}"#;
        let parts = linux_json_to_parts(json);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].device, "/dev/nvme0n1p1");
        assert_eq!(parts[0].mounted_at.as_deref(), Some("/boot/efi/"));
        assert_eq!(parts[1].device, "/dev/sda1");
        assert_eq!(parts[1].mounted_at, None, "пустой список монтирований");
    }

    #[test]
    fn test_linux_lsblk_string_mountpoint() {
        let json = r#"{"blockdevices":[{"name":"sda1","parttype":"c12a7328-f81f-11d2-ba4b-00a0c93ec93b","mountpoints":"/mnt/esp"}]}"#;
        let parts = linux_json_to_parts(json);
        assert_eq!(parts[0].mounted_at.as_deref(), Some("/mnt/esp/"));
    }

    #[test]
    fn test_find_esp_runs() {
        // не должен паниковать ни на одной ОС; в CI-песочнице разделов может не быть
        let parts = find_esp();
        let _ = parts.len();
    }
}
