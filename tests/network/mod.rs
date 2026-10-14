// Module: magic_hotspot.tests.network
// Purpose: Verify USB discovery, Wi-Fi parsing and profile activation without host changes.
// Created: 2026-10-11
// Architecture: Inject nmcli responses at the networking boundary.

use super::*;

fn cable_query(args: &[&str], state: &str, carrier: &str, driver: &str) -> Result<String, String> {
    if args.contains(&"status") {
        return Ok(format!(
            "enp1s0:ethernet:connected\nenxphone:ethernet:{state}\n"
        ));
    }
    if args.contains(&"show") {
        return Ok(if args.contains(&"enxphone") {
            format!("{driver}\n{carrier}\n")
        } else {
            "r8169\non\n".into()
        });
    }
    panic!("Unexpected query: {args:?}");
}

#[test]
fn ready_iphone_uses_usb_without_a_saved_wifi_profile() {
    let mut activated = false;
    let message = activate_hotspot_with("", &mut |args| {
        if args.contains(&"connect") {
            assert_eq!(args, ["--wait", "15", "device", "connect", "enxphone"]);
            activated = true;
            return Ok(String::new());
        }
        cable_query(args, "disconnected", "on", "ipheth")
    });
    assert!(activated);
    assert_eq!(message.unwrap(), "Connected to iPhone by USB.");
}

#[test]
fn inactive_iphone_cable_gives_setup_help_without_activation() {
    let message = activate_hotspot_with("iPhone", &mut |args| {
        cable_query(args, "unavailable", "off", "ipheth")
    });
    assert_eq!(message.unwrap_err(), USB_SETUP_HINT);
}

#[test]
fn ordinary_ethernet_is_never_activated_as_an_iphone() {
    let message = activate_hotspot_with("iPhone", &mut |args| {
        cable_query(args, "disconnected", "on", "r8152")
    });
    assert!(
        message
            .unwrap_err()
            .contains("No Wi-Fi adapter or iPhone USB")
    );
}

#[test]
fn active_usb_link_is_not_reactivated() {
    let message = activate_hotspot_with("iPhone", &mut |args| {
        cable_query(args, "connected", "on", "ipheth")
    });
    assert!(message.unwrap().contains("already active"));
}

#[test]
fn unmanaged_usb_link_is_not_activated() {
    let message = activate_hotspot_with("iPhone", &mut |args| {
        cable_query(args, "unmanaged", "on", "ipheth")
    });
    assert_eq!(message.unwrap_err(), USB_SETUP_HINT);
}

#[test]
fn saved_wifi_is_used_when_no_usb_link_is_ready() {
    let mut activated = false;
    let message = activate_hotspot_with("My Phone", &mut |args| {
        if args.contains(&"status") {
            return Ok("wlan0:wifi:disconnected\n".into());
        }
        assert_eq!(args, ["--wait", "15", "connection", "up", "id", "My Phone"]);
        activated = true;
        Ok(String::new())
    });
    assert!(activated);
    assert!(message.unwrap().contains("Connected using"));
}

#[test]
fn usb_activation_failure_is_reported() {
    let message = activate_hotspot_with("iPhone", &mut |args| {
        if args.contains(&"connect") {
            return Err("Not authorized".into());
        }
        cable_query(args, "disconnected", "on", "ipheth")
    });
    assert!(
        message
            .unwrap_err()
            .contains("Could not connect to iPhone by USB: Not authorized")
    );
}

#[test]
fn no_adapter_is_not_reported_as_an_empty_successful_scan() {
    let mut calls = 0;
    let result = scan_wifi_with(|args| {
        calls += 1;
        assert!(args.contains(&"DEVICE,TYPE,STATE"));
        Ok("enp1s0:ethernet:connected\nlo:loopback:connected (externally)\n".into())
    });
    assert!(result.err().unwrap().contains("no Wi-Fi adapter"));
    assert_eq!(calls, 1);
}

fn unavailable_case(state: &str, radio: &str) -> String {
    scan_wifi_with(|args| {
        if args.contains(&"status") {
            Ok(format!("wlan0:wifi:{state}\n"))
        } else if args == ["radio", "wifi"] {
            Ok(radio.into())
        } else {
            panic!("An unusable adapter must not be scanned");
        }
    })
    .err()
    .unwrap()
}

#[test]
fn radio_off_has_an_actionable_message() {
    assert!(unavailable_case("unavailable", "disabled").contains("Wi-Fi is off"));
}

#[test]
fn unmanaged_adapter_has_its_own_message() {
    assert!(unavailable_case("unmanaged", "enabled").contains("not managed"));
}

#[test]
fn unavailable_adapter_has_its_own_message() {
    assert!(unavailable_case("unavailable", "enabled").contains("adapter is unavailable"));
}

#[test]
fn usable_adapter_scans_with_an_explicit_interface_and_preserves_escaped_names() {
    let networks = scan_wifi_with(|args| {
        if args.contains(&"status") {
            return Ok("wlan0:wifi:disconnected\n".into());
        }
        if args == ["radio", "wifi"] {
            return Ok("enabled\n".into());
        }
        assert!(args.windows(2).any(|pair| pair == ["ifname", "wlan0"]));
        assert!(args.windows(2).any(|pair| pair == ["--rescan", "yes"]));
        Ok(
            "Library\\: West:40:WPA2\nLibrary\\: West:82:WPA3\nBack\\\\slash:70:WPA2\n:55:WPA2\n"
                .into(),
        )
    })
    .unwrap();
    assert_eq!(networks.len(), 2);
    assert_eq!(networks[0].ssid, "Library: West");
    assert_eq!(networks[0].signal, "82");
    assert_eq!(networks[0].security, "WPA3");
    assert_eq!(networks[1].ssid, "Back\\slash");
}

#[test]
fn second_adapter_is_scanned_when_first_adapter_fails() {
    let networks = scan_wifi_with(|args| {
        if args.contains(&"status") {
            return Ok("wlan0:wifi:disconnected\nwlan1:wifi:connected\n".into());
        }
        if args == ["radio", "wifi"] {
            return Ok("enabled\n".into());
        }
        if args.contains(&"wlan0") {
            return Err("first adapter scan failed".into());
        }
        Ok("iPhone:90:WPA2\n".into())
    })
    .unwrap();
    assert_eq!(networks.len(), 1);
    assert_eq!(networks[0].ssid, "iPhone");
}

#[test]
fn scan_errors_are_not_treated_as_empty_network_lists() {
    let error = scan_wifi_with(|args| {
        if args.contains(&"status") {
            Ok("wlan0:wifi:disconnected\n".into())
        } else if args == ["radio", "wifi"] {
            Ok("enabled\n".into())
        } else {
            Err("Not authorized to scan".into())
        }
    })
    .err()
    .unwrap();
    assert_eq!(error, "Not authorized to scan");
}

#[test]
fn only_a_ready_adapter_with_no_results_returns_an_empty_list() {
    let networks = scan_wifi_with(|args| {
        if args.contains(&"status") {
            Ok("wlan0:wifi:connected\n".into())
        } else if args == ["radio", "wifi"] {
            Ok("enabled\n".into())
        } else {
            Ok(String::new())
        }
    })
    .unwrap();
    assert!(networks.is_empty());
}
