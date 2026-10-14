// Module: magic_hotspot.pan
// Purpose: Observe PAN IP setup and retry one failed hotspot activation.
// Created: 2026-10-13
// Architecture: Read-only NetworkManager D-Bus polling; nmcli owns activation.
use glib::variant::ToVariant;
use gtk::gio;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const ACTIVATION_WAIT: &str = "90";
const POLL_INTERVAL: Duration = Duration::from_millis(250);
const NM: &str = "org.freedesktop.NetworkManager";
const DEVICE: &str = "org.freedesktop.NetworkManager.Device";
pub(crate) const RECOVERY_HINT: &str = "Bluetooth connected, but hotspot IP setup failed again. Turn Personal Hotspot off and on, then try again. If it still fails, restart the phone.";

#[derive(Default, Clone)]
struct Sample {
    state: u32,
    reason: u32,
    uuid: Option<String>,
}
#[derive(Default)]
struct Evidence {
    started: bool,
    reached_ip_setup: bool,
    passed_ip_setup: bool,
    interrupted: bool,
}
impl Evidence {
    fn observe(&mut self, sample: &Sample, uuid: &str) {
        self.started |= sample.uuid.as_deref() == Some(uuid) && matches!(sample.state, 40..=100);
        if !self.started {
            return;
        }
        if sample.uuid.as_deref().is_some_and(|id| id != uuid) || matches!(sample.reason, 36..=39) {
            self.interrupted = true;
        }
        if sample.uuid.as_deref() == Some(uuid) {
            self.reached_ip_setup |= sample.state == 70;
            self.passed_ip_setup |= matches!(sample.state, 80 | 90 | 100);
        }
    }
    fn retryable(&self, error: &str, last: Option<&Sample>) -> bool {
        self.reached_ip_setup
            && !self.passed_ip_setup
            && !self.interrupted
            && last.is_some_and(|s| matches!(s.state, 30 | 120))
            && (error.contains("The Bluetooth connection failed or timed out")
                || error.contains("IP configuration could not be reserved"))
    }
}
struct Failure {
    error: String,
    retryable: bool,
}
fn recover(mut attempt: impl FnMut() -> Result<(), Failure>) -> Result<(), String> {
    match attempt() {
        Ok(()) => Ok(()),
        Err(first) if first.retryable => match attempt() {
            Ok(()) => Ok(()),
            Err(second) if second.retryable => {
                // The dialog displays the translated guidance; retain diagnostics
                // after Hint for callers, as with nmcli's existing error format.
                Err(format!("{RECOVERY_HINT}\nHint: {}", second.error))
            }
            Err(second) => Err(second.error),
        },
        Err(first) => Err(first.error),
    }
}
fn properties(
    bus: &gio::DBusConnection,
    path: &str,
    interface: &str,
) -> Option<HashMap<String, glib::Variant>> {
    bus.call_sync(
        Some(NM),
        path,
        "org.freedesktop.DBus.Properties",
        "GetAll",
        Some(&(interface,).to_variant()),
        None,
        gio::DBusCallFlags::NONE,
        1000,
        None::<&gio::Cancellable>,
    )
    .ok()?
    .child_value(0)
    .get()
}
fn sample(bus: &gio::DBusConnection, path: &str) -> Option<Sample> {
    let p = properties(bus, path, DEVICE)?;
    let state = p.get("State")?.get::<u32>()?;
    let (_, reason) = p.get("StateReason")?.get::<(u32, u32)>()?;
    let active = p
        .get("ActiveConnection")?
        .get::<glib::variant::ObjectPath>()?;
    let uuid = if active.as_str() == "/" {
        None
    } else {
        Some(
            properties(
                bus,
                active.as_str(),
                "org.freedesktop.NetworkManager.Connection.Active",
            )?
            .get("Uuid")?
            .get::<String>()?,
        )
    };
    Some(Sample {
        state,
        reason,
        uuid,
    })
}

