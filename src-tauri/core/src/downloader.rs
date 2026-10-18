//! Загрузка кекстов из GitHub Releases прямо в EFI-папку.
//!
//! Логика: для каждого выбранного кекта известен репозиторий (`owner/name`).
//! Берём последний релиз, среди ассетов выбираем подходящий ZIP по эвристике
//! (имя кекта, вариант под целевую macOS, RELEASE вместо DEBUG), скачиваем
//! и распаковываем все `.kext`-бандлы из архива в `EFI/OC/Kexts`.

use std::collections::HashSet;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use plist::{Dictionary, Value};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = concat!("OpenEFI/", env!("CARGO_PKG_VERSION"), " (github.com/mel0k1/OpenEFI)");
const MAX_DOWNLOAD_BYTES: u64 = 256 * 1024 * 1024;
const MAX_EXTRACT_BYTES: u64 = 512 * 1024 * 1024;

/// Кект, который нужно скачать
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KextDownloadItem {
    pub name: String,
    pub repo: Option<String>,
}

/// Результат скачивания одного кекта
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KextDownloadResult {
    pub name: String,
    pub ok: bool,
    /// Версия, вытащенная из имени ассета (например «1.7.0»)
    pub version: Option<String>,
    /// Какие .kext-бандлы реально легли на диск
    pub installed: Vec<String>,
    /// Причина неудачи/пропуска
    pub message: Option<String>,
}

/// Итоговый отчёт установки в EFI
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KextsToEfiReport {
    /// Каталог, куда легли кексты (например «/Volumes/EFI/EFI/OC/Kexts»)
    pub kexts_dir: String,
    pub results: Vec<KextDownloadResult>,
    /// Куда сделан автоматический бэкап EFI/OC перед изменениями
    #[serde(default)]
    pub backup_path: Option<String>,
    /// Предупреждения защиты EFI (нет OpenCore, не похоже на ESP и т.п.)
    #[serde(default)]
    pub audit_warnings: Vec<String>,
}

/// Прогресс: сколько кектов обработано из скольких
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DownloadProgress {
    pub done: usize,
    pub total: usize,
    pub kext: String,
}

#[derive(Deserialize, Debug)]
struct Release {
    #[allow(dead_code)]
    tag_name: Option<String>,
    assets: Vec<Asset>,
}

#[derive(Deserialize, Debug)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Имена macOS для подбора вариантных ассетов (AirportItlwm и т.п.)
pub fn macos_name(darwin: u32) -> Option<&'static str> {
    Some(match darwin {
        19 => "Catalina",
        20 => "BigSur",
        21 => "Monterey",
        22 => "Ventura",
        23 => "Sonoma",
        24 => "Sequoia",
        25 => "Tahoe",
        _ => return None,
    })
}

/// Все известные имена macOS встречаются в названиях ассетов — штрафуем нецелевые
const MACOS_VARIANTS: &[&str] = &[
    "catalina", "bigsur", "big_sur", "monterey", "ventura", "sonoma", "sequoia", "tahoe",
];

fn normalize(s: &str) -> String {
    s.to_lowercase()
}

/// Скоринг ассета под конкретный кект. None — ассет не кандидат (не ZIP).
fn asset_score(
    asset_name: &str,
    kext_name: &str,
    repo_name: &str,
    macos_hint: Option<&str>,
) -> Option<i32> {
    let a = normalize(asset_name);
    if !a.ends_with(".zip") {
        return None;
    }
    let k = normalize(kext_name);
    let mut s = 0;
    if a.starts_with(&k) {
        s += 12;
    } else if a.contains(&k) {
        s += 2;
    } else {
        // имя кекта не найдено — пробуем имя репозитория как слабый сигнал
        let rn = normalize(repo_name);
        if !rn.is_empty() && a.starts_with(&rn) {
            s += 1;
        }
    }
    if let Some(mh) = macos_hint {
        let mh = normalize(mh);
        if !mh.is_empty() && a.contains(&mh) {
            s += 5;
        }
    }
    for other in MACOS_VARIANTS {
        if Some(*other) != macos_hint.map(normalize).as_deref() && a.contains(other) {
            s -= 4;
        }
    }
    if a.contains("release") {
        s += 3;
    } else if a.contains("debug") {
        s -= 4;
    }
    if a.contains("alpha") {
        s -= 3;
    }
    if a.contains("beta") {
        s -= 2;
    }
    Some(s)
}

/// Выбор лучшего ассета для кекта из списка ассетов релиза
pub fn pick_asset<'a>(
    assets: &'a [Asset],
    kext_name: &str,
    repo_name: &str,
    macos_hint: Option<&str>,
) -> Option<&'a Asset> {
    let mut best: Option<(i32, &'a Asset)> = None;
    for a in assets {
        if let Some(score) = asset_score(&a.name, kext_name, repo_name, macos_hint) {
            if best.map(|(bs, _)| score > bs).unwrap_or(true) {
                best = Some((score, a));
            }
        }
    }
    best.map(|(_, a)| a)
}

