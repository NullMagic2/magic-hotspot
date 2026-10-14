// Module: magic_hotspot.network
// Purpose: Discover iPhone USB links, scan Wi-Fi and activate hotspots through nmcli.
// Created: 2026-10-09
// Architecture: Called by main.rs; delegates privileged networking to NetworkManager.

use std::process::Command;

/// One SSID visible in the most recent NetworkManager scan.
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: String,
    pub security: String,
}

struct UsbIphone {
    interface: String,
    active: bool,
    ready: bool,
}

// The Linux ipheth driver identifies iPhone tethering; an interface name or
// Ethernet type alone cannot distinguish it from an ordinary wired connection.
fn iphone_link_with(
    query: &mut impl FnMut(&[&str]) -> Result<String, String>,
) -> Result<(Option<UsbIphone>, bool), String> {
    let devices = query(&[
        "--terse",
        "--escape",
        "yes",
        "--fields",
        "DEVICE,TYPE,STATE",
        "device",
        "status",
    ])?;
    let rows: Vec<_> = devices.lines().map(split_terse_fields).collect();
    let has_wifi = rows
        .iter()
        .any(|fields| fields.len() == 3 && fields[1] == "wifi");
    let mut iphone = None;
    for fields in rows
        .iter()
        .filter(|fields| fields.len() == 3 && fields[1] == "ethernet")
    {
        let details = query(&[
            "--get-values",
            "GENERAL.DRIVER,WIRED-PROPERTIES.CARRIER",
            "device",
            "show",
            &fields[0],
        ])?;
        let mut values = details.lines();
        if values.next().unwrap_or("").trim() != "ipheth" {
            continue;
        }
        let candidate = UsbIphone {
            interface: fields[0].clone(),
            active: fields[2].starts_with("connected"),
            ready: values.next().unwrap_or("").trim() == "on"
                && !matches!(fields[2].as_str(), "unmanaged" | "unavailable" | "unknown"),
        };
        if iphone
            .as_ref()
            .map(|item: &UsbIphone| !item.ready)
            .unwrap_or(true)
        {
            iphone = Some(candidate);
        }
    }
    Ok((iphone, has_wifi))
}

const USB_SETUP_HINT: &str = "Enable Personal Hotspot. Unlock iPhone and tap Trust.";
/// Prefer a ready iPhone cable link, then the user's saved Wi-Fi profile.
pub fn activate_hotspot(name: &str) -> Result<String, String> {
    if matches!(iphone_link_with(&mut run_query), Ok((Some(link), false)) if !link.ready && !link.active)
    {
        let unit = "/usr/lib/systemd/system/magic-hotspot-usb.service";
        if std::path::Path::new(unit).exists() {
            let started = std::time::SystemTime::now();
            let recovery = Command::new("systemctl")
                .args(["start", "magic-hotspot-usb.service"])
                .env("LC_ALL", "C")
                .output();
            match recovery {
                Ok(result) if result.status.success() => {}
                Ok(_) => {
                    let path = "/run/magic-hotspot-usb/status";
                    if std::fs::metadata(path)
                        .and_then(|file| file.modified())
                        .map(|when| when >= started)
                        .unwrap_or(false)
                    {
                        if let Ok(message) = std::fs::read_to_string(path) {
                            return Err(message.trim().to_string());
                        }
                    }
                    return Err("Could not run USB recovery. Use Magic Hotspot from your local signed-in desktop.".into());
                }
                Err(_) => return Err("Could not start the installed USB recovery service.".into()),
            }
        }
    }
    activate_hotspot_with(name, &mut run_query)
}

fn activate_hotspot_with(
    name: &str,
    query: &mut impl FnMut(&[&str]) -> Result<String, String>,
) -> Result<String, String> {
    match iphone_link_with(query) {
        Ok((Some(link), _)) if link.active => Ok("iPhone USB connection is already active.".into()),
        Ok((Some(link), _)) if link.ready => {
            query(&["--wait", "15", "device", "connect", &link.interface])
                .map(|_| "Connected to iPhone by USB.".into())
                .map_err(|error| format!("Could not connect to iPhone by USB: {error}"))
        }
        Ok((Some(_), false)) => Err(USB_SETUP_HINT.into()),
        Ok((None, false)) => Err("No Wi-Fi adapter or iPhone USB connection detected.".into()),
        Ok(_) => activate_profile_with(name, query),
        Err(error) => Err(format!("Could not check iPhone connections: {error}")),
    }
}

/// Refresh the wireless scan and return visible networks in NetworkManager order.
pub fn scan_wifi() -> Result<Vec<WifiNetwork>, String> {
    scan_wifi_with(run_query)
}

