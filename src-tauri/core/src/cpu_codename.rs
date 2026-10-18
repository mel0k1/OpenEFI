pub const INTEL_GENERATIONS: &[&str] = &[
    "Arrow Lake-S", "Arrow Lake-H", "Arrow Lake-HX", "Arrow Lake-U", "Lunar Lake",
    "Meteor Lake-H", "Meteor Lake-U", "Raptor Lake-S", "Raptor Lake-E", "Raptor Lake-HX",
    "Raptor Lake-H", "Raptor Lake-PX", "Raptor Lake-P", "Raptor Lake-U", "Alder Lake-S",
    "Alder Lake-HX", "Alder Lake-H", "Alder Lake-P", "Alder Lake-U", "Alder Lake-N",
    "Lakefield", "Rocket Lake-S", "Rocket Lake-E", "Tiger Lake-H", "Tiger Lake-B",
    "Tiger Lake-UP3", "Tiger Lake-H35", "Tiger Lake-UP4", "Ice Lake-U", "Ice Lake-SP",
    "Comet Lake-S", "Comet Lake-W", "Comet Lake-H", "Comet Lake-U", "Coffee Lake-S",
    "Coffee Lake-E", "Coffee Lake-H", "Coffee Lake-U", "Cannon Lake-U", "Whiskey Lake-U",
    "Kaby Lake", "Kaby Lake-H", "Kaby Lake-X", "Kaby Lake-G", "Amber Lake-Y",
    "Cascade Lake-X", "Cascade Lake-P", "Cascade Lake-W", "Skylake", "Skylake-X",
    "Broadwell", "Broadwell-H", "Broadwell-U", "Broadwell-Y", "Broadwell-E",
    "Haswell", "Haswell-ULT", "Haswell-ULX", "Haswell-H", "Haswell-E", "Haswell-EP",
    "Haswell-EX", "Ivy Bridge", "Ivy Bridge-E", "Sandy Bridge", "Sandy Bridge-E",
    "Beckton", "Westmere-EX", "Gulftown", "Westmere-EP", "Clarkdale", "Arrandale",
    "Lynnfield", "Jasper Forest", "Clarksfield", "Gainestown", "Bloomfield",
];

pub const AMD_GENERATIONS: &[&str] = &[
    "Summit Ridge", "Whitehaven", "Raven Ridge", "Great Horned Owl", "Dalí",
    "Banded Kestrel", "River Hawk", "Pinnacle Ridge", "Colfax", "Picasso",
    "Grey Hawk", "Matisse", "Castle Peak", "Renoir", "Lucienne", "Vermeer",
    "Cezanne", "Barceló", "Rembrandt", "Mendocino", "Barceló-R", "Rembrandt-R",
    "V3000", "Chagall", "Raphael", "Storm Peak", "Phoenix", "Dragon Range",
    "Hawk Point", "Granite Ridge", "Strix Point",
];