/// Версия кекта из имени ассета: «VirtualSMC-1.3.4-RELEASE.zip» → «1.3.4»
pub fn version_from_asset(asset_name: &str, kext_name: &str) -> Option<String> {
    let a = asset_name.to_lowercase();
    let k = kext_name.to_lowercase();
    let rest = a.split_once(&k).map(|(_, r)| r)?;
    let rest = rest.trim_start_matches(|c: char| !(c.is_ascii_digit() || c == '.'));
    let ver: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let ver = ver.trim_matches('.');
    if ver.is_empty() { None } else { Some(ver.to_string()) }
}

/// Определяем базу EFI по выбранной пользователем папке:
/// - выбрали том с папкой EFI внутри → том/EFI
/// - выбрали саму папку EFI → она
/// - выбрали папку OC → её родитель (EFI)
/// - иначе считаем, что выбранная папка и есть EFI
pub fn resolve_efi_base(selected: &Path) -> PathBuf {
    let name = selected
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if name == "efi" {
        return selected.to_path_buf();
    }
    if name == "oc" {
        return selected
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| selected.to_path_buf());
    }
    let inner_efi = selected.join("EFI");
    if inner_efi.is_dir() {
        return inner_efi;
    }
    selected.to_path_buf()
}

fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .user_agent(USER_AGENT)
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout_read(std::time::Duration::from_secs(120))
        .build()
}

fn fetch_latest_release(agent: &ureq::Agent, repo: &str) -> Result<Release, String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    fetch_release_url(agent, &url, repo)
}

fn fetch_release_by_tag(agent: &ureq::Agent, repo: &str, tag: &str) -> Result<Release, String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/tags/{tag}");
    fetch_release_url(agent, &url, repo)
}

fn fetch_release_url(agent: &ureq::Agent, url: &str, repo: &str) -> Result<Release, String> {
    let resp = agent
        .get(url)
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => match code {
                404 => format!("в репозитории {repo} нет релизов"),
                403 => "лимит запросов GitHub API исчерпан — повторите позже".into(),
                c => format!("GitHub API вернул {c} для {repo}"),
            },
            _ => format!("нет связи с GitHub ({repo})"),
        })?;
    let text = resp.into_string().map_err(|e| format!("ответ GitHub не читается: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("неожиданный ответ GitHub API: {e}"))
}

/// Скачивание файла потоком на диск (для крупных ассетов и утилит)
pub fn download_to_file(agent: &ureq::Agent, url: &str, dest: &Path) -> Result<u64, String> {
    let resp = agent
        .get(url)
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => format!("скачивание прервано: HTTP {code}"),
            _ => "нет связи с сервером при скачивании".into(),
        })?;
    let mut reader = resp.into_reader().take(MAX_DOWNLOAD_BYTES + 1);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("не создать папку {}: {e}", parent.display()))?;
    }
    let mut out = std::fs::File::create(dest)
        .map_err(|e| format!("не записать {}: {e}", dest.display()))?;
    let mut written: u64 = 0;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("ошибка чтения: {e}"))?;
        if n == 0 {
            break;
        }
        written += n as u64;
        if written > MAX_DOWNLOAD_BYTES {
            drop(out);
            let _ = std::fs::remove_file(dest);
            return Err("файл слишком большой".into());
        }
        out.write_all(&buf[..n])
            .map_err(|e| format!("ошибка записи {}: {e}", dest.display()))?;
    }
    Ok(written)
}

/// Утилита USBToolBox: репозиторий, тег (None — последний релиз), имя ассета
pub fn utb_tool_asset(target: &str) -> Result<(&'static str, Option<&'static str>, &'static str), String> {
    match target {
        // macOS-сборка тулза не обновлялась с 0.1.1 (рекомендация самого USBToolBox)
        "macos" => Ok(("USBToolBox/tool", Some("0.1.1"), "macOS.zip")),
        "windows" => Ok(("USBToolBox/tool", None, "Windows.exe")),
        other => Err(format!("неизвестная цель утилиты: {other}")),
    }
}

/// Скачать утилиту USBToolBox по цели (windows|macos) в указанный файл
pub fn download_utb_tool(target: &str, dest: &Path) -> Result<String, String> {
    let (repo, tag, asset_name) = utb_tool_asset(target)?;
    let agent = http_agent();
    let release = match tag {
        Some(t) => fetch_release_by_tag(&agent, repo, t)?,
        None => fetch_latest_release(&agent, repo)?,
    };
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| format!("в релизе {repo} нет ассета {asset_name}"))?;
    download_to_file(&agent, &asset.browser_download_url, dest)?;
    Ok(asset.name.clone())
}

/// Публичный ассет релиза (имя + ссылка) для диалогов сохранения
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PickedAsset {
    pub name: String,
    pub browser_download_url: String,
}

/// Найти в последнем релизе репозитория ассет по расширению (например «.dmg»)
pub fn pick_release_asset(repo: &str, ext: &str) -> Result<PickedAsset, String> {
    let agent = http_agent();
    let release = fetch_latest_release(&agent, repo)?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.to_lowercase().ends_with(ext))
        .ok_or_else(|| format!("в релизе {repo} нет ассета с расширением {ext}"))?;
    Ok(PickedAsset {
        name: asset.name.clone(),
        browser_download_url: asset.browser_download_url.clone(),
    })
}

/// Скачать файл по прямой ссылке на диск
pub fn download_to_file_url(url: &str, dest: &Path) -> Result<(), String> {
    let agent = http_agent();
    download_to_file(&agent, url, dest).map(|_| ())
}

