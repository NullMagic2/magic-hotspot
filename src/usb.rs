// Module: magic_hotspot.usb
// Purpose: Observe iPhone cable generations and network-link changes through sysfs.
// Created: 2026-10-09
// Architecture: Read metadata only; GTK polls off-thread and never reads a phone serial.

use std::path::Path;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Snapshot(Vec<Phone>);

#[derive(Clone, PartialEq, Eq)]
struct Phone {
    path: String,
    address: String,
    configuration: String,
    links: Vec<(String, String)>,
}

impl Snapshot {
    pub fn is_connected(&self) -> bool {
        !self.0.is_empty()
    }

    pub fn same_attachment(&self, other: &Self) -> bool {
        self.0
            .iter()
            .map(|phone| (&phone.path, &phone.address))
            .eq(other.0.iter().map(|phone| (&phone.path, &phone.address)))
    }
}

fn value(path: &Path, field: &str) -> String {
    std::fs::read_to_string(path.join(field))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

pub fn snapshot() -> Snapshot {
    snapshot_in(Path::new("/sys/bus/usb/devices"))
}

fn snapshot_in(root: &Path) -> Snapshot {
    let mut phones = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return Snapshot::default();
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if value(&path, "idVendor") != "05ac" || value(&path, "product") != "iPhone" {
            continue;
        }
        let address = value(&path, "devnum");
        if address.is_empty() {
            continue; // It disappeared while being read.
        }
        let mut links = Vec::new();
        if let Ok(interfaces) = std::fs::read_dir(&path) {
            for interface in interfaces.flatten() {
                let interface = interface.path();
                let is_ipheth = std::fs::read_link(interface.join("driver"))
                    .ok()
                    .and_then(|driver| driver.file_name().map(|name| name == "ipheth"))
                    .unwrap_or(false);
                if !is_ipheth {
                    continue;
                }
                if let Ok(names) = std::fs::read_dir(interface.join("net")) {
                    for name in names.flatten() {
                        links.push((
                            name.file_name().to_string_lossy().into_owned(),
                            value(&name.path(), "carrier"),
                        ));
                    }
                }
            }
        }
        links.sort();
        phones.push(Phone {
            path: entry.file_name().to_string_lossy().into_owned(),
            address,
            configuration: value(&path, "bConfigurationValue"),
            links,
        });
    }
    phones.sort_by(|a, b| a.path.cmp(&b.path));
    Snapshot(phones)
}

#[cfg(test)]
#[path = "../tests/usb/mod.rs"]
mod tests;