pub fn intel_codename(name: &str) -> Option<&'static str> {
    let upper = name.to_uppercase();

    if upper.contains("XEON") {
        return xeon_codename(&upper);
    }

    let norm = upper
        .replace("(R)", " ")
        .replace("(TM)", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if norm.contains("CORE ULTRA") {
        return core_ultra_codename(&norm);
    }

    let model = extract_model(&norm, &["CORE I3", "CORE I5", "CORE I7", "CORE I9", "CELERON", "PENTIUM"])?;

    let suffix = model
        .trim_start_matches(|c: char| c.is_ascii_digit())
        .to_string();
    let digits: String = model.chars().take_while(|c| c.is_ascii_digit()).collect();
    let gen_num: u32 = if digits.len() == 5 || (digits.len() == 4 && digits.starts_with('1')) {
        digits.get(..2).and_then(|s| s.parse().ok()).unwrap_or(0)
    } else if digits.len() == 4 {
        digits.get(..1).and_then(|s| s.parse().ok()).unwrap_or(0)
    } else if digits.len() == 3 {
        return legacy_core_codename(&digits);
    } else {
        return None;
    };

    let has = |p: &str| suffix.contains(p);

    Some(match gen_num {
        2 => "Sandy Bridge",
        3 => "Ivy Bridge",
        4 if has("S") || has("T") => "Haswell",
        4 if has("U") || has("Y") => "Haswell-ULT",
        4 if has("M") || has("H") => "Haswell-H",
        4 => "Haswell",
        5 if has("U") || has("Y") => "Broadwell-U",
        5 if has("H") => "Broadwell-H",
        5 if has("E") => "Broadwell-E",
        5 => "Broadwell",
        6 if has("U") || has("Y") => "Skylake",
        6 if has("X") || has("E") => "Skylake-X",
        6 => "Skylake",
        7 if has("U") => "Kaby Lake",
        7 if has("Y") => "Amber Lake-Y",
        7 if has("X") => "Kaby Lake-X",
        7 if has("G") => "Kaby Lake-G",
        7 if has("H") => "Kaby Lake-H",
        7 => "Kaby Lake",
        8 if has("Y") => "Amber Lake-Y",
        8 if has("U") => "Whiskey Lake-U",
        8 if has("H") => "Coffee Lake-H",
        8 => "Coffee Lake-S",
        9 if has("U") || has("Y") => "Coffee Lake-U",
        9 if has("H") => "Coffee Lake-H",
        9 if has("X") => "Cascade Lake-X",
        9 => "Coffee Lake-S",
        10 if has("G") => "Ice Lake-U",
        10 if has("U") => "Comet Lake-U",
        10 if has("H") => "Comet Lake-H",
        10 => "Comet Lake-S",
        11 if has("U") || has("G") => "Tiger Lake-UP3",
        11 if has("H") => "Tiger Lake-H",
        11 => "Rocket Lake-S",
        12 if has("X") => "Alder Lake-HX",
        12 if has("H") => "Alder Lake-H",
        12 if has("P") => "Alder Lake-P",
        12 if has("U") => "Alder Lake-U",
        12 if has("N") => "Alder Lake-N",
        12 => "Alder Lake-S",
        13 if has("X") => "Raptor Lake-HX",
        13 if has("H") => "Raptor Lake-H",
        13 if has("P") => "Raptor Lake-P",
        13 if has("U") => "Raptor Lake-U",
        13 if has("E") => "Raptor Lake-E",
        13 => "Raptor Lake-S",
        14 if has("X") => "Raptor Lake-HX",
        14 if has("H") => "Raptor Lake-H",
        14 if has("U") => "Raptor Lake-U",
        14 => "Raptor Lake-S",
        15 if has("X") => "Arrow Lake-HX",
        15 if has("H") => "Arrow Lake-H",
        15 if has("U") => "Arrow Lake-U",
        15 => "Arrow Lake-S",
        _ => return None,
    })
}

fn core_ultra_codename(upper: &str) -> Option<&'static str> {
    let model = last_model_token(upper)?;
    let digits: String = model.chars().take_while(|c| c.is_ascii_digit()).collect();
    let num: u32 = digits.parse().ok()?;
    if num < 200 {
        return Some(if model.ends_with('H') || upper.contains('H') {
            "Meteor Lake-H"
        } else {
            "Meteor Lake-U"
        });
    }
    Some(if model.ends_with('V') {
        "Lunar Lake"
    } else {
        "Arrow Lake-S"
    })
}

fn last_model_token(upper: &str) -> Option<String> {
    upper
        .split_whitespace()
        .filter(|t| t.chars().any(|c| c.is_ascii_digit()))
        .last()
        .map(|t| t.chars().filter(|c| c.is_ascii_alphanumeric()).collect())
}

fn xeon_codename(upper: &str) -> Option<&'static str> {
    let num: String = upper.chars().filter(|c| c.is_ascii_digit()).take(4).collect();
    let family: u32 = num.get(..1).and_then(|c| c.to_string().parse().ok()).unwrap_or(0);

    if upper.contains("W-") {
        return Some(match family {
            2 | 3 => "Cascade Lake-W",
            _ => "Skylake-X",
        });
    }
    if upper.contains("E3-") {
        return Some(match family {
            1 | 2 | 3 => "Skylake",
            _ => "Haswell",
        });
    }
    Some(match family {
        5 | 6 => "Westmere-EP",
        7 => "Sandy Bridge-E",
        8 => "Haswell-EP",
        1 => "Skylake-X",
        2 => "Cascade Lake-P",
        _ => "Ice Lake-SP",
    })
}

