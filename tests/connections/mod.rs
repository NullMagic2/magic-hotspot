// Module: magic_hotspot.tests.connections
// Purpose: Verify live transport identity, switching rollback and targeted disconnect.
// Created: 2026-10-11
// Architecture: Inject NetworkManager responses without touching host connections.

use super::*;

#[derive(Default)]
struct Manager {
    active: Vec<(&'static str, &'static str, &'static str)>,
    commands: Vec<Vec<String>>,
    restoring_fails: bool,
    activating: bool,
    stale_priority: bool,
}

impl Manager {
    fn query(&mut self, args: &[&str]) -> Result<String, String> {
        self.commands
            .push(args.iter().map(|arg| arg.to_string()).collect());
        if args.contains(&"UUID,TYPE,DEVICE") {
            return Ok(self
                .active
                .iter()
                .map(|(id, kind, device)| format!("{id}:{kind}:{device}\n"))
                .collect());
        }
        if args.contains(&"IP4.ROUTE,IP6.ROUTE") {
            return Ok(if self.stale_priority {
                "dst = 0.0.0.0/0, nh = 1.2.3.4, mt = 1"
            } else {
                ""
            }
            .into());
        }
        if args.contains(&"GENERAL.STATE") {
            return Ok(if self.activating {
                "activating"
            } else {
                "activated"
            }
            .into());
        }
        if args.contains(&"GENERAL.DRIVER") {
            return Ok(if args.last() == Some(&"usb0") {
                "ipheth"
            } else {
                "r8169"
            }
            .into());
        }
        if args.contains(&"bluetooth.bdaddr,bluetooth.type") {
            return Ok("AA:BB:CC:DD:EE:FF\npanu\n".into());
        }
        if args.contains(&"802-11-wireless.ssid") {
            return Ok("Cafe: Wi-Fi\n".into());
        }
        if args.contains(&"ipv4.route-metric,ipv6.route-metric") {
            return Ok("-1\n600\n".into());
        }
        if args.first() == Some(&"device")
            && args.get(2) == Some(&"wifi0")
            && args.get(4) == Some(&"-1")
            && self.restoring_fails
        {
            return Err("reapply failed".into());
        }
        Ok(String::new())
    }
    fn modifications(&self) -> Vec<&Vec<String>> {
        self.commands
            .iter()
            .filter(|args| args.first().map(String::as_str) == Some("device"))
            .collect()
    }
}

#[test]
fn snapshot_matches_ssid_and_address_and_excludes_unrelated_links() {
    let mut nm = Manager {
        active: vec![
            ("wired", "802-3-ethernet", "eth0"),
            ("usb", "802-3-ethernet", "usb0"),
            ("wifi", "802-11-wireless", "wifi0"),
            ("pan", "bluetooth", r"AA\:BB\:CC\:DD\:EE\:FF"),
            ("vpn", "vpn", "tun0"),
        ],
        ..Default::default()
    };
    let active = snapshot_with(&mut |args| nm.query(args)).unwrap();
    assert_eq!(active.len(), 3);
    assert_eq!(active[0].target, Target::Usb);
    assert_eq!(active[1].target, Target::Wifi("Cafe: Wi-Fi".into()));
    assert_eq!(active[2].device, "AA:BB:CC:DD:EE:FF");
    assert_eq!(
        active[2].target,
        Target::Bluetooth("AA:BB:CC:DD:EE:FF".into())
    );
}

#[test]
fn activating_link_is_not_reported_as_connected() {
    let mut nm = Manager {
        active: vec![("wifi", "802-11-wireless", "wifi0")],
        activating: true,
        ..Default::default()
    };
    assert!(
        snapshot_with(&mut |args| nm.query(args))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn switching_prefers_new_transport_and_restores_previous_profile_metrics() {
    let mut nm = Manager {
        active: vec![
            ("wifi", "802-11-wireless", "wifi0"),
            ("pan", "bluetooth", "bt0"),
        ],
        ..Default::default()
    };
    let mut preference = Preference::default();
    preference
        .select_with(&Target::Wifi("Cafe: Wi-Fi".into()), &mut |args| {
            nm.query(args)
        })
        .unwrap();
    preference
        .select_with(
            &Target::Bluetooth("AA:BB:CC:DD:EE:FF".into()),
            &mut |args| nm.query(args),
        )
        .unwrap();
    let changes = nm.modifications();
    assert_eq!(changes.len(), 3);
    assert_eq!(
        changes[1],
        &vec![
            "device",
            "modify",
            "bt0",
            "ipv4.route-metric",
            "1",
            "ipv6.route-metric",
            "1"
        ]
    );
    assert_eq!(
        changes[2],
        &vec![
            "device",
            "modify",
            "wifi0",
            "ipv4.route-metric",
            "-1",
            "ipv6.route-metric",
            "600"
        ]
    );
    assert!(
        !nm.commands
            .iter()
            .any(|args| args.contains(&"connection".into()) && args.contains(&"modify".into()))
    );
}

#[test]
fn failed_switch_restores_new_link_and_reports_original_error() {
    let mut nm = Manager {
        active: vec![
            ("wifi", "802-11-wireless", "wifi0"),
            ("pan", "bluetooth", "bt0"),
        ],
        ..Default::default()
    };
    let mut preference = Preference::default();
    preference
        .select_with(&Target::Wifi("Cafe: Wi-Fi".into()), &mut |args| {
            nm.query(args)
        })
        .unwrap();
    nm.restoring_fails = true;
    assert!(
        preference
            .select_with(
                &Target::Bluetooth("AA:BB:CC:DD:EE:FF".into()),
                &mut |args| nm.query(args)
            )
            .unwrap_err()
            .contains("reapply failed")
    );
    assert_eq!(
        nm.modifications().last().unwrap()[2..],
        ["bt0", "ipv4.route-metric", "-1", "ipv6.route-metric", "600"]
    );
}

#[test]
fn disconnect_leaves_other_connections_and_routes_untouched() {
    let mut nm = Manager {
        active: vec![
            ("wired", "802-3-ethernet", "eth0"),
            ("wifi", "802-11-wireless", "wifi0"),
            ("pan", "bluetooth", "bt0"),
        ],
        ..Default::default()
    };
    disconnect_with(
        &[Target::Bluetooth("AA:BB:CC:DD:EE:FF".into())],
        &mut |args| nm.query(args),
    )
    .unwrap();
    let downs: Vec<_> = nm
        .commands
        .iter()
        .filter(|args| args.contains(&"disconnect".into()))
        .collect();
    assert_eq!(
        downs,
        [&vec!["--wait", "15", "device", "disconnect", "bt0"]]
    );
    assert!(nm.modifications().is_empty());
}

#[test]
fn absent_target_never_changes_or_disconnects_another_link() {
    let mut nm = Manager {
        active: vec![("wifi", "802-11-wireless", "wifi0")],
        ..Default::default()
    };
    let target = Target::Bluetooth("AA:BB:CC:DD:EE:FF".into());
    assert!(
        Preference::default()
            .select_with(&target, &mut |args| nm.query(args))
            .is_err()
    );
    disconnect_with(&[target], &mut |args| nm.query(args)).unwrap();
    assert!(nm.modifications().is_empty());
    assert!(
        !nm.commands
            .iter()
            .any(|args| args.contains(&"disconnect".into()))
    );
}

#[test]
fn selecting_after_restart_restores_previous_temporary_priority() {
    let mut nm = Manager {
        active: vec![
            ("wifi", "802-11-wireless", "wifi0"),
            ("pan", "bluetooth", "bt0"),
        ],
        stale_priority: true,
        ..Default::default()
    };
    Preference::default()
        .select_with(
            &Target::Bluetooth("AA:BB:CC:DD:EE:FF".into()),
            &mut |args| nm.query(args),
        )
        .unwrap();
    let changes = nm.modifications();
    assert_eq!(changes.len(), 2);
    assert_eq!(
        changes[0][2..],
        [
            "wifi0",
            "ipv4.route-metric",
            "-1",
            "ipv6.route-metric",
            "600"
        ]
    );
    assert_eq!(
        changes[1][2..],
        ["bt0", "ipv4.route-metric", "1", "ipv6.route-metric", "1"]
    );
}

#[test]
fn selecting_working_bluetooth_never_reactivates_its_link() {
    let mut nm = Manager {
        active: vec![("pan", "bluetooth", "bt0")],
        ..Default::default()
    };
    Preference::default()
        .connect_with(
            &Target::Bluetooth("AA:BB:CC:DD:EE:FF".into()),
            || panic!("working link must not be reactivated"),
            &mut |args| nm.query(args),
        )
        .unwrap();
    assert_eq!(nm.modifications().len(), 1);
}

#[test]
fn state_query_failure_never_restarts_a_possibly_working_link() {
    let mut called = false;
    assert!(
        Preference::default()
            .connect_with(
                &Target::Bluetooth("AA:BB:CC:DD:EE:FF".into()),
                || {
                    called = true;
                    Ok(String::new())
                },
                &mut |_| Err("NetworkManager unavailable".into())
            )
            .is_err()
    );
    assert!(!called);
}
