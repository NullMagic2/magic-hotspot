// Module: magic_hotspot.bluetooth
// Purpose: Discover tethering candidates through BlueZ; activate PAN through NetworkManager.
// Created: 2026-10-11
// Architecture: Worker-only bounded D-Bus calls; the desktop agent retains pairing confirmation.

use glib::Variant;
use gtk::gio;
use std::collections::HashMap;
use std::time::Duration;

const NAP: &str = "00001116-0000-1000-8000-00805f9b34fb";
type Properties = HashMap<String, Variant>;
type Objects = HashMap<String, HashMap<String, Properties>>;

#[derive(Clone)]
pub struct Phone {
    pub path: String,
    pub address: String,
    pub name: String,
    pub paired: bool,
    pub nap: bool,
}

fn call(
    bus: &gio::DBusConnection,
    path: &str,
    interface: &str,
    method: &str,
    timeout: i32,
) -> Result<Variant, String> {
    bus.call_sync(
        Some("org.bluez"),
        path,
        interface,
        method,
        None,
        None,
        gio::DBusCallFlags::NONE,
        timeout,
        None::<&gio::Cancellable>,
    )
    .map_err(|e| e.to_string())
}
fn objects(bus: &gio::DBusConnection) -> Result<Objects, String> {
    call(
        bus,
        "/",
        "org.freedesktop.DBus.ObjectManager",
        "GetManagedObjects",
        5000,
    )?
    .child_value(0)
    .get()
    .ok_or_else(|| "Invalid Bluetooth device list".into())
}
fn candidates(objects: Objects) -> Vec<Phone> {
    let mut phones = Vec::new();
    for (path, interfaces) in objects {
        let Some(p) = interfaces.get("org.bluez.Device1") else {
            continue;
        };
        if p.get("Blocked").and_then(Variant::get::<bool>) == Some(true) {
            continue;
        }
        let nap = p
            .get("UUIDs")
            .and_then(Variant::get::<Vec<String>>)
            .unwrap_or_default()
            .iter()
            .any(|s| s.eq_ignore_ascii_case(NAP));
        let phone_class = p
            .get("Class")
            .and_then(Variant::get::<u32>)
            .is_some_and(|v| (v >> 8) & 0x1f == 2);
        if !nap && !phone_class {
            continue;
        }
        let Some(address) = p.get("Address").and_then(Variant::get::<String>) else {
            continue;
        };
        let name = p
            .get("Alias")
            .and_then(Variant::get::<String>)
            .unwrap_or_else(|| address.clone());
        phones.push(Phone {
            path,
            address,
            name,
            nap,
            paired: p
                .get("Paired")
                .and_then(Variant::get::<bool>)
                .unwrap_or(false),
        });
    }
    phones.sort_by(|a, b| a.name.cmp(&b.name));
    phones
}

/// Each successful discovery start owns one session; stop only those sessions.
pub fn scan() -> Result<Vec<Phone>, String> {
    let bus = gio::bus_get_sync(gio::BusType::System, None::<&gio::Cancellable>)
        .map_err(|e| e.to_string())?;
    let before = objects(&bus)?;
    let mut started = Vec::new();
    let mut powered = false;
    let mut failure = None;
    for (path, interfaces) in &before {
        if interfaces
            .get("org.bluez.Adapter1")
            .and_then(|p| p.get("Powered"))
            .and_then(Variant::get::<bool>)
            != Some(true)
        {
            continue;
        }
        powered = true;
        match call(&bus, path, "org.bluez.Adapter1", "StartDiscovery", 5000) {
            Ok(_) => started.push(path.clone()),
            Err(e) => {
                failure = Some(e);
            }
        }
    }
    if !started.is_empty() {
        std::thread::sleep(Duration::from_secs(3));
    }
    let result = objects(&bus).map(candidates);
    for path in started {
        let _ = call(&bus, &path, "org.bluez.Adapter1", "StopDiscovery", 5000);
    }
    let phones = result?;
    if !powered {
        return Err("Bluetooth is off or unavailable".into());
    }
    if phones.is_empty() {
        if let Some(e) = failure {
            return Err(e);
        }
    }
    Ok(phones)
}

pub fn connect(phone: Phone) -> Result<String, String> {
    let bus = gio::bus_get_sync(gio::BusType::System, None::<&gio::Cancellable>)
        .map_err(|e| e.to_string())?;
    if !phone.paired {
        call(&bus, &phone.path, "org.bluez.Device1", "Pair", 45000)?;
    }
    let refreshed = candidates(objects(&bus)?)
        .into_iter()
        .find(|p| p.path == phone.path)
        .ok_or("Phone is unavailable")?;
    if !refreshed.nap {
        return Err("Turn on Personal Hotspot and try again".into());
    }
    let uuid = pan_profile(&refreshed.address, &mut crate::network::run_query)?;
    crate::pan::connect(&refreshed.address, &uuid)?;
    Ok("Connected by Bluetooth".into())
}

fn pan_profile(
    address: &str,
    query: &mut impl FnMut(&[&str]) -> Result<String, String>,
) -> Result<String, String> {
    let profiles = query(&["--terse", "--fields", "UUID,TYPE", "connection", "show"])?;
    let mut selected = None;
    for row in profiles.lines() {
        let Some((uuid, kind)) = row.split_once(':') else {
            continue;
        };
        if kind != "bluetooth" {
            continue;
        }
        let properties = query(&[
            "--escape",
            "no",
            "--get-values",
            "bluetooth.bdaddr,bluetooth.type",
            "connection",
            "show",
            "uuid",
            uuid,
        ])?;
        let mut values = properties.lines();
        if values
            .next()
            .is_some_and(|s| s.eq_ignore_ascii_case(address))
            && values.next() == Some("panu")
        {
            selected = Some(uuid.to_owned());
            break;
        }
    }
    let uuid = match selected {
        Some(uuid) => uuid,
        None => {
            let uuid = glib::uuid_string_random().to_string();
            query(&[
                "--wait",
                "15",
                "connection",
                "add",
                "type",
                "bluetooth",
                "con-name",
                "Magic Hotspot Bluetooth",
                "connection.uuid",
                &uuid,
                "connection.autoconnect",
                "no",
                "bluetooth.bdaddr",
                address,
                "bluetooth.type",
                "panu",
            ])?;
            uuid
        }
    };
    Ok(uuid)
}

#[cfg(test)]
#[path = "../tests/bluetooth/mod.rs"]
mod tests;