pub fn connect(address: &str, uuid: &str) -> Result<(), String> {
    // Missing observation must never prevent a normal connection or justify a retry.
    let bus = gio::bus_get_sync(gio::BusType::System, None::<&gio::Cancellable>).ok();
    let path = crate::network::run_query(&[
        "--get-values",
        "GENERAL.DBUS-PATH",
        "device",
        "show",
        address,
    ])
    .ok();
    recover(|| {
        let stop = AtomicBool::new(false);
        let (result, evidence, last) = std::thread::scope(|scope| {
            let observer = scope.spawn(|| {
                let mut evidence = Evidence::default();
                let mut last = None;
                if let (Some(bus), Some(path)) = (bus.as_ref(), path.as_ref()) {
                    let deadline = Instant::now() + Duration::from_secs(95);
                    loop {
                        last = sample(bus, path.trim());
                        if let Some(s) = last.as_ref() {
                            evidence.observe(s, uuid);
                        }
                        if stop.load(Ordering::Acquire) || Instant::now() >= deadline {
                            break;
                        }
                        std::thread::sleep(POLL_INTERVAL);
                    }
                }
                (evidence, last)
            });
            let result = crate::network::run_query(&[
                "--wait",
                ACTIVATION_WAIT,
                "connection",
                "up",
                "uuid",
                uuid,
            ]);
            stop.store(true, Ordering::Release);
            let (evidence, last) = observer.join().unwrap_or_default();
            (result, evidence, last)
        });
        result.map(|_| ()).map_err(|error| Failure {
            retryable: evidence.retryable(&error, last.as_ref()),
            error,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires tests/pan/check.py isolated NetworkManager bus"]
    fn isolated_network_manager_recovery() {
        assert_eq!(std::env::var("MAGIC_PAN_TEST").as_deref(), Ok("isolated"));
        let result = connect("AA:BB:CC:DD:EE:FF", "selected");
        match std::env::var("PAN_CASE").unwrap().as_str() {
            "recover" => assert!(result.is_ok(), "{result:?}"),
            "persistent" => assert!(result.unwrap_err().starts_with(RECOVERY_HINT)),
            "permission" => assert!(result.unwrap_err().contains("not authorized")),
            "cancel" => assert!(result.unwrap_err().contains("user requested")),
            _ => panic!("unknown test case"),
        }
    }
    #[test]
    fn old_disconnect_reason_does_not_suppress_new_attempt() {
        let mut e = Evidence::default();
        e.observe(
            &Sample {
                state: 30,
                reason: 39,
                ..Default::default()
            },
            "selected",
        );
        e.observe(&ip_setup(), "selected");
        assert!(e.retryable(
            ERROR,
            Some(&Sample {
                state: 30,
                ..Default::default()
            })
        ));
    }
    const ERROR: &str =
        "Error: Connection activation failed: The Bluetooth connection failed or timed out";
    fn ip_setup() -> Sample {
        Sample {
            state: 70,
            uuid: Some("selected".into()),
            ..Default::default()
        }
    }
    #[test]
    fn detects_failure_before_ip_but_not_after_ip_or_external_interruption() {
        for (middle, expected) in [
            (None, true),
            (
                Some(Sample {
                    state: 80,
                    uuid: Some("selected".into()),
                    ..Default::default()
                }),
                false,
            ),
            (
                Some(Sample {
                    state: 30,
                    reason: 39,
                    ..Default::default()
                }),
                false,
            ),
            (
                Some(Sample {
                    state: 70,
                    uuid: Some("other".into()),
                    ..Default::default()
                }),
                false,
            ),
        ] {
            let mut e = Evidence::default();
            e.observe(&ip_setup(), "selected");
            if let Some(s) = middle {
                e.observe(&s, "selected");
            }
            assert_eq!(
                e.retryable(
                    ERROR,
                    Some(&Sample {
                        state: 30,
                        ..Default::default()
                    })
                ),
                expected
            );
            assert!(!e.retryable(
                "not authorized",
                Some(&Sample {
                    state: 30,
                    ..Default::default()
                })
            ));
            assert!(!e.retryable(ERROR, None));
            assert!(!e.retryable(ERROR, Some(&ip_setup())));
        }
        assert!(!Evidence::default().retryable(
            ERROR,
            Some(&Sample {
                state: 30,
                ..Default::default()
            })
        ));
    }
    #[test]
    fn one_retry_can_recover_and_success_does_not_retry() {
        let mut calls = 0;
        assert!(
            recover(|| {
                calls += 1;
                if calls == 1 {
                    Err(Failure {
                        error: ERROR.into(),
                        retryable: true,
                    })
                } else {
                    Ok(())
                }
            })
            .is_ok()
        );
        assert_eq!(calls, 2);
        calls = 0;
        assert!(
            recover(|| {
                calls += 1;
                Ok(())
            })
            .is_ok()
        );
        assert_eq!(calls, 1);
    }
    #[test]
    fn persistent_failure_is_bounded_and_has_guidance() {
        let mut calls = 0;
        let error = recover(|| {
            calls += 1;
            Err(Failure {
                error: ERROR.into(),
                retryable: true,
            })
        })
        .unwrap_err();
        assert_eq!(calls, 2);
        assert!(error.starts_with(RECOVERY_HINT));
        assert!(error.contains(ERROR));
    }
    #[test]
    fn unrelated_errors_are_preserved_without_retry() {
        let mut calls = 0;
        let error = recover(|| {
            calls += 1;
            Err(Failure {
                error: "not authorized".into(),
                retryable: false,
            })
        })
        .unwrap_err();
        assert_eq!(calls, 1);
        assert_eq!(error, "not authorized");
    }
}
