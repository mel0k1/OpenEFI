use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::db::{mac_models, MacModelRaw};
use crate::macos::parse_darwin;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MacDevice {
    pub name: String,
    pub cpu: String,
    #[serde(rename = "gen")]
    pub cpu_gen: String,
    pub dgpu: Option<String>,
    pub initial: String,
    pub last: Option<String>,
}

pub fn mac_devices() -> &'static Vec<MacDevice> {
    static DEVICES: std::sync::OnceLock<Vec<MacDevice>> = std::sync::OnceLock::new();
    DEVICES.get_or_init(|| {
        serde_json::from_str(include_str!("mac_devices.json")).expect("parse mac_devices.json")
    })
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SmbiosRecommendation {
    pub recommended: String,
    pub reason: String,
}

pub fn recommend(report: &crate::hardware::HardwareReport, darwin: u32) -> SmbiosRecommendation {
    let codename = report.cpu.codename.clone().unwrap_or_default();
    let platform = if report.board_name.to_uppercase().contains("NUC") {
        "NUC"
    } else if report.platform.contains("Laptop") {
        "Laptop"
    } else {
        "Desktop"
    };
    let cores = report.cpu.cores.max(1) as u32;
    let cpu_name = &report.cpu.name;
    let is_amd = report.cpu.manufacturer == "AMD";

    let mut model = if platform == "Laptop" {
        "MacBookPro16,2".to_string()
    } else if darwin < 25 {
        "iMacPro1,1".to_string()
    } else {
        "MacPro7,1".to_string()
    };
    let mut reason = "Универсальный выбор по умолчанию".to_string();

    if is_amd {
        model = if darwin >= 25 { "MacPro7,1".into() } else { "iMacPro1,1".into() };
        reason = format!("AMD {}: серверная SMBIOS", codename);
        if platform == "Laptop" {
            model = "MacBookPro16,2".into();
            reason = format!("AMD {}: ноутбучная SMBIOS", codename);
        }
        return SmbiosRecommendation { recommended: model, reason };
    }

    let gen_matches = |names: &[&str]| names.iter().any(|n| codename.contains(n));

    if gen_matches(&["Lynnfield"]) && !cpu_name.contains("Xeon") && darwin < 19 {
        model = "iMac11,1".into();
        reason = "Lynnfield + дискретная графика".into();
    } else if gen_matches(&["Clarkdale"]) && !cpu_name.contains("Xeon") && darwin < 19 && platform != "Laptop" {
        model = "iMac11,2".into();
        reason = "Clarkdale + iGPU".into();
    } else if gen_matches(&[
        "Beckton", "Westmere-EX", "Gulftown", "Westmere-EP", "Clarkdale", "Lynnfield",
        "Jasper Forest", "Gainestown", "Bloomfield",
    ]) {
        model = if darwin < 19 { "MacPro5,1".into() } else { "MacPro6,1".into() };
        reason = "Серверный Xeon Westmere/Lynnfield".into();
    } else if gen_matches(&["Sandy Bridge", "Ivy Bridge"]) && darwin < 22 {
        model = "MacPro6,1".into();
        reason = "Sandy/Ivy Bridge: MacPro6,1 с dGPU".into();
    }

    let last_gpu_is_igpu = report
        .gpus
        .last()
        .map(|g| g.device_type == "Integrated GPU")
        .unwrap_or(true);
    let first_gpu_is_igpu = report
        .gpus
        .first()
        .map(|g| g.device_type == "Integrated GPU")
        .unwrap_or(false);

    if platform != "Laptop" && !last_gpu_is_igpu {
        return SmbiosRecommendation { recommended: model, reason };
    }

    if gen_matches(&["Arrandale", "Clarksfield"]) {
        model = "MacBookPro6,1".into();
        reason = "Arrandale/Clarksfield ноутбук".into();
    } else if gen_matches(&["Sandy Bridge"]) {
        model = match platform {
            "Desktop" => {
                reason = "Sandy Bridge настольный".into();
                "iMac12,2"
            }
            "NUC" => {
                reason = "Sandy Bridge NUC".into();
                if cores < 4 { "Macmini5,1" } else { "Macmini5,3" }
            }
            _ => {
                reason = "Sandy Bridge ноутбук".into();
                if cores < 4 { "MacBookPro8,1" } else { "MacBookPro8,2" }
            }
        }
        .to_string();
    } else if gen_matches(&["Ivy Bridge"]) {
        model = match platform {
            "Desktop" => {
                reason = "Ivy Bridge настольный".into();
                if first_gpu_is_igpu { "iMac13,1" } else { "iMac13,2" }
            }
            "NUC" => {
                reason = "Ivy Bridge NUC".into();
                if cores < 4 { "Macmini6,1" } else { "Macmini6,2" }
            }
            _ => {
                reason = "Ivy Bridge ноутбук".into();
                if cores < 4 { "MacBookPro10,2" } else { "MacBookPro10,1" }
            }
        }
        .to_string();
    } else if gen_matches(&["Haswell"]) {
        model = match platform {
            "Desktop" => {
                reason = "Haswell настольный".into();
                if first_gpu_is_igpu { "iMac14,4" } else { "iMac15,1" }
            }
            "NUC" => {
                reason = "Haswell NUC".into();
                "Macmini7,1"
            }
            _ => {
                reason = "Haswell ноутбук".into();
                if cores < 4 { "MacBookPro11,1" } else { "MacBookPro11,5" }
            }
        }
        .to_string();
    } else if gen_matches(&["Broadwell"]) {
        model = match platform {
            "Desktop" => {
                reason = "Broadwell настольный".into();
                if first_gpu_is_igpu { "iMac16,2" } else { "iMac17,1" }
            }
            "NUC" => {
                reason = "Broadwell NUC".into();
                "iMac16,1"
            }
            _ => {
                reason = "Broadwell ноутбук".into();
                if cores < 4 { "MacBookPro12,1" } else { "MacBookPro11,5" }
            }
        }
        .to_string();
    } else if gen_matches(&["Skylake"]) {
        model = if platform == "Laptop" {
            reason = "Skylake ноутбук".into();
            { if cores < 4 { "MacBookPro13,1" } else { "MacBookPro13,3" } }.to_string()
        } else {
            reason = "Skylake + iGPU".into();
            "iMac17,1".to_string()
        };
    } else if gen_matches(&["Amber Lake", "Kaby Lake"]) {
        model = if platform == "Laptop" {
            reason = "Kaby Lake ноутбук".into();
            { if cores < 4 { "MacBookPro14,1" } else { "MacBookPro14,3" } }.to_string()
        } else if first_gpu_is_igpu {
            reason = "Kaby Lake + iGPU".into();
            "iMac18,1".to_string()
        } else {
            reason = "Kaby Lake настольный".into();
            "iMac18,3".to_string()
        };
    } else if gen_matches(&[
        "Cannon Lake", "Whiskey Lake", "Coffee Lake", "Comet Lake",
    ]) {
        model = if platform == "Laptop" {
            if cpu_name.contains("-8") {
                reason = "Coffee Lake 8xxx ноутбук".into();
                { if cores < 6 { "MacBookPro15,2" } else { "MacBookPro15,3" } }.to_string()
            } else {
                reason = "Whiskey/Coffee Lake ноутбук".into();
                { if cores < 6 { "MacBookPro16,3" } else { "MacBookPro16,1" } }.to_string()
            }
        } else if platform == "NUC" {
            reason = "Coffee Lake NUC".into();
            "Macmini8,1".to_string()
        } else {
            if gen_matches(&["Comet Lake"]) {
                reason = "Comet Lake настольный".into();
                { if cores < 10 { "iMac20,1" } else { "iMac20,2" } }.to_string()
            } else {
                reason = "Coffee Lake настольный + iGPU".into();
                "iMac19,1".to_string()
            }
        };
    } else if gen_matches(&["Ice Lake"]) {
        model = "MacBookAir9,1".into();
        reason = "Ice Lake ноутбук".into();
    } else if gen_matches(&["Tiger Lake"]) {
        model = if platform == "Laptop" {
            reason = "Tiger Lake ноутбук".into();
            "MacBookPro16,2".to_string()
        } else {
            reason = "Tiger Lake".into();
            "iMac20,1".to_string()
        };
    } else if gen_matches(&["Rocket Lake"]) {
        model = "iMac20,1".into();
        reason = "Rocket Lake настольный + iGPU".into();
    } else if gen_matches(&["Alder Lake", "Raptor Lake"]) {
        if platform == "Laptop" {
            model = "MacBookPro16,2".into();
            reason = format!("{} ноутбук: Apple Silicon-подобная SMBIOS", codename);
        } else {
            model = "iMacPro1,1".into();
            reason = format!("{}: SMBIOS с поддержкой гибридной графики", codename);
        }
    } else if gen_matches(&["Meteor Lake", "Lunar Lake", "Arrow Lake"]) {
        model = "MacBookPro16,2".into();
        reason = format!("{}: новейшие iGPU требуют свежих SMBIOS", codename);
    }

    SmbiosRecommendation { recommended: model, reason }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SmbiosData {
    pub model: String,
    pub serial: String,
    pub mlb: String,
    pub rom: String,
    pub uuid: String,
}

const BASE34: &[u8] = b"0123456789ABCDEFGHJKLMNPQRSTUVWXYZ";
const YEAR_REVERSE: &[u8] = b"CDFGHJKLMNPQRSTVWXYZ";
const WEEK_REVERSE: &[u8] = b"0123456789CDFGHJKLMNPQRTVWX123456789CDFGHJKLMNPQRTVWXY";
const MLB_BLOCK1: &[&str] = &[
    "200", "600", "403", "404", "405", "303", "108", "207", "609", "501", "306", "102", "701",
    "301", "501", "101", "300", "130", "100", "270", "310",
];
const MLB_BLOCK2: &[&str] = &["GU", "4N", "J9", "QX", "OP", "CD", "GU"];
const MLB_BLOCK3: &[&str] = &[
    "1H", "1M", "AD", "1F", "A8", "UE", "JA", "JC", "8C", "CB", "FB",
];

pub fn generate(model: &str) -> SmbiosData {
    let models = mac_models();
    let raw: Option<&MacModelRaw> = models.get(model);
    let (country, code, board, years): (String, String, String, Vec<u32>) = match raw {
        Some(r) => {
            let country_len = if r.code.len() == 4 { 3 } else { 2 };
            (
                r.prefix.chars().take(country_len).collect(),
                r.code.clone(),
                r.board.clone(),
                r.years.clone(),
            )
        }
        None => ("C02".to_string(), String::new(), String::new(), vec![2021]),
    };

    let mut rng = rand::rng();
    let year: u32 = if years.is_empty() {
        2021
    } else {
        years[rng.random_range(0..years.len())]
    };
    let week: u32 = rng.random_range(1..=52);
    let line: u32 = rng.random_range(0..=3399);

    let serial = assemble_serial(&country, year, week, line, &code);
    let mlb = generate_mlb(&serial, &country, &code, &board);
    let rom: String = (0..6)
        .map(|_| format!("{:02X}", rng.random::<u8>()))
        .collect();
    let uuid = uuid::Uuid::new_v4().to_string().to_uppercase();

    SmbiosData {
        model: model.to_string(),
        serial,
        mlb,
        rom,
        uuid,
    }
}

fn assemble_serial(country: &str, year: u32, week: u32, line: u32, code: &str) -> String {
    let modern = code.len() == 4;
    let mut serial = String::from(country);
    if modern {
        let base = if year >= 2020 { 2020 } else { 2010 };
        let idx = ((year - base) * 2 + u32::from(week >= 27)) as usize;
        serial.push(YEAR_REVERSE[idx.min(19)] as char);
        serial.push(WEEK_REVERSE[week.min(53) as usize] as char);
    } else {
        serial.push((b'0' + ((year - 2000) % 10) as u8) as char);
        serial.push_str(&format!("{:02}", week));
    }

    let rmin: u32 = if line > 1155 { (line - 1155 + 67) / 68 } else { 0 };
    serial.push(BASE34[rmin as usize % 34] as char);
    serial.push(BASE34[((line - rmin * 68) / 34) as usize] as char);
    serial.push(BASE34[((line - rmin * 68) % 34) as usize] as char);
    serial.push_str(code);
    serial
}

fn generate_mlb(serial: &str, country: &str, code: &str, board: &str) -> String {
    let modern = code.len() == 4;
    let mut rng = rand::rng();
    let mut mlb;

    if !modern {
        let year_char = serial.as_bytes()[country.len()];
        let week = serial
            .get(country.len() + 1..country.len() + 3)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);
        let year = (year_char as char).to_digit(10).unwrap_or(3);
        loop {
            let seed = rng.random_range(0..0x7FFEu32) * 0x73BA1C;
            let ccc = ascii7(seed);
            let suffix = BASE34[rng.random_range(0..34)] as char;
            mlb = format!("{}{}{:02}0{}{}{}", country, year, week, ccc, board, suffix);
            if mlb_checksum_valid(&mlb) {
                break;
            }
        }
    } else {
        let year_char = serial.as_bytes()[country.len()] as char;
        let week_char = serial.as_bytes()[country.len() + 1] as char;
        let src_year = b"CDFGHJKLMNPQRSTVWXYZ";
        let dst_year = b"00112233445566778899";
        let mut year_digit: Option<u32> = None;
        for (i, c) in src_year.iter().enumerate() {
            if *c as char == year_char {
                year_digit = Some((dst_year[i] - b'0') as u32);
                break;
            }
        }
        let mut week: i32 = 0;
        if "DGJLNQSVXZ".contains(year_char) {
            week = 27;
        }
        let src_week = b"123456789CDFGHJKLMNPQRSTVWXYZ";
        for (i, c) in src_week.iter().enumerate() {
            if *c as char == week_char {
                week += i as i32 + 1;
                break;
            }
        }
        if week < 1 {
            week = 1;
        }
        loop {
            let w = week - 1;
            let w_str = if w <= 9 {
                if w == 0 { 53 } else { w }.to_string()
            } else {
                w.to_string()
            };
            let p1 = MLB_BLOCK1[rng.random_range(0..MLB_BLOCK1.len())];
            let p2 = MLB_BLOCK2[rng.random_range(0..MLB_BLOCK2.len())];
            let p3 = MLB_BLOCK3[rng.random_range(0..MLB_BLOCK3.len())];
            mlb = format!(
                "{}{}{}{}{}{}{}",
                country,
                year_digit.unwrap_or(7),
                format!("{:0>2}", w_str),
                p1,
                p2,
                board,
                p3
            );
            if mlb_checksum_valid(&mlb) {
                break;
            }
        }
    }
    mlb
}

