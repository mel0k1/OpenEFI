use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

pub static PCI_IDS: &str = include_str!("pci_ids.txt");
pub static DEVICE_IDS_JSON: &str = include_str!("device_ids.json");
pub static MAC_MODELS_JSON: &str = include_str!("mac_models.json");

pub struct PciDatabase {
    vendors: HashMap<&'static str, &'static str>,
    devices: HashMap<&'static str, &'static str>,
}

impl PciDatabase {
    pub fn global() -> &'static PciDatabase {
        static DB: OnceLock<PciDatabase> = OnceLock::new();
        DB.get_or_init(|| {
            let mut vendors = HashMap::new();
            let mut devices = HashMap::new();
            let mut current_vendor = "";
            for line in PCI_IDS.lines() {
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }
                if !line.starts_with('\t') {
                    let (id, name) = line.split_once("  ").unwrap_or((line, line));
                    current_vendor = id;
                    vendors.insert(current_vendor, name);
                } else if !line.starts_with("\t\t") {
                    let body = line.trim_start();
                    if let Some((id, name)) = body.split_once("  ") {
                        let key = format!("{current_vendor}:{id}");
                        devices.insert(leak_string(key), name);
                    }
                }
            }
            PciDatabase { vendors, devices }
        })
    }

    pub fn vendor_name(&self, vendor: &str) -> Option<&'static str> {
        self.vendors.get(vendor.to_lowercase().as_str()).copied()
    }

    pub fn device_name(&self, vendor: &str, device: &str) -> Option<&'static str> {
        let key = format!("{}:{}", vendor.to_lowercase(), device.to_lowercase());
        self.devices.get(key.as_str()).copied()
    }
}

// pci_ids.txt lives for the whole program lifetime, so leaking parsed keys is safe
fn leak_string(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

#[derive(Deserialize, Clone)]
pub struct MacModelRaw {
    pub prefix: String,
    pub code: String,
    pub board: String,
    pub years: Vec<u32>,
}

pub fn mac_models() -> &'static HashMap<String, MacModelRaw> {
    static MODELS: OnceLock<HashMap<String, MacModelRaw>> = OnceLock::new();
    MODELS.get_or_init(|| serde_json::from_str(MAC_MODELS_JSON).expect("parse mac_models.json"))
}

#[derive(Deserialize, Clone)]
pub struct DeviceIds {
    pub pci: HashMap<String, Vec<String>>,
    pub codecs: Vec<String>,
}

pub fn device_ids() -> &'static DeviceIds {
    static IDS: OnceLock<DeviceIds> = OnceLock::new();
    IDS.get_or_init(|| serde_json::from_str(DEVICE_IDS_JSON).expect("parse device_ids.json"))
}

impl DeviceIds {
    pub fn list(&self, name: &str) -> &[String] {
        static EMPTY: Vec<String> = Vec::new();
        self.pci.get(name).unwrap_or(&EMPTY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pci_lookup() {
        let db = PciDatabase::global();
        assert_eq!(
            db.vendor_name("8086"),
            Some("Intel Corporation")
        );
        assert!(db.device_name("8086", "a780").is_some() || db.device_name("8086", "4680").is_some());
        assert!(db.device_name("1002", "744c").is_some() || db.device_name("10de", "28e0").is_some());
        assert_eq!(db.vendor_name("10de"), Some("NVIDIA Corporation"));
    }

    #[test]
    fn device_ids_loaded() {
        let ids = device_ids();
        assert!(ids.list("IntelWiFiIDs").len() > 50);
        assert!(ids.list("IntelMausiIDs").len() > 30);
        assert!(ids.codecs.len() > 50);
        assert!(ids.list("BroadcomWiFiIDs").iter().any(|x| x == "14E4-4311"));
    }

    #[test]
    fn mac_models_loaded() {
        let models = mac_models();
        assert!(models.len() > 60);
        let imac = models.get("iMac19,1").unwrap();
        assert_eq!(imac.prefix.len(), 12);
        assert_eq!(imac.code, "JV3Q");
        assert_eq!(imac.years, vec![2019, 2020]);
    }
}