fn fetch_bytes(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, String> {
    let resp = agent
        .get(url)
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => format!("скачивание прервано: HTTP {code}"),
            _ => "нет связи с сервером при скачивании".into(),
        })?;
    let mut reader = resp
        .into_reader()
        .take(MAX_DOWNLOAD_BYTES + 1);
    let mut buf: Vec<u8> = Vec::new();
    reader
        .read_to_end(&mut buf)
        .map_err(|e| format!("ошибка чтения архива: {e}"))?;
    if buf.len() as u64 > MAX_DOWNLOAD_BYTES {
        return Err("архив слишком большой".into());
    }
    Ok(buf)
}

/// Распаковка всех `.kext`-бандлов из ZIP-архива в `dest_dir`.
/// Возвращает имена установленных бандлов («Lilu.kext»).
/// Поддерживает вложенные пути («Release/Lilu.kext/…»), защищён от zip-slip.
pub fn extract_kexts_from_zip<R: Read + Seek>(
    reader: R,
    dest_dir: &Path,
) -> Result<Vec<String>, String> {
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("битый ZIP-архив: {e}"))?;

    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().replace('\\', "/")))
        .collect();

    // корни бандлов: минимальные префиксы путей, оканчивающиеся на *.kext
    let mut roots: Vec<String> = Vec::new();
    for name in &names {
        let comps: Vec<&str> = name.split('/').collect();
        for (i, c) in comps.iter().enumerate() {
            if c.to_lowercase().ends_with(".kext") && i + 1 < comps.len() {
                let root = comps[..=i].join("/");
                if !roots.iter().any(|r| r == &root) {
                    roots.push(root);
                }
            }
        }
    }
    if roots.is_empty() {
        return Err("в архиве нет .kext-бандлов".into());
    }
    // не берём вложенные корни, если есть более внешний
    let kept: Vec<String> = roots
        .iter()
        .filter(|r| !roots.iter().any(|other| other.as_str() != r.as_str() && r.starts_with(other.as_str())))
        .cloned()
        .collect();
    roots = kept;

    std::fs::create_dir_all(dest_dir).map_err(|e| format!("не создать папку кекстов: {e}"))?;
    let mut cleaned: HashSet<String> = HashSet::new();
    let mut written: u64 = 0;
    let mut installed: Vec<String> = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("битый ZIP: {e}"))?;
        let name = file.name().replace('\\', "/");
        let Some(root) = roots
            .iter()
            .filter(|r| name.starts_with(r.as_str()) && name.len() > r.len() + 1)
            .max_by_key(|r| r.len())
            .cloned()
        else {
            continue;
        };
        let bundle = root.rsplit('/').next().unwrap_or(&root).to_string();
        if !bundle.to_lowercase().ends_with(".kext") {
            continue;
        }
        let rest = &name[root.len() + 1..];
        // защита от zip-slip: никаких ".." в остатке пути
        if rest.split('/').any(|c| c == "..") {
            continue;
        }
        let bundle_dir = dest_dir.join(&bundle);
        if cleaned.insert(bundle.clone()) {
            if bundle_dir.exists() {
                std::fs::remove_dir_all(&bundle_dir)
                    .map_err(|e| format!("не заменить старый {}: {e}", bundle))?;
            }
        }
        let dest = bundle_dir.join(rest);
        if !dest.starts_with(dest_dir) {
            continue; // попытка вылезти за пределы каталога
        }
        written += file.size();
        if written > MAX_EXTRACT_BYTES {
            return Err("архив подозрительно большой (zip-bomb?)".into());
        }
        if file.is_dir() {
            std::fs::create_dir_all(&dest).map_err(|e| format!("не создать папку: {e}"))?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("не создать папку: {e}"))?;
            }
            let mut out = std::fs::File::create(&dest)
                .map_err(|e| format!("не записать {}: {e}", dest.display()))?;
            std::io::copy(&mut file, &mut out)
                .map_err(|e| format!("ошибка записи {}: {e}", dest.display()))?;
            if !installed.contains(&bundle) {
                installed.push(bundle);
            }
        }
    }
    if installed.is_empty() {
        return Err("в архиве нет .kext-бандлов".into());
    }
    Ok(installed)
}