fn legacy_core_codename(digits: &str) -> Option<&'static str> {
    let num: u32 = digits.parse().ok()?;
    Some(match num {
        500..=699 => "Clarkdale",
        700..=879 => "Lynnfield",
        _ => "Bloomfield",
    })
}

pub fn amd_codename(name: &str) -> Option<&'static str> {
    let upper = name.to_uppercase();

    if upper.contains("THREADRIPPER") {
        let model = last_model_token(&upper).unwrap_or_default();
        let digits: String = model.chars().take_while(|c| c.is_ascii_digit()).collect();
        let num: u32 = digits.get(..4).and_then(|s| s.parse().ok()).unwrap_or(0);
        return Some(match num / 1000 {
            3 => "Castle Peak",
            7 => "Storm Peak",
            5 if num >= 5900 => "Chagall",
            _ => "Whitehaven",
        });
    }
    if upper.contains("EPYC") {
        let model = last_model_token(&upper).unwrap_or_default();
        let digits: String = model.chars().take_while(|c| c.is_ascii_digit()).collect();
        let num: u32 = digits.get(..4).and_then(|s| s.parse().ok()).unwrap_or(0);
        return Some(match num / 1000 {
            7 => "Naples",
            8 if num >= 8002 => "Rome",
            9 => "Milan",
            4 => "Genoa",
            _ => "Rome",
        });
    }
    if upper.contains("RYZEN AI") {
        return Some("Strix Point");
    }

    let model = {
        let after_ryzen = upper.split("RYZEN").nth(1).unwrap_or(&upper);
        let tokens: Vec<&str> = after_ryzen
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|t| t.chars().any(|c| c.is_ascii_digit()))
            .collect();
        let token = tokens.iter().find(|t| t.len() >= 3).or_else(|| tokens.first())?;
        (*token).to_string()
    };
    let suffix: String = model.chars().rev().take_while(|c| c.is_ascii_alphabetic()).collect();
    let suffix: String = suffix.chars().rev().collect();
    let is_mobile = suffix.contains('U') || suffix.contains('Y') || suffix.contains('H');
    let digits: String = model.chars().take_while(|c| c.is_ascii_digit()).collect();
    let num: u32 = digits.get(..4).and_then(|s| s.parse().ok())?;
    let series = num / 1000;
    let low = (num % 1000) / 100;

    Some(match (series, is_mobile) {
        (1, false) => "Summit Ridge",
        (1, true) => "Raven Ridge",
        (2, false) if suffix.contains('G') => "Picasso",
        (2, false) => "Pinnacle Ridge",
        (3, false) if suffix.contains('G') => "Picasso",
        (3, false) => "Matisse",
        (3, true) if low >= 5 => "Renoir",
        (3, true) => "Raven Ridge",
        (4, true) if low >= 5 && num % 10 >= 5 => "Lucienne",
        (4, true) => "Renoir",
        (4, false) => "Renoir",
        (5, false) if suffix.contains('G') => "Cezanne",
        (5, false) => "Vermeer",
        (5, true) if num % 1000 <= 500 => "Barceló",
        (5, true) if low >= 5 => "Cezanne",
        (5, true) => "Cezanne",
        (6, _) => "Rembrandt",
        (7, false) => "Raphael",
        (7, true) if suffix.contains("HX") => "Dragon Range",
        (7, true) => "Phoenix",
        (8, false) => "Granite Ridge",
        (8, true) => "Hawk Point",
        (9, false) => "Granite Ridge",
        (9, true) => "Hawk Point",
        _ => "Raphael",
    })
}

fn extract_model(upper: &str, prefixes: &[&str]) -> Option<String> {
    for prefix in prefixes {
        if let Some(pos) = upper.find(prefix) {
            let rest = &upper[pos + prefix.len()..];
            let tokens: Vec<&str> = rest
                .split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|t| !t.is_empty())
                .collect();
            if let Some(token) = tokens.first() {
                return Some(token.to_string());
            }
        }
    }
    None
}

