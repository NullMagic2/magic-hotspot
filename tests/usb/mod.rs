// Module: magic_hotspot.tests.usb
// Purpose: Verify attachment metadata without reading phone identifiers.
// Created: 2026-10-11
// Architecture: Temporary sysfs fixtures, never real USB devices.

use super::*;

fn phone(root: &Path, vendor: &str, product: &str, address: &str) -> std::path::PathBuf {
    let path = root.join("1-4");
    std::fs::create_dir_all(&path).unwrap();
    for (field, contents) in [
        ("idVendor", vendor),
        ("product", product),
        ("devnum", address),
        ("bConfigurationValue", "4"),
    ] {
        std::fs::write(path.join(field), contents).unwrap();
    }
    path
}

#[test]
fn unplug_and_replug_at_the_same_port_are_different_generations() {
    let root = tempfile::tempdir().unwrap();
    let path = phone(root.path(), "05ac", "iPhone", "12");
    let first = snapshot_in(root.path());
    assert!(first.is_connected());
    std::fs::remove_dir_all(&path).unwrap();
    assert!(!snapshot_in(root.path()).is_connected());
    phone(root.path(), "05ac", "iPhone", "13");
    assert!(first != snapshot_in(root.path()));
}

#[test]
fn other_usb_devices_are_ignored() {
    let root = tempfile::tempdir().unwrap();
    phone(root.path(), "05ac", "iPad", "12");
    assert!(!snapshot_in(root.path()).is_connected());
    phone(root.path(), "1234", "iPhone", "12");
    assert!(!snapshot_in(root.path()).is_connected());
}

#[test]
fn driver_attach_and_carrier_changes_refresh_the_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let path = phone(root.path(), "05ac", "iPhone", "12");
    let before = snapshot_in(root.path());
    let interface = path.join("1-4:4.2");
    let link = interface.join("net/enxphone");
    std::fs::create_dir_all(&link).unwrap();
    std::os::unix::fs::symlink("/sys/bus/usb/drivers/ipheth", interface.join("driver")).unwrap();
    std::fs::write(link.join("carrier"), "0").unwrap();
    let attached = snapshot_in(root.path());
    assert!(before != attached);
    std::fs::write(link.join("carrier"), "1").unwrap();
    assert!(attached != snapshot_in(root.path()));
}