pub(crate) fn run_query(arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("nmcli")
        .env("LC_ALL", "C")
        .args(arguments)
        .output()
        .map_err(|error| format!("Could not run nmcli: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(if detail.trim().is_empty() {
            "NetworkManager query failed. Check your desktop's Wi-Fi settings.".to_string()
        } else {
            detail.trim().to_string()
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

// Successful empty `wifi list` output also occurs with no Wi-Fi adapter. Check
// adapter/radio state first, and scan each usable interface explicitly.
fn scan_wifi_with(
    mut query: impl FnMut(&[&str]) -> Result<String, String>,
) -> Result<Vec<WifiNetwork>, String> {
    let devices = query(&[
        "--terse",
        "--escape",
        "yes",
        "--fields",
        "DEVICE,TYPE,STATE",
        "device",
        "status",
    ])?;
    let wifi: Vec<Vec<String>> = devices
        .lines()
        .map(split_terse_fields)
        .filter(|fields| fields.len() == 3 && fields[1] == "wifi")
        .collect();
    if wifi.is_empty() {
        return Err(
            "NetworkManager detects no Wi-Fi adapter. iPhone USB status is shown above.".into(),
        );
    }
    if query(&["radio", "wifi"])?.trim() != "enabled" {
        return Err(
            "Wi-Fi is off. Enable it in your desktop's Wi-Fi settings, then Refresh.".into(),
        );
    }
    let usable: Vec<_> = wifi
        .iter()
        .filter(|fields| !matches!(fields[2].as_str(), "unmanaged" | "unavailable" | "unknown"))
        .collect();
    if usable.is_empty() {
        return Err(if wifi.iter().all(|fields| fields[2] == "unmanaged") {
            "Your Wi-Fi adapter is not managed by NetworkManager. Check your desktop's network configuration.".into()
        } else {
            "Your Wi-Fi adapter is unavailable. Check its driver or airplane-mode switch in your desktop's settings.".into()
        });
    }
    let mut networks = Vec::new();
    let mut errors = Vec::new();
    for device in usable {
        match query(&[
            "--wait",
            "15",
            "--terse",
            "--escape",
            "yes",
            "--fields",
            "SSID,SIGNAL,SECURITY",
            "device",
            "wifi",
            "list",
            "ifname",
            &device[0],
            "--rescan",
            "yes",
        ]) {
            Ok(text) => append_networks(&text, &mut networks),
            Err(error) => errors.push(error),
        }
    }
    if networks.is_empty() && !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok(networks)
}

fn append_networks(text: &str, networks: &mut Vec<WifiNetwork>) {
    for row in text.lines() {
        let fields = split_terse_fields(row);
        if fields.len() < 3 || fields[0].trim().is_empty() {
            continue;
        }
        if let Some(existing) = networks.iter_mut().find(|item| item.ssid == fields[0]) {
            if fields[1].parse::<u8>().unwrap_or(0) > existing.signal.parse::<u8>().unwrap_or(0) {
                existing.signal = fields[1].clone();
                existing.security = fields[2].clone();
            }
        } else {
            networks.push(WifiNetwork {
                ssid: fields[0].clone(),
                signal: fields[1].clone(),
                security: fields[2].clone(),
            });
        }
    }
}

/// Decode nmcli's escaped colon-separated output without splitting colons in SSIDs.
pub(crate) fn split_terse_fields(row: &str) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut escaped = false;
    for character in row.chars() {
        if escaped {
            fields.last_mut().unwrap().push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == ':' {
            fields.push(String::new());
        } else {
            fields.last_mut().unwrap().push(character);
        }
    }
    if escaped {
        fields.last_mut().unwrap().push('\\');
    }
    fields
}

/// Ask NetworkManager to activate an existing saved connection by its profile name.
pub fn activate_profile(name: &str) -> Result<String, String> {
    activate_profile_with(name, &mut run_query)
}

fn activate_profile_with(
    name: &str,
    query: &mut impl FnMut(&[&str]) -> Result<String, String>,
) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Enter the saved iPhone Wi-Fi connection name.".into());
    }
    query(&["--wait", "15", "connection", "up", "id", name])
        .map(|_| format!("Connected using “{name}”."))
        .map_err(|detail| {
            if detail.contains("unknown connection") || detail.contains("not found") {
                format!("“{name}” is not a saved Wi-Fi connection yet. Connect to it once from Ubuntu’s Wi-Fi menu.")
            } else if detail.trim().is_empty() {
                format!("Could not connect to “{name}”. Check the saved profile and hotspot.")
            } else {
                detail.trim().to_string()
            }
        })
}

#[cfg(test)]
#[path = "../tests/network/mod.rs"]
mod tests;
