//! Обновление установленных кекстов: читаем версии из Info.plist бандлов
//! в EFI/OC/Kexts, сверяем с последними релизами GitHub и предлагаем
//! обновить устаревшие тем же конвейером, что и первичную установку.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::configaudit::read_bundle_version;
use crate::downloader::{
    fetch_latest_release, http_agent, pick_asset, resolve_efi_base, version_from_asset,
};

/// Кект, установленный в EFI/OC/Kexts
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstalledKext {
    /// Имя бандла на диске («Lilu.kext»)
    pub bundle: String,
    /// Версия из Contents/Info.plist
    pub version: Option<String>,
    /// Репозиторий из каталога, если кект известен
    pub repo: Option<String>,
    /// Имя неизвестно каталогу OpenEFI (форк или ручная сборка)
    pub known: bool,
}

/// Результат сверки одного кекта с GitHub
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KextUpdateCheck {
    pub bundle: String,
    pub repo: Option<String>,
    pub installed: Option<String>,
    pub latest: Option<String>,
    pub outdated: bool,
    /// Пояснение, почему сравнить не удалось (переводится tb())
    pub message: Option<String>,
}

/// Итог проверки обновлений
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KextUpdatesReport {
    /// Что лежит на диске
    pub installed: Vec<InstalledKext>,
    /// Сверка с GitHub (только для кектов с известным репозиторием)
    pub checks: Vec<KextUpdateCheck>,
}

/// Сравнение версий «1.2.10» > «1.2.9» (по числовым сегментам)
pub fn version_gt(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.trim()
            .trim_start_matches('v')
            .split('.')
            .map(|p| p.trim().parse::<u64>().unwrap_or(0))
            .collect()
    };
    let (a, b) = (parse(a), parse(b));
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

/// Сканировать EFI/OC/Kexts: бандлы, версии, репозитории
pub fn scan_installed(selected: &Path) -> Result<Vec<InstalledKext>, String> {
    let efi_base = resolve_efi_base(selected);
    let kexts_dir = efi_base.join("OC").join("Kexts");
    if !kexts_dir.is_dir() {
        return Err("в EFI/OC нет папки Kexts — сначала установите OpenCore и кексты".into());
    }
    let mut out: Vec<InstalledKext> = Vec::new();
    let mut rd = std::fs::read_dir(&kexts_dir)
        .map_err(|e| format!("не читать {}: {e}", kexts_dir.display()))?;
    let mut bundles: Vec<String> = rd
        .by_ref()
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.to_lowercase().ends_with(".kext"))
        .collect();
    bundles.sort_by_key(|a| a.to_lowercase());
    for bundle in bundles {
        let info = find_info_plist(&kexts_dir.join(&bundle));
        let version = info.and_then(|p| read_bundle_version(&p));
        out.push(InstalledKext {
            known: crate::kexts::is_known(&bundle),
            repo: crate::kexts::repo_for(&bundle),
            bundle,
            version,
        });
    }
    Ok(out)
}

/// Info.plist бандла: Contents/Info.plist либо Info.plist в корне (некоторые форки)
fn find_info_plist(bundle_dir: &Path) -> Option<PathBuf> {
    let std_path = bundle_dir.join("Contents").join("Info.plist");
    if std_path.is_file() {
        return Some(std_path);
    }
    let alt = bundle_dir.join("Info.plist");
    alt.is_file().then_some(alt)
}

/// Сверить установленные кекты с последними релизами GitHub.
/// Ошибки сети/репозитория не прерывают проверку — попадают в message.
pub fn check_updates(installed: &[InstalledKext]) -> Vec<KextUpdateCheck> {
    let agent = http_agent();
    let mut checks: Vec<KextUpdateCheck> = Vec::new();
    // группируем по репозиторию: один запрос к API на репозиторий
    let mut repos: Vec<(String, Vec<&InstalledKext>)> = Vec::new();
    for k in installed {
        let Some(repo) = k.repo.as_ref().filter(|r| r.split('/').count() == 2) else {
            checks.push(KextUpdateCheck {
                bundle: k.bundle.clone(),
                repo: k.repo.clone(),
                installed: k.version.clone(),
                latest: None,
                outdated: false,
                message: Some(if k.known {
                    "репозиторий не указан — обновляйте вручную".into()
                } else {
                    "неизвестный OpenEFI кекст — обновляйте вручную".into()
                }),
            });
            continue;
        };
        match repos.iter_mut().find(|(r, _)| r == repo) {
            Some((_, list)) => list.push(k),
            None => repos.push((repo.clone(), vec![k])),
        }
    }

    for (repo, group) in repos {
        let release = match fetch_latest_release(&agent, &repo) {
            Ok(r) => r,
            Err(e) => {
                for k in group {
                    checks.push(KextUpdateCheck {
                        bundle: k.bundle.clone(),
                        repo: Some(repo.clone()),
                        installed: k.version.clone(),
                        latest: None,
                        outdated: false,
                        message: Some(e.clone()),
                    });
                }
                continue;
            }
        };
        let repo_name = repo.rsplit('/').next().unwrap_or("");
        for k in group {
            let name = k.bundle.trim_end_matches(".kext");
            let latest = pick_asset(&release.assets, name, repo_name, None)
                .and_then(|a| version_from_asset(&a.name, name));
            let has_latest = latest.is_some();
            let outdated = match (&k.version, &latest) {
                (Some(cur), Some(new)) => version_gt(new, cur),
                _ => false,
            };
            checks.push(KextUpdateCheck {
                bundle: k.bundle.clone(),
                repo: Some(repo.clone()),
                installed: k.version.clone(),
                latest,
                outdated,
                message: if has_latest {
                    None
                } else {
                    Some("в релизе нет подходящего архива — сверяйте вручную".into())
                },
            });
        }
    }

    checks
}