/// Основной сценарий: скачать выбранные кексты и распаковать в EFI/OC/Kexts.
/// `progress` вызывается после завершения обработки каждого кекта.
pub fn download_kexts(
    items: &[KextDownloadItem],
    selected_dir: &Path,
    darwin: u32,
    mut progress: impl FnMut(DownloadProgress),
) -> Result<KextsToEfiReport, String> {
    let efi_base = resolve_efi_base(selected_dir);
    let (backup_path, audit_warnings) = efi_safety(&efi_base);
    let kexts_dir = efi_base.join("OC").join("Kexts");
    std::fs::create_dir_all(&kexts_dir).map_err(|e| {
        format!(
            "не создать {} — папка может быть защищена от записи: {e}",
            kexts_dir.display()
        )
    })?;

    let agent = http_agent();
    let macos = macos_name(darwin);
    let total = items.len();
    let mut results: Vec<KextDownloadResult> = Vec::new();
    let mut done = 0usize;

    // группировка по репозиторию: один запрос к API на репозиторий
    let mut repos: Vec<(String, Vec<String>)> = Vec::new(); // (repo, [kext names])
    for it in items {
        let Some(repo) = it.repo.clone().filter(|r| r.split('/').count() == 2) else {
            // UTBMap ставится через помощник USB, системные драйверы — в macOS
            let hint = match it.name.as_str() {
                "UTBMap" => "маппинг добавляется через помощник USB на этом же шаге".to_string(),
                n if n == "NVIDIA Driver" => "системный драйвер — устанавливается внутри macOS, не в EFI".to_string(),
                _ => "репозиторий не указан — скачайте вручную с сайта разработчика".to_string(),
            };
            results.push(KextDownloadResult {
                name: it.name.clone(),
                ok: false,
                version: None,
                installed: vec![],
                message: Some(hint),
            });
            done += 1;
            progress(DownloadProgress { done, total, kext: it.name.clone() });
            continue;
        };
        match repos.iter_mut().find(|(r, _)| *r == repo) {
            Some((_, names)) => names.push(it.name.clone()),
            None => repos.push((repo, vec![it.name.clone()])),
        }
    }

    for (repo, kext_names) in repos {
        let release = match fetch_latest_release(&agent, &repo) {
            Ok(r) => r,
            Err(e) => {
                for name in &kext_names {
                    results.push(KextDownloadResult {
                        name: name.clone(),
                        ok: false,
                        version: None,
                        installed: vec![],
                        message: Some(e.clone()),
                    });
                    done += 1;
                    progress(DownloadProgress { done, total, kext: name.clone() });
                }
                continue;
            }
        };
        let repo_name = repo.rsplit('/').next().unwrap_or("");
        let mut group_asset: Option<&Asset> = None;

        for name in &kext_names {
            let chosen = pick_asset(&release.assets, name, repo_name, macos).or(group_asset);
            let Some(asset) = chosen else {
                results.push(KextDownloadResult {
                    name: name.clone(),
                    ok: false,
                    version: None,
                    installed: vec![],
                    message: Some(format!(
                        "в релизе {repo} не найден подходящий архив — скачайте вручную"
                    )),
                });
                done += 1;
                progress(DownloadProgress { done, total, kext: name.clone() });
                continue;
            };
            let bytes = match fetch_bytes(&agent, &asset.browser_download_url) {
                Ok(b) => b,
                Err(e) => {
                    results.push(KextDownloadResult {
                        name: name.clone(),
                        ok: false,
                        version: None,
                        installed: vec![],
                        message: Some(e),
                    });
                    done += 1;
                    progress(DownloadProgress { done, total, kext: name.clone() });
                    continue;
                }
            };
            let installed = match extract_kexts_from_zip(std::io::Cursor::new(&bytes), &kexts_dir) {
                Ok(v) => v,
                Err(e) => {
                    results.push(KextDownloadResult {
                        name: name.clone(),
                        ok: false,
                        version: None,
                        installed: vec![],
                        message: Some(e),
                    });
                    done += 1;
                    progress(DownloadProgress { done, total, kext: name.clone() });
                    continue;
                }
            };
            if group_asset.is_none() {
                group_asset = Some(asset);
            }
            results.push(KextDownloadResult {
                name: name.clone(),
                ok: true,
                version: version_from_asset(&asset.name, name),
                installed,
                message: None,
            });
            done += 1;
            progress(DownloadProgress { done, total, kext: name.clone() });
        }
    }

    Ok(KextsToEfiReport {
        kexts_dir: kexts_dir.to_string_lossy().to_string(),
        results,
        backup_path,
        audit_warnings,
    })
}

/// SSDT для скачивания (из матрицы ssdts.rs)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SsdtDownloadItem {
    pub name: String,
    /// Имя на диске (EFI/OC/ACPI) и в ACPI/Add/Path
    pub file: String,
    /// Имя файла в extra-files/compiled репозитория Dortania
    pub repo_file: String,
}

/// Итоговый отчёт установки SSDT в EFI
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SsdtsToEfiReport {
    /// Каталог, куда легли таблицы (например «/Volumes/EFI/EFI/OC/ACPI»)
    pub acpi_dir: String,
    pub results: Vec<KextDownloadResult>,
    /// Путь к отредактированному config.plist, если он был найден
    pub config_path: Option<String>,
    /// Сохранили ли резервную копию конфига
    pub backup_made: bool,
    /// Сколько таблиц включено/добавлено в ACPI/Add
    pub registered: usize,
    /// Куда сделан автоматический бэкап EFI/OC перед изменениями
    #[serde(default)]
    pub backup_path: Option<String>,
    /// Предупреждения защиты EFI (нет OpenCore, не похоже на ESP и т.п.)
    #[serde(default)]
    pub audit_warnings: Vec<String>,
}