pub fn has_avx2(cpu_name: &str) -> bool {
    let upper = cpu_name.to_uppercase();
    if upper.contains("INTEL") {
        if upper.contains("PENTIUM") || upper.contains("CELERON") {
            return intel_codename(cpu_name)
                .map(|c| c.contains("Haswell") || c.contains("Broadwell") || c.contains("Skylake"))
                .unwrap_or(false);
        }
        return !matches!(
            intel_codename(cpu_name),
            Some("Sandy Bridge")
                | Some("Sandy Bridge-E")
                | Some("Ivy Bridge")
                | Some("Ivy Bridge-E")
                | Some("Bloomfield")
                | Some("Lynnfield")
                | Some("Clarkdale")
                | Some("Arrandale")
                | Some("Gulftown")
                | Some("Westmere-EP")
                | Some("Westmere-EX")
                | Some("Gainestown")
                | Some("Beckton")
                | Some("Jasper Forest")
                | Some("Clarksfield")
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intel_codenames() {
        assert_eq!(intel_codename("Intel(R) Core(TM) i9-9900K CPU @ 3.60GHz"), Some("Coffee Lake-S"));
        assert_eq!(intel_codename("Intel(R) Core(TM) i7-9750H CPU @ 2.60GHz"), Some("Coffee Lake-H"));
        assert_eq!(intel_codename("Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz"), Some("Whiskey Lake-U"));
        assert_eq!(intel_codename("Intel(R) Core(TM) i7-2600 CPU @ 3.40GHz"), Some("Sandy Bridge"));
        assert_eq!(intel_codename("Intel(R) Core(TM) i7-10750H CPU @ 2.60GHz"), Some("Comet Lake-H"));
        assert_eq!(intel_codename("Intel(R) Core(TM) i5-10210U"), Some("Comet Lake-U"));
        assert_eq!(intel_codename("Intel(R) Core(TM) i7-1065G7"), Some("Ice Lake-U"));
        assert_eq!(intel_codename("Intel(R) Core(TM) i7-1165G7"), Some("Tiger Lake-UP3"));
        assert_eq!(intel_codename("Intel(R) Core(TM) i9-11900K"), Some("Rocket Lake-S"));
        assert_eq!(intel_codename("Intel(R) Core(TM) i5-12600K"), Some("Alder Lake-S"));
        assert_eq!(intel_codename("Intel(R) Core(TM) i9-13900K"), Some("Raptor Lake-S"));
        assert_eq!(intel_codename("Intel(R) Core(TM) Ultra 7 155H"), Some("Meteor Lake-H"));
        assert_eq!(intel_codename("Intel(R) Core(TM) Ultra 9 285K"), Some("Arrow Lake-S"));
        assert_eq!(intel_codename("Intel(R) Core(TM) i5-6500 CPU @ 3.20GHz"), Some("Skylake"));
        assert_eq!(intel_codename("Intel(R) Xeon(R) W-3245M CPU @ 3.20GHz"), Some("Cascade Lake-W"));
    }

    #[test]
    fn amd_codenames() {
        assert_eq!(amd_codename("AMD Ryzen 9 5950X 16-Core Processor"), Some("Vermeer"));
        assert_eq!(amd_codename("AMD Ryzen 7 5800H"), Some("Cezanne"));
        assert_eq!(amd_codename("AMD Ryzen 5 5500U"), Some("Barceló"));
        assert_eq!(amd_codename("AMD Ryzen 9 7950X"), Some("Raphael"));
        assert_eq!(amd_codename("AMD Ryzen 9 7945HX"), Some("Dragon Range"));
        assert_eq!(amd_codename("AMD Ryzen 5 7640U"), Some("Phoenix"));
        assert_eq!(amd_codename("AMD Ryzen 5 3400G"), Some("Picasso"));
        assert_eq!(amd_codename("AMD Ryzen 9 9950X"), Some("Granite Ridge"));
    }

    #[test]
    fn avx2_detection() {
        assert!(has_avx2("Intel(R) Core(TM) i9-9900K CPU @ 3.60GHz"));
        assert!(has_avx2("AMD Ryzen 9 5950X"));
        assert!(!has_avx2("Intel(R) Core(TM) i7-2600 CPU @ 3.40GHz"));
    }
}