/// Обновить выбранные кекты: тот же конвейер, что и первичная установка
/// (скачивание релиза + распаковка + автобэкап EFI/OC).
pub fn update_kexts(
    bundles: &[String],
    selected_dir: &Path,
    darwin: u32,
    progress: impl FnMut(crate::downloader::DownloadProgress),
) -> Result<crate::downloader::KextsToEfiReport, String> {
    // проверяем, что EFI/OC/Kexts существует (заодно получаем актуальные пути)
    scan_installed(selected_dir)?;
    let items: Vec<crate::downloader::KextDownloadItem> = bundles
        .iter()
        .map(|b| {
            let name = b.trim().trim_end_matches(".kext").to_string();
            crate::downloader::KextDownloadItem {
                repo: crate::kexts::repo_for(&name),
                name,
            }
        })
        .collect();
    crate::downloader::download_kexts(&items, selected_dir, darwin, progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_gt() {
        assert!(version_gt("1.7.1", "1.7.0"));
        assert!(version_gt("1.2.10", "1.2.9"));
        assert!(version_gt("2.0", "1.9.9"));
        assert!(version_gt("1.10", "1.9"));
        assert!(!version_gt("1.7.0", "1.7.0"));
        assert!(!version_gt("1.6.9", "1.7.0"));
        assert!(!version_gt("1.7", "1.7.0"));
        assert!(version_gt("v1.8", "1.7.0"));
    }

    #[test]
    fn test_repo_lookup() {
        assert_eq!(
            crate::kexts::repo_for("Lilu.kext").as_deref(),
            Some("acidanthera/Lilu")
        );
        assert_eq!(
            crate::kexts::repo_for("WhateverGreen").as_deref(),
            Some("acidanthera/WhateverGreen")
        );
        assert_eq!(crate::kexts::repo_for("UTBMap.kext"), None);
        assert!(crate::kexts::is_known("Lilu.kext"));
        assert!(!crate::kexts::is_known("MyFork.kext"));
    }

    #[test]
    fn test_scan_installed_versions() {
        let tmp = std::env::temp_dir().join(format!("openefi-kupd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let kexts_dir = tmp.join("EFI").join("OC").join("Kexts");
        for (name, ver) in [("Lilu.kext", "1.6.8"), ("MyFork.kext", "0.4.2")] {
            let d = kexts_dir.join(name).join("Contents");
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(
                d.join("Info.plist"),
                format!(
                    "<plist><dict><key>CFBundleShortVersionString</key><string>{ver}</string></dict></plist>"
                ),
            )
            .unwrap();
        }
        let list = scan_installed(&tmp).unwrap();
        assert_eq!(list.len(), 2);
        let lilu = list.iter().find(|k| k.bundle == "Lilu.kext").unwrap();
        assert_eq!(lilu.version.as_deref(), Some("1.6.8"));
        assert_eq!(lilu.repo.as_deref(), Some("acidanthera/Lilu"));
        assert!(lilu.known);
        let fork = list.iter().find(|k| k.bundle == "MyFork.kext").unwrap();
        assert_eq!(fork.version.as_deref(), Some("0.4.2"));
        assert!(!fork.known);
        assert_eq!(fork.repo, None);

        // без Kexts — ошибка
        let empty = std::env::temp_dir().join(format!("openefi-kupd2-{}", std::process::id()));
        std::fs::create_dir_all(empty.join("EFI").join("OC")).unwrap();
        assert!(scan_installed(&empty).is_err());

        std::fs::remove_dir_all(&tmp).ok();
        std::fs::remove_dir_all(&empty).ok();
    }

    #[test]
    fn test_check_updates_shape_without_network_knowledge() {
        // кекты без репозитория не дергают сеть и получают message
        let installed = vec![
            InstalledKext {
                bundle: "UTBMap.kext".into(),
                version: Some("1.1".into()),
                repo: None,
                known: true,
            },
            InstalledKext {
                bundle: "MyFork.kext".into(),
                version: None,
                repo: None,
                known: false,
            },
        ];
        let checks = check_updates(&installed);
        assert_eq!(checks.len(), 2);
        assert!(checks.iter().all(|c| !c.outdated && c.latest.is_none()));
        assert!(checks.iter().all(|c| c.message.is_some()));
    }
}
