//! USB-маппинг: установка сгенерированного UTBMap.kext в EFI и
//! регистрация его в config.plist (Kernel → Add) с отключением UTBDefault.
//!
//! Поток по USBToolBox: пользователь снимает маппинг утилитой (Windows/macOS),
//! получает UTBMap.kext, а OpenEFI кладёт его в EFI/OC/Kexts и правит конфиг.

use std::path::{Path, PathBuf};

use plist::{Dictionary, Value};

use crate::downloader::resolve_efi_base;

/// Отчёт установки UTBMap
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct UtbMapReport {
    /// Куда лёг кекст (EFI/OC/Kexts)
    pub kexts_dir: String,
    /// Путь к отредактированному config.plist, если он был найден
    pub config_path: Option<String>,
    /// Сохранили ли резервную копию конфига
    pub backup_made: bool,
    /// Добавлена/обновлена запись UTBMap в Kernel/Add
    pub utbmap_registered: bool,
    /// Отключён ли UTBDefault в конфиге (Enabled=false)
    pub utbdefault_disabled: bool,
}

/// Найти UTBMap.kext: допускаем сам бандл или папку, где он лежит
fn locate_utbmap(selected: &Path) -> Result<PathBuf, String> {
    let looks_like_kext = |p: &Path| {
        p.is_dir()
            && p.file_name()
                .map(|n| n.to_string_lossy().to_lowercase().ends_with(".kext"))
                .unwrap_or(false)
            && p.join("Contents").is_dir()
    };
    if looks_like_kext(selected) {
        return Ok(selected.to_path_buf());
    }
    // возможно, выбрали папку вывода утилиты — ищем UTBMap.kext внутри
    for name in ["UTBMap.kext", "utbmap.kext"] {
        let cand = selected.join(name);
        if looks_like_kext(&cand) {
            return Ok(cand);
        }
    }
    Err("в выбранной папке не найден UTBMap.kext — выберите сам бандл или папку с ним".into())
}