/// Перед любыми записями в EFI: предупреждения аудита + полный бэкап EFI/OC,
/// если OpenCore уже установлен. Никогда не завершаемся ошибкой из-за бэкапа —
/// только добавляем предупреждение в отчёт.
fn efi_safety(efi_base: &Path) -> (Option<String>, Vec<String>) {
    let mut warnings = crate::efiprotect::audit_warnings(efi_base);
    let mut backup_path = None;
    if efi_base.join("OC").is_dir() {
        match crate::efiprotect::backup_oc(efi_base) {
            Ok(rep) => backup_path = Some(rep.backup_path),
            Err(e) => warnings.insert(0, format!("бэкап EFI/OC не сделан: {e}")),
        }
    }
    (backup_path, warnings)
}

/// Включить существующие и добавить недостающие записи ACPI/Add
/// для скачанных .aml. Возвращает (сколько таблиц, сделан ли бэкап).
fn patch_acpi_add(config: &Path, files: &[String]) -> Result<(usize, bool), String> {
    let mut root = Value::from_reader(std::io::BufReader::new(
        std::fs::File::open(config).map_err(|e| format!("не открыть {}: {e}", config.display()))?,
    ))
    .map_err(|e| format!("не разобрать {}: {e}", config.display()))?
    .into_dictionary()
    .ok_or_else(|| format!("{} — не словарь plist", config.display()))?;

    let acpi = root
        .get_mut("ACPI")
        .and_then(|v| v.as_dictionary_mut())
        .ok_or("в config.plist нет секции ACPI")?;
    let add = match acpi.get_mut("Add") {
        Some(v) if matches!(v, Value::Array(_)) => v,
        Some(_) => return Err("ACPI → Add в config.plist — не массив".into()),
        None => {
            acpi.insert("Add".into(), Value::Array(Vec::new()));
            acpi.get_mut("Add").ok_or("не создать ACPI → Add")?
        }
    };
    let entries = add.as_array_mut().unwrap();

    let mut registered = 0usize;
    for file in files {
        let file_lc = file.to_lowercase();
        // запись может лежать уже в конфиге (в т.ч. с подпапкой «ACPI/SSDT-….aml»)
        let mut found = false;
        for e in entries.iter_mut() {
            let Some(d) = e.as_dictionary_mut() else { continue };
            let matches = d
                .get("Path")
                .and_then(|p| p.as_string())
                .map(|p| {
                    let p = p.replace('\\', "/").to_lowercase();
                    p == file_lc || p.ends_with(&format!("/{file_lc}"))
                })
                .unwrap_or(false);
            if matches {
                d.insert("Enabled".into(), Value::Boolean(true));
                registered += 1;
                found = true;
                break;
            }
        }
        if !found {
            let mut d = Dictionary::new();
            d.insert(
                "Comment".into(),
                Value::String(file.trim_end_matches(".aml").to_string()),
            );
            d.insert("Enabled".into(), Value::Boolean(true));
            d.insert("Path".into(), Value::String(file.clone()));
            entries.push(Value::Dictionary(d));
            registered += 1;
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
    Ok((registered, true))
}

/// Основной сценарий: скачать выбранные SSDT (.aml) из репозитория Dortania
/// в EFI/OC/ACPI. Если рядом есть config.plist — таблицы включаются в ACPI/Add
/// (с резервной копией конфига).
pub fn download_ssdts(
    items: &[SsdtDownloadItem],
    selected_dir: &Path,
    mut progress: impl FnMut(DownloadProgress),
) -> Result<SsdtsToEfiReport, String> {
    let efi_base = resolve_efi_base(selected_dir);
    let (backup_path, audit_warnings) = efi_safety(&efi_base);
    let acpi_dir = efi_base.join("OC").join("ACPI");
    std::fs::create_dir_all(&acpi_dir).map_err(|e| {
        format!(
            "не создать {} — папка может быть защищена от записи: {e}",
            acpi_dir.display()
        )
    })?;

    let agent = http_agent();
    let total = items.len();
    let mut results: Vec<KextDownloadResult> = Vec::new();
    let mut done = 0usize;

    for it in items {
        let res = if it.repo_file.is_empty() {
            KextDownloadResult {
                name: it.name.clone(),
                ok: false,
                version: None,
                installed: vec![],
                message: Some(
                    "прекомпилированного файла нет в репозитории Dortania — соберите вручную по гайду"
                        .into(),
                ),
            }
        } else {
            let url = crate::ssdts::raw_url(&it.repo_file);
            let dest = acpi_dir.join(&it.file);
            match download_to_file(&agent, &url, &dest) {
                Ok(_) => KextDownloadResult {
                    name: it.name.clone(),
                    ok: true,
                    version: None,
                    installed: vec![it.file.clone()],
                    message: None,
                },
                Err(e) => KextDownloadResult {
                    name: it.name.clone(),
                    ok: false,
                    version: None,
                    installed: vec![],
                    message: Some(e),
                },
            }
        };
        done += 1;
        progress(DownloadProgress { done, total, kext: it.name.clone() });
        results.push(res);
    }

    let ok_files: Vec<String> = results
        .iter()
        .filter(|r| r.ok)
        .filter_map(|r| r.installed.first().cloned())
        .collect();

    // регистрация в config.plist
    let mut config_path = None;
    let mut backup_made = false;
    let mut registered = 0usize;
    if !ok_files.is_empty() {
        let config = efi_base.join("OC").join("config.plist");
        if config.is_file() {
            let (n, backup) = patch_acpi_add(&config, &ok_files)?;
            registered = n;
            backup_made = backup;
            config_path = Some(config.to_string_lossy().to_string());
        }
    }

    Ok(SsdtsToEfiReport {
        acpi_dir: acpi_dir.to_string_lossy().to_string(),
        results,
        config_path,
        backup_made,
        registered,
        backup_path,
        audit_warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> Asset {
        Asset { name: name.into(), browser_download_url: format!("https://x/{name}") }
    }

    #[test]
    fn test_pick_asset_release_over_debug() {
        let assets = vec![
            asset("Lilu-1.7.0-DEBUG.zip"),
            asset("Lilu-1.7.0-RELEASE.zip"),
        ];
        let picked = pick_asset(&assets, "Lilu", "Lilu", None).unwrap();
        assert!(picked.name.contains("RELEASE"));
    }

    #[test]
    fn test_pick_asset_ignores_non_zip() {
        let assets = vec![
            asset("docs.pdf"),
            asset("Lilu-1.7.0-RELEASE.zip"),
        ];
        let picked = pick_asset(&assets, "Lilu", "Lilu", None).unwrap();
        assert_eq!(picked.name, "Lilu-1.7.0-RELEASE.zip");
    }

    #[test]
    fn test_pick_asset_itlwm_vs_airport() {
        let assets = vec![
            asset("itlwm_v2.6.1_stable.kext.zip"),
            asset("AirportItlwm_v2.6.1_stable_Sonoma.kext.zip"),
            asset("AirportItlwm_v2.6.1_stable_Ventura.kext.zip"),
        ];
        // itlwm: ассет, начинающийся с имени кекта, должен победить
        let itl = pick_asset(&assets, "itlwm", "itlwm", Some("Sonoma")).unwrap();
        assert!(itl.name.starts_with("itlwm_"));
        // AirportItlwm: под Sonoma — сонома-вариант
        let ap = pick_asset(&assets, "AirportItlwm", "itlwm", Some("Sonoma")).unwrap();
        assert!(ap.name.contains("Sonoma"));
        // без подсказки — первый по скорингу (оба AirportItlwm равны, берём лучший стартовый)
        let ap2 = pick_asset(&assets, "AirportItlwm", "itlwm", None).unwrap();
        assert!(ap2.name.contains("AirportItlwm"));
    }

    #[test]
    fn test_pick_asset_macos_variant_penalty() {
        let assets = vec![
            asset("WhateverGreen-1.6.9-RELEASE.zip"),
            asset("AirportItlwm_Ventura.zip"),
        ];
        let picked = pick_asset(&assets, "WhateverGreen", "WhateverGreen", Some("Sonoma")).unwrap();
        assert!(picked.name.contains("WhateverGreen"));
    }

    #[test]
    fn test_version_from_asset() {
        assert_eq!(version_from_asset("Lilu-1.7.0-RELEASE.zip", "Lilu").as_deref(), Some("1.7.0"));
        assert_eq!(version_from_asset("itlwm_v2.6.1_stable.kext.zip", "itlwm").as_deref(), Some("2.6.1"));
        assert_eq!(version_from_asset("VirtualSMC-1.3.4-RELEASE.zip", "SMCProcessor").is_none(), true);
    }

    #[test]
    fn test_resolve_efi_base() {
        let tmp = std::env::temp_dir().join(format!("openefi-dl-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let volume = tmp.join("volume");
        let efi = volume.join("EFI");
        std::fs::create_dir_all(&efi).unwrap();

        // выбрали корень тома с EFI внутри
        assert_eq!(resolve_efi_base(&volume), efi);
        // выбрали саму EFI
        assert_eq!(resolve_efi_base(&efi), efi);
        // выбрали OC
        let oc = efi.join("OC");
        std::fs::create_dir_all(&oc).unwrap();
        assert_eq!(resolve_efi_base(&oc), efi);
        // пустая папка — считаем её EFI
        let empty = tmp.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert_eq!(resolve_efi_base(&empty), empty);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_extract_kexts_from_zip() {
        use zip::write::SimpleFileOptions;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            let opts = SimpleFileOptions::default();
            w.add_directory("Release/", opts).unwrap();
            w.start_file("Release/Lilu.kext/Contents/Info.plist", opts).unwrap();
            std::io::Write::write_all(&mut w, b"<plist>lilu</plist>").unwrap();
            w.start_file("Release/Lilu.kext/Contents/MacOS/Lilu", opts).unwrap();
            std::io::Write::write_all(&mut w, b"\xde\xad\xbe\xef").unwrap();
            w.start_file("docs/readme.txt", opts).unwrap();
            std::io::Write::write_all(&mut w, b"readme").unwrap();
            w.finish().unwrap();
        }
        buf.set_position(0);

        let tmp = std::env::temp_dir().join(format!("openefi-zip-test-{}", std::process::id()));
        let kexts_dir = tmp.join("Kexts");
        std::fs::create_dir_all(&kexts_dir).unwrap();
        // имитируем старую установку — должна быть заменена
        std::fs::create_dir_all(kexts_dir.join("Lilu.kext/old")).unwrap();

        let installed = extract_kexts_from_zip(buf, &kexts_dir).unwrap();
        assert_eq!(installed, vec!["Lilu.kext"]);
        assert!(kexts_dir.join("Lilu.kext/Contents/Info.plist").exists());
        assert!(kexts_dir.join("Lilu.kext/Contents/MacOS/Lilu").exists());
        assert!(!kexts_dir.join("Lilu.kext/old").exists(), "старое содержимое должно быть удалено");
        assert!(!kexts_dir.join("docs").exists(), "мусор вне бандлов не копируется");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_extract_rejects_zip_without_kext() {
        use zip::write::SimpleFileOptions;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            w.start_file("just/a/file.txt", SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut w, b"x").unwrap();
            w.finish().unwrap();
        }
        buf.set_position(0);
        let tmp = std::env::temp_dir().join(format!("openefi-zip-bad-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(extract_kexts_from_zip(buf, &tmp).is_err());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_utb_tool_asset_mapping() {
        let (repo_w, tag_w, asset_w) = utb_tool_asset("windows").unwrap();
        assert_eq!((repo_w, asset_w), ("USBToolBox/tool", "Windows.exe"));
        assert!(tag_w.is_none());
        let (repo_m, tag_m, asset_m) = utb_tool_asset("macos").unwrap();
        assert_eq!((repo_m, asset_m), ("USBToolBox/tool", "macOS.zip"));
        assert_eq!(tag_m, Some("0.1.1"));
        assert!(utb_tool_asset("linux").is_err());
    }

    #[test]
    fn test_macos_name() {
        assert_eq!(macos_name(22), Some("Ventura"));
        assert_eq!(macos_name(25), Some("Tahoe"));
        assert_eq!(macos_name(18), None);
    }

    #[test]
    #[ignore] // прогон вручную: нужны скачанные архивы в /tmp (см. worklog)
    fn test_extract_real_zips_if_present() {
        let cases = [
            ("/tmp/lilu.zip", "Lilu.kext"),
            ("/tmp/utb.zip", "UTBDefault.kext"),
            ("/tmp/vsmc.zip", "SMCProcessor.kext"),
        ];
        for (file, expected) in cases {
            let Ok(bytes) = std::fs::read(file) else { continue }; // файла нет — пропускаем
            let tmp = std::env::temp_dir().join(format!("openefi-real-{}", std::process::id()));
            std::fs::create_dir_all(&tmp).unwrap();
            let installed = extract_kexts_from_zip(std::io::Cursor::new(&bytes), &tmp).unwrap();
            assert!(
                installed.iter().any(|b| b == expected),
                "{file}: ожидался {expected}, получено {installed:?}"
            );
            assert!(
                !installed.iter().any(|b| b.ends_with(".dSYM")),
                "dSYM не должен ставиться: {installed:?}"
            );
            assert!(tmp.join(expected).join("Contents").exists());
            std::fs::remove_dir_all(&tmp).ok();
        }
    }

    #[test]
    #[ignore] // сетевой тест: запускается вручную
    fn test_network_fetch_lilu_release() {
        let agent = http_agent();
        let rel = fetch_latest_release(&agent, "acidanthera/Lilu").unwrap();
        let picked = pick_asset(&rel.assets, "Lilu", "Lilu", None).unwrap();
        assert!(picked.name.contains("Lilu"));
        let bytes = fetch_bytes(&agent, &picked.browser_download_url).unwrap();
        assert!(bytes.len() > 10_000);
        let tmp = std::env::temp_dir().join(format!("openefi-net-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let installed = extract_kexts_from_zip(std::io::Cursor::new(&bytes), &tmp).unwrap();
        assert!(installed.contains(&"Lilu.kext".to_string()));
        std::fs::remove_dir_all(&tmp).ok();
    }

    fn config_with_plug_disabled() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
  <key>ACPI</key>
  <dict>
    <key>Add</key>
    <array>
      <dict>
        <key>Comment</key><string>SSDT-PLUG-DRTNIA</string>
        <key>Enabled</key><false/>
        <key>Path</key><string>SSDT-PLUG-DRTNIA.aml</string>
      </dict>
      <dict>
        <key>Comment</key><string>чужая таблица</string>
        <key>Enabled</key><true/>
        <key>Path</key><string>SSDT-OTHER.aml</string>
      </dict>
    </array>
  </dict>
  <key>Kernel</key>
  <dict><key>Add</key><array/></dict>
</dict>
</plist>"#
            .into()
    }

    #[test]
    fn test_patch_acpi_add_enable_and_append() {
        let tmp = std::env::temp_dir().join(format!("openefi-acpi1-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let oc = tmp.join("volume").join("EFI").join("OC");
        std::fs::create_dir_all(&oc).unwrap();
        let config = oc.join("config.plist");
        std::fs::write(&config, config_with_plug_disabled()).unwrap();

        let (registered, backup) =
            patch_acpi_add(&config, &["SSDT-PLUG-DRTNIA.aml".into(), "SSDT-AWAC.aml".into()]).unwrap();
        assert_eq!(registered, 2, "включена PLUG + добавлена AWAC");
        assert!(backup);
        assert!(oc.join("config.plist.bak-openefi").exists(), "нужна резервная копия");

        let root = Value::from_reader(std::fs::File::open(&config).unwrap())
            .unwrap()
            .into_dictionary()
            .unwrap();
        let arr = root
            .get("ACPI").and_then(|a| a.as_dictionary()).and_then(|a| a.get("Add"))
            .and_then(|a| a.as_array()).unwrap();
        let paths: Vec<(String, bool)> = arr
            .iter()
            .filter_map(|v| v.as_dictionary())
            .filter_map(|d| {
                let p = d.get("Path")?.as_string()?.to_string();
                let e = d.get("Enabled")?.as_boolean()?;
                Some((p, e))
            })
            .collect();
        assert_eq!(paths.len(), 3, "чужая таблица не тронута");
        assert!(paths.contains(&("SSDT-PLUG-DRTNIA.aml".to_string(), true)));
        assert!(paths.contains(&("SSDT-AWAC.aml".to_string(), true)));
        assert!(paths.contains(&("SSDT-OTHER.aml".to_string(), true)));
        // Kernel-секция не изменилась
        assert!(root.get("Kernel").is_some());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_patch_acpi_add_subdir_path_matched() {
        let tmp = std::env::temp_dir().join(format!("openefi-acpi2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let oc = tmp.join("EFI").join("OC");
        std::fs::create_dir_all(&oc).unwrap();
        let config = oc.join("config.plist");
        // путь с подпапкой и другим регистром — всё равно должен совпасть
        std::fs::write(
            &config,
            config_with_plug_disabled().replace("SSDT-PLUG-DRTNIA.aml</string>", "ACPI/SSDT-PLUG-DRTNIA.aml</string>").replace("<false/>", "<true/>"),
        )
        .unwrap();

        let (registered, _) = patch_acpi_add(&config, &["SSDT-PLUG-DRTNIA.aml".into()]).unwrap();
        assert_eq!(registered, 1, "найдена запись с подпапкой, дубликат не добавлен");
        let root = Value::from_reader(std::fs::File::open(&config).unwrap())
            .unwrap()
            .into_dictionary()
            .unwrap();
        let arr = root
            .get("ACPI").and_then(|a| a.as_dictionary()).and_then(|a| a.get("Add"))
            .and_then(|a| a.as_array()).unwrap();
        assert_eq!(arr.len(), 2, "новая запись не добавлена");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_patch_acpi_add_no_acpi_section() {
        let tmp = std::env::temp_dir().join(format!("openefi-acpi3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let oc = tmp.join("EFI").join("OC");
        std::fs::create_dir_all(&oc).unwrap();
        let config = oc.join("config.plist");
        std::fs::write(
            &config,
            r#"<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>Kernel</key><dict/></dict></plist>"#,
        )
        .unwrap();
        let err = patch_acpi_add(&config, &["SSDT-AWAC.aml".into()]).unwrap_err();
        assert!(err.contains("ACPI"), "внятная ошибка: {err}");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_download_ssdts_reports_manual_only() {
        // элемент без repo_file: папка создаётся, результата ok нет, конфиг не трогается
        let tmp = std::env::temp_dir().join(format!("openefi-acpi4-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let volume = tmp.join("volume");
        std::fs::create_dir_all(volume.join("EFI").join("OC")).unwrap();

        let items = vec![SsdtDownloadItem {
            name: "SSDT-DMAR".into(),
            file: "SSDT-DMAR.aml".into(),
            repo_file: String::new(),
        }];
        let report = download_ssdts(&items, &volume, |_| {}).unwrap();
        assert_eq!(report.results.len(), 1);
        assert!(!report.results[0].ok);
        assert!(report.results[0].message.as_deref().unwrap_or_default().contains("Dortania"));
        assert!(report.config_path.is_none());
        assert_eq!(report.registered, 0);
        assert!(volume.join("EFI").join("OC").join("ACPI").is_dir());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    #[ignore] // сетевой тест: запускается вручную
    fn test_network_fetch_ssdt_raw() {
        let tmp = std::env::temp_dir().join(format!("openefi-net-ssdt-{}", std::process::id()));
        let volume = tmp.join("volume");
        std::fs::create_dir_all(volume.join("EFI")).unwrap(); // чтобы resolve_efi_base нашёл EFI
        let items = vec![
            SsdtDownloadItem {
                name: "SSDT-PLUG-DRTNIA".into(),
                file: "SSDT-PLUG-DRTNIA.aml".into(),
                repo_file: "SSDT-PLUG-DRTNIA.aml".into(),
            },
            SsdtDownloadItem {
                name: "SSDT-AWAC-DRTNIA".into(),
                file: "SSDT-AWAC.aml".into(),
                repo_file: "SSDT-AWAC.aml".into(),
            },
        ];
        let report = download_ssdts(&items, &volume, |_| {}).unwrap();
        assert_eq!(report.results.iter().filter(|r| r.ok).count(), 2);
        let plug = volume.join("EFI").join("OC").join("ACPI").join("SSDT-PLUG-DRTNIA.aml");
        let bytes = std::fs::read(&plug).unwrap();
        assert!(bytes.starts_with(b"SSDT"), "AML должен начинаться с сигнатуры SSDT");
        assert!(bytes.len() > 100);
        let awac = volume.join("EFI").join("OC").join("ACPI").join("SSDT-AWAC.aml");
        assert!(std::fs::read(&awac).unwrap().starts_with(b"SSDT"));
        std::fs::remove_dir_all(&tmp).ok();
    }
}