fn ascii7(mut value: u32) -> String {
    while value > 10_000_000 {
        value /= 10;
    }
    let mut out = String::new();
    loop {
        out.insert(0, BASE34[(value % 34) as usize] as char);
        value /= 34;
        if value == 0 {
            break;
        }
    }
    out
}

fn mlb_checksum_valid(mlb: &str) -> bool {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKLMNPQRSTUVWXYZ";
    let len = mlb.len();
    let mut checksum: u32 = 0;
    for (i, ch) in mlb.chars().enumerate() {
        match ALPHABET.iter().position(|a| *a as char == ch) {
            Some(j) => {
                let mult: u32 = if (i & 1) == (len & 1) { 3 } else { 1 };
                checksum += mult * j as u32;
            }
            None => return false,
        }
    }
    checksum % (ALPHABET.len() as u32) == 0
}

pub fn compatible_models(darwin: u32, is_laptop: bool) -> Vec<&'static MacDevice> {
    mac_devices()
        .iter()
        .filter(|d| {
            let last = d.last.as_deref().map(|l| parse_darwin(l)).unwrap_or(crate::macos::LATEST_DARWIN);
            let supported = parse_darwin(&d.initial) <= darwin && darwin <= last;
            let platform_ok = if is_laptop {
                d.name.starts_with("MacBook")
            } else {
                !d.name.starts_with("MacBook")
            };
            supported && platform_ok
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKLMNPQRSTUVWXYZ";

    fn serial_alphabet_ok(serial: &str) -> bool {
        serial
            .chars()
            .all(|c| ALPHABET.contains(&(c as u8)) && c != 'I' && c != 'O')
    }

    #[test]
    fn generated_serials_are_wellformed() {
        for model in ["iMac19,1", "MacPro7,1", "MacBookPro16,1", "iMacPro1,1", "Macmini8,1"] {
            for _ in 0..50 {
                let data = generate(model);
                assert_eq!(data.serial.len(), 12, "serial for {model}: {}", data.serial);
                assert!(serial_alphabet_ok(&data.serial));
                assert!(data.mlb.len() >= 17, "mlb: {}", data.mlb);
                assert!(mlb_checksum_valid(&data.mlb), "mlb checksum: {}", data.mlb);
                assert_eq!(data.rom.len(), 12);
                assert_eq!(data.uuid.len(), 36);
            }
        }
    }

    #[test]
    fn serial_encodes_chosen_year_and_week() {
        for _ in 0..100 {
            let data = generate("iMac19,1");
            let year_char = data.serial.as_bytes()[3] as char;
            // Y/Z = 2019, C/D = 2020; чётный индекс = первая половина, нечётный = вторая
            let year_idx = YEAR_REVERSE.iter().position(|c| *c as char == year_char).unwrap();
            assert!(
                matches!(year_idx, 18 | 19 | 0 | 1),
                "iMac19,1 serial {data:?} year char {year_char}"
            );
            let week_char = data.serial.as_bytes()[4] as char;
            let first = WEEK_REVERSE
                .iter()
                .position(|c| *c as char == week_char)
                .unwrap();
            let second = WEEK_REVERSE[first + 1..]
                .iter()
                .position(|c| *c as char == week_char)
                .map(|p| p + first + 1);
            let second_half = year_idx % 2 == 1;
            let week = if second_half { second.unwrap_or(first) } else { first };
            assert!((1..=52).contains(&week), "week {week} (char {week_char})");
            assert_eq!(
                second_half,
                week >= 27,
                "год {}/{} не согласован с неделей {}",
                year_char,
                year_idx,
                week
            );
        }
    }

    #[test]
    fn all_models_have_valid_prefix() {
        for (name, raw) in mac_models() {
            assert!(
                raw.prefix.len() == raw.code.len() + 8 || raw.prefix.len() == raw.code.len() + 6,
                "model {name} prefix {} code {}",
                raw.prefix,
                raw.code
            );
        }
    }

    #[test]
    fn recommend_reaches_all_branches() {
        let mut report = crate::hardware::HardwareReport {
            os: "linux".into(),
            platform: "Desktop".into(),
            board_name: "Test Board".into(),
            cpu: crate::hardware::CpuInfo {
                manufacturer: "Intel".into(),
                name: "Intel(R) Core(TM) i9-9900K CPU @ 3.60GHz".into(),
                codename: Some("Coffee Lake-S".into()),
                cores: 8,
                threads: 16,
                avx2: true,
            },
            memory_gb: 32.0,
            gpus: vec![],
            network: vec![],
            audio: vec![],
            bluetooth: vec![],
            storage: vec![],
            warnings: vec![],
        };
        let rec = recommend(&report, 24);
        assert_eq!(rec.recommended, "iMac19,1");

        report.platform = "Laptop".into();
        report.cpu.codename = Some("Tiger Lake-UP3".into());
        assert_eq!(recommend(&report, 24).recommended, "MacBookPro16,2");

        report.platform = "Desktop".into();
        report.cpu.manufacturer = "AMD".into();
        report.cpu.name = "AMD Ryzen 9 5950X 16-Core Processor".into();
        report.cpu.codename = Some("Vermeer".into());
        assert_eq!(recommend(&report, 24).recommended, "iMacPro1,1");
    }

    #[test]
    fn compatible_models_filter_works() {
        let laptops = compatible_models(21, true);
        assert!(laptops.iter().all(|d| d.name.starts_with("MacBook")));
        let desktops = compatible_models(21, false);
        assert!(desktops.iter().all(|d| !d.name.starts_with("MacBook")));
    }
}