/// Рекурсивное копирование папки (замена существующего назначения)
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if dst.exists() {
        std::fs::remove_dir_all(dst).map_err(|e| format!("не заменить старый {}: {e}", dst.display()))?;
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

/// Запись кекта в Kernel → Add
fn kext_add_entry(bundle_path: &str, comment: &str) -> Value {
    let mut d = Dictionary::new();
    d.insert("Arch".into(), Value::String("Any".into()));
    d.insert("BundlePath".into(), Value::String(bundle_path.into()));
    d.insert("Comment".into(), Value::String(comment.into()));
    d.insert("Enabled".into(), Value::Boolean(true));
    d.insert("ExecutablePath".into(), Value::String("Contents/MacOS/UTBMap".into()));
    d.insert("MaxKernel".into(), Value::String(String::new()));
    d.insert("MinKernel".into(), Value::String(String::new()));
    d.insert("PlistPath".into(), Value::String("Contents/Info.plist".into()));
    Value::Dictionary(d)
}

/// Правка config.plist: включить/добавить UTBMap, отключить UTBDefault
fn patch_config(config: &Path) -> Result<(bool, bool), String> {
    let mut root = Value::from_reader(std::io::BufReader::new(
        std::fs::File::open(config).map_err(|e| format!("не открыть {}: {e}", config.display()))?,
    ))
    .map_err(|e| format!("не разобрать {}: {e}", config.display()))?
    .into_dictionary()
    .ok_or_else(|| format!("{} — не словарь plist", config.display()))?;

    let kernel = root
        .get_mut("Kernel")
        .and_then(|v| v.as_dictionary_mut())
        .ok_or("в config.plist нет секции Kernel")?;
    let add = match kernel.get_mut("Add") {
        Some(v) if matches!(v, Value::Array(_)) => v,
        Some(_) => return Err("Kernel → Add в config.plist — не массив".into()),
        None => {
            kernel.insert("Add".into(), Value::Array(Vec::new()));
            kernel.get_mut("Add").ok_or("не создать Kernel → Add")?
        }
    };
    let entries = add.as_array_mut().unwrap();

    let mut utbmap_registered = false;
    let mut utbdefault_disabled = false;

    // уже есть запись UTBMap? — включаем
    for e in entries.iter_mut() {
        let Some(d) = e.as_dictionary_mut() else { continue };
        let is_utbmap = d
            .get("BundlePath")
            .and_then(|b| b.as_string())
            .map(|b| b.to_lowercase().ends_with("utbmap.kext"))
            .unwrap_or(false);
        if is_utbmap {
            d.insert("Enabled".into(), Value::Boolean(true));
            utbmap_registered = true;
            break;
        }
    }
    if !utbmap_registered {
        entries.push(kext_add_entry("Kexts/UTBMap.kext", "USB map by USBToolBox"));
        utbmap_registered = true;
    }

    // по документации USBToolBox UTBDefault заменяется маппингом — отключаем
    for e in entries.iter_mut() {
        let Some(d) = e.as_dictionary_mut() else { continue };
        let is_default = d
            .get("BundlePath")
            .and_then(|b| b.as_string())
            .map(|b| b.to_lowercase().ends_with("utbdefault.kext"))
            .unwrap_or(false);
        if is_default {
            d.insert("Enabled".into(), Value::Boolean(false));
            utbdefault_disabled = true;
        }
    }

    // резервная копия до записи
    let backup = config.with_extension("plist.bak-openefi");
    std::fs::copy(config, &backup).map_err(|e| format!("не сделать копию {}: {e}", backup.display()))?;

    let mut buf = Vec::new();
    Value::Dictionary(root)
        .to_writer_xml(&mut buf)
        .map_err(|e| format!("не сериализовать {}: {e}", config.display()))?;
    std::fs::write(config, buf).map_err(|e| format!("не записать {}: {e}", config.display()))?;
    Ok((utbmap_registered, utbdefault_disabled))
}

/// Полный сценарий: найти UTBMap.kext → скопировать в EFI/OC/Kexts → править конфиг
pub fn install_utbmap(kext_selected: &Path, efi_selected: &Path) -> Result<UtbMapReport, String> {
    let src = locate_utbmap(kext_selected)?;
    let efi_base = resolve_efi_base(efi_selected);
    let kexts_dir = efi_base.join("OC").join("Kexts");
    std::fs::create_dir_all(&kexts_dir)
        .map_err(|e| format!("не создать {} — папка может быть защищена от записи: {e}", kexts_dir.display()))?;

    let bundle_name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "UTBMap.kext".into());
    copy_dir_recursive(&src, &kexts_dir.join(&bundle_name))?;

    let config = efi_base.join("OC").join("config.plist");
    let (config_path, backup_made, utbmap_registered, utbdefault_disabled) = if config.is_file() {
        let (reg, dis) = patch_config(&config)?;
        (Some(config.to_string_lossy().to_string()), true, reg, dis)
    } else {
        (None, false, false, false)
    };

    Ok(UtbMapReport {
        kexts_dir: kexts_dir.to_string_lossy().to_string(),
        config_path,
        backup_made,
        utbmap_registered,
        utbdefault_disabled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_minimal_kext(dst: &Path) {
        std::fs::create_dir_all(dst.join("Contents/MacOS")).unwrap();
        std::fs::write(
            dst.join("Contents/Info.plist"),
            "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>de.usbtoolbox.utbmap</string></dict></plist>",
        )
        .unwrap();
        std::fs::write(dst.join("Contents/MacOS/UTBMap"), b"binary").unwrap();
    }

    fn config_with_utbdefault() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
  <key>Kernel</key>
  <dict>
    <key>Add</key>
    <array>
      <dict>
        <key>Arch</key><string>Any</string>
        <key>BundlePath</key><string>Kexts/USBToolBox.kext</string>
        <key>Enabled</key><true/>
        <key>ExecutablePath</key><string>Contents/MacOS/USBToolBox</string>
        <key>MaxKernel</key><string></string>
        <key>MinKernel</key><string></string>
        <key>PlistPath</key><string>Contents/Info.plist</string>
      </dict>
      <dict>
        <key>Arch</key><string>Any</string>
        <key>BundlePath</key><string>Kexts/UTBDefault.kext</string>
        <key>Enabled</key><true/>
        <key>ExecutablePath</key><string>Contents/MacOS/UTBDefault</string>
        <key>MaxKernel</key><string></string>
        <key>MinKernel</key><string></string>
        <key>PlistPath</key><string>Contents/Info.plist</string>
      </dict>
    </array>
  </dict>
</dict>
</plist>"#
            .into()
    }

    fn bundles<'a>(root: &'a Dictionary) -> Vec<(String, bool)> {
        root.get("Kernel")
            .and_then(|k| k.as_dictionary())
            .and_then(|k| k.get("Add"))
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_dictionary())
                    .filter_map(|d| {
                        let bp = d.get("BundlePath")?.as_string()?.to_string();
                        let en = d.get("Enabled")?.as_boolean()?;
                        Some((bp, en))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn test_install_utbmap_full_flow() {
        let tmp = std::env::temp_dir().join(format!("openefi-utbmap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        // «вывод утилиты»: папка с готовым UTBMap.kext
        let tool_out = tmp.join("tool-output");
        write_minimal_kext(&tool_out.join("UTBMap.kext"));

        // EFI-папка с конфигом, где уже есть UTBDefault
        let efi = tmp.join("volume/EFI");
        let oc = efi.join("OC");
        std::fs::create_dir_all(&oc).unwrap();
        std::fs::write(oc.join("config.plist"), config_with_utbdefault()).unwrap();

        let report = install_utbmap(&tool_out, &tmp.join("volume")).unwrap();
        assert_eq!(report.kexts_dir, oc.join("Kexts").to_string_lossy().to_string());
        assert!(oc.join("Kexts/UTBMap.kext/Contents/MacOS/UTBMap").exists());
        assert!(report.utbmap_registered);
        assert!(report.utbdefault_disabled);
        assert!(report.config_path.is_some());
        assert!(oc.join("config.plist.bak-openefi").exists(), "нужна резервная копия");

        let root = Value::from_reader(std::fs::File::open(oc.join("config.plist")).unwrap())
            .unwrap()
            .into_dictionary()
            .unwrap();
        let bs = bundles(&root);
        let utbmap = bs.iter().find(|(b, _)| b.ends_with("UTBMap.kext")).unwrap();
        assert!(utbmap.1, "UTBMap должен быть включён");
        let utbdefault = bs.iter().find(|(b, _)| b.ends_with("UTBDefault.kext")).unwrap();
        assert!(!utbdefault.1, "UTBDefault должен быть отключён");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_install_utbmap_without_config() {
        let tmp = std::env::temp_dir().join(format!("openefi-utbmap2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        let kext = tmp.join("src/UTBMap.kext");
        write_minimal_kext(&kext);
        let efi = tmp.join("volume/EFI");
        std::fs::create_dir_all(&efi).unwrap();

        let report = install_utbmap(&kext, &tmp.join("volume")).unwrap();
        assert!(report.config_path.is_none(), "конфига нет — править нечего");
        assert!(!report.utbmap_registered);
        assert!(efi.join("OC/Kexts/UTBMap.kext/Contents/Info.plist").exists());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_install_utbmap_kext_not_found() {
        let tmp = std::env::temp_dir().join(format!("openefi-utbmap3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let empty = tmp.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        let efi = tmp.join("volume/EFI");
        std::fs::create_dir_all(&efi).unwrap();
        assert!(install_utbmap(&empty, &tmp.join("volume")).is_err());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
