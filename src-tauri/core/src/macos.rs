use serde::Serialize;

pub const MACOS_VERSIONS: &[(&str, &str, u32)] = &[
    ("26", "Tahoe", 25),
    ("15", "Sequoia", 24),
    ("14", "Sonoma", 23),
    ("13", "Ventura", 22),
    ("12", "Monterey", 21),
    ("11", "Big Sur", 20),
    ("10.15", "Catalina", 19),
];

pub const LATEST_DARWIN: u32 = 25;

#[derive(Serialize, Clone, Debug)]
pub struct MacosVersion {
    pub number: String,
    pub name: String,
    pub darwin: u32,
    pub supported: bool,
    pub reason: String,
}

pub fn parse_darwin(version: &str) -> u32 {
    version
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

pub fn macos_name_by_darwin(darwin: u32) -> &'static str {
    MACOS_VERSIONS
        .iter()
        .find(|(_, _, d)| *d == darwin)
        .map(|(n, name, _)| if n.contains('.') { concat!("macOS ", "") } else { name })
        .unwrap_or("macOS")
}

pub fn build_version_list(max_darwin: u32) -> Vec<MacosVersion> {
    MACOS_VERSIONS
        .iter()
        .map(|(number, name, darwin)| MacosVersion {
            number: number.to_string(),
            name: name.to_string(),
            darwin: *darwin,
            supported: *darwin <= max_darwin,
            reason: if *darwin > max_darwin {
                "Выше лимита железа".into()
            } else {
                String::new()
            },
        })
        .collect()
}
