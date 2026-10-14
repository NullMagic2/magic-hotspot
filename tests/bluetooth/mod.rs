// Module: magic_hotspot.bluetooth.tests
// Purpose: Check tethering selection and exact NetworkManager operations without hardware.
// Created: 2026-10-11
// Architecture: BlueZ property fixtures and an injected nmcli query.
use super::*;
use glib::variant::ToVariant;

fn device(class: u32, nap: bool, blocked: bool) -> Properties {
    HashMap::from([
        ("Class".into(), class.to_variant()),
        ("Address".into(), "AA:BB:CC:DD:EE:FF".to_variant()),
        ("Alias".into(), "Phone".to_variant()),
        ("Paired".into(), true.to_variant()),
        ("Blocked".into(), blocked.to_variant()),
        (
            "UUIDs".into(),
            if nap { vec![NAP.to_string()] } else { vec![] }.to_variant(),
        ),
    ])
}
#[test]
fn only_phones_and_nap_devices_are_candidates_not_blocked_devices() {
    for (class, nap, blocked, included) in [
        (0x0204, false, false, true),
        (0x0104, true, false, true),
        (0x0540, false, false, false),
        (0x0204, true, true, false),
    ] {
        let objects = HashMap::from([(
            "/org/bluez/hci0/dev_AA".into(),
            HashMap::from([("org.bluez.Device1".into(), device(class, nap, blocked))]),
        )]);
        assert_eq!(candidates(objects).len(), usize::from(included));
    }
}
#[test]
fn bluez_object_paths_and_wrapped_properties_decode() {
    let path = glib::variant::ObjectPath::try_from("/org/bluez/hci0/dev_AA").unwrap();
    let data = HashMap::from([(
        path,
        HashMap::from([("org.bluez.Device1".to_string(), device(0x0204, true, false))]),
    )])
    .to_variant();
    let phones = candidates(data.get::<Objects>().unwrap());
    assert_eq!(phones.len(), 1);
    assert_eq!(phones[0].address, "AA:BB:CC:DD:EE:FF");
    assert!(phones[0].nap && phones[0].paired);
}
#[test]
fn reuse_matching_pan_profile_without_touching_other_connections() {
    let mut operations = vec![];
    let message = pan_profile("AA:BB:CC:DD:EE:FF", &mut |args| {
        operations.push(args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        if args.contains(&"UUID,TYPE") {
            return Ok("wifi:wifi\ndun:bluetooth\npan:bluetooth\n".into());
        }
        if args.contains(&"dun") {
            return Ok("AA:BB:CC:DD:EE:FF\ndun\n".into());
        }
        if args.contains(&"bluetooth.bdaddr,bluetooth.type") {
            assert!(args.windows(2).any(|p| p == ["--escape", "no"]));
            return Ok("aa:bb:cc:dd:ee:ff\npanu\n".into());
        }
        panic!("Unexpected profile operation: {args:?}")
    })
    .unwrap();
    assert_eq!(message, "pan");
    assert_eq!(operations.len(), 3);
}
#[test]
fn new_pan_profile_has_explicit_uuid_and_does_not_autoconnect() {
    let mut created = None;
    pan_profile("AA:BB:CC:DD:EE:FF", &mut |args| {
        if args.contains(&"UUID,TYPE") {
            return Ok(String::new());
        }
        if args.contains(&"add") {
            assert!(
                args.windows(2)
                    .any(|p| p == ["connection.autoconnect", "no"])
            );
            assert!(args.windows(2).any(|p| p == ["bluetooth.type", "panu"]));
            let uuid = args.iter().position(|a| *a == "connection.uuid").unwrap() + 1;
            created = Some(args[uuid].to_string());
        } else {
            assert_eq!(args.last().copied(), created.as_deref());
        }
        Ok(String::new())
    })
    .unwrap();
}
#[test]
fn add_failure_is_preserved_and_never_activates_a_profile() {
    let mut calls = 0;
    let error = pan_profile("AA:BB:CC:DD:EE:FF", &mut |args| {
        calls += 1;
        if args.contains(&"UUID,TYPE") {
            Ok(String::new())
        } else {
            Err("not authorized".into())
        }
    })
    .unwrap_err();
    assert_eq!(error, "not authorized");
    assert_eq!(calls, 2);
}
