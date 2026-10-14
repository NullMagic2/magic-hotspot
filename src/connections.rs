// Module: magic_hotspot.connections
// Purpose: Observe active transports and apply the user's temporary route preference.
// Created: 2026-10-11
// Architecture: NetworkManager owns links and profiles; all calls run on workers.

use crate::network::{run_query, split_terse_fields};

type Query<'a> = &'a mut dyn FnMut(&[&str]) -> Result<String, String>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Usb,
    Wifi(String),
    Bluetooth(String),
}

impl Target {
    pub fn mode(&self) -> &'static str {
        match self {
            Self::Usb => "USB",
            Self::Wifi(_) => "Wi-Fi",
            Self::Bluetooth(_) => "Bluetooth",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActiveConnection {
    pub uuid: String,
    pub device: String,
    pub target: Target,
}

pub fn snapshot() -> Result<Vec<ActiveConnection>, String> {
    snapshot_with(&mut run_query)
}

fn snapshot_with(query: Query<'_>) -> Result<Vec<ActiveConnection>, String> {
    let output = query(&[
        "--terse",
        "--escape",
        "yes",
        "--fields",
        "UUID,TYPE,DEVICE",
        "connection",
        "show",
        "--active",
    ])?;
    let mut active = Vec::new();
    for line in output.lines() {
        let fields = split_terse_fields(line);
        if fields.len() != 3 || fields[2].is_empty() || fields[2] == "--" {
            continue;
        }
        if !matches!(
            fields[1].as_str(),
            "bluetooth" | "802-11-wireless" | "802-3-ethernet"
        ) {
            continue;
        }
        if query(&[
            "--get-values",
            "GENERAL.STATE",
            "connection",
            "show",
            "uuid",
            &fields[0],
        ])?
        .trim()
            != "activated"
        {
            continue;
        }
        let target = match fields[1].as_str() {
            "bluetooth" => {
                let details = query(&[
                    "--escape",
                    "no",
                    "--get-values",
                    "bluetooth.bdaddr,bluetooth.type",
                    "connection",
                    "show",
                    "uuid",
                    &fields[0],
                ])?;
                let mut values = details.lines();
                let address = values.next().unwrap_or("").trim().to_ascii_uppercase();
                if values.next() != Some("panu") {
                    continue;
                }
                Target::Bluetooth(address)
            }
            "802-11-wireless" => Target::Wifi(
                query(&[
                    "--escape",
                    "no",
                    "--get-values",
                    "802-11-wireless.ssid",
                    "connection",
                    "show",
                    "uuid",
                    &fields[0],
                ])?
                .trim_end_matches('\n')
                .to_string(),
            ),
            "802-3-ethernet" => {
                if query(&[
                    "--get-values",
                    "GENERAL.DRIVER",
                    "device",
                    "show",
                    &fields[2],
                ])?
                .trim()
                    != "ipheth"
                {
                    continue;
                }
                Target::Usb
            }
            _ => continue,
        };
        active.push(ActiveConnection {
            uuid: fields[0].clone(),
            device: fields[2].clone(),
            target,
        });
    }
    Ok(active)
}

struct PreferredLink {
    uuid: String,
    device: String,
    metrics: [String; 2],
}

/// Only the previous application-selected link needs its temporary metrics restored.
#[derive(Default)]
pub struct Preference {
    previous: Option<PreferredLink>,
}

impl Preference {
    pub fn connect(
        &mut self,
        target: &Target,
        activate: impl FnOnce() -> Result<String, String>,
    ) -> Result<(), String> {
        self.connect_with(target, activate, &mut run_query)
    }

    fn connect_with(
        &mut self,
        target: &Target,
        activate: impl FnOnce() -> Result<String, String>,
        query: Query<'_>,
    ) -> Result<(), String> {
        // Selecting an already-active mode must never tear down its working link.
        if !snapshot_with(query)?
            .iter()
            .any(|link| &link.target == target)
        {
            activate()?;
        }
        self.select_with(target, query)
    }

    fn select_with(&mut self, target: &Target, query: Query<'_>) -> Result<(), String> {
        let active = snapshot_with(query)?;
        let link = active
            .iter()
            .find(|link| &link.target == target)
            .ok_or_else(|| {
                "The selected connection is not active. Try connecting again.".to_string()
            })?;
        // After an app restart, clear a previous temporary priority on another
        // hotspot link before selecting this one. Normal profile priorities survive.
        if self.previous.is_none() {
            for other in active.iter().filter(|other| other.uuid != link.uuid) {
                let routes = query(&[
                    "--get-values",
                    "IP4.ROUTE,IP6.ROUTE",
                    "device",
                    "show",
                    &other.device,
                ])?;
                if !routes
                    .split(['|', '\n'])
                    .any(|route| route.trim().ends_with(", mt = 1"))
                {
                    continue;
                }
                let metrics = query(&[
                    "--get-values",
                    "ipv4.route-metric,ipv6.route-metric",
                    "connection",
                    "show",
                    "uuid",
                    &other.uuid,
                ])?;
                let values: Vec<_> = metrics.lines().collect();
                if values.len() != 2 {
                    return Err(
                        "Could not restore the previous connection's route settings.".into(),
                    );
                }
                query(&[
                    "device",
                    "modify",
                    &other.device,
                    "ipv4.route-metric",
                    values[0],
                    "ipv6.route-metric",
                    values[1],
                ])?;
            }
        }
        let metrics = query(&[
            "--get-values",
            "ipv4.route-metric,ipv6.route-metric",
            "connection",
            "show",
            "uuid",
            &link.uuid,
        ])?;
        let values: Vec<_> = metrics.lines().collect();
        if values.len() != 2 || values.iter().any(|value| value.parse::<i64>().is_err()) {
            return Err("Could not read the connection's route settings.".into());
        }
        // Modify only the active connection, leaving saved profiles and credentials intact.
        query(&[
            "device",
            "modify",
            &link.device,
            "ipv4.route-metric",
            "1",
            "ipv6.route-metric",
            "1",
        ])?;
        if let Some(previous) = &self.previous {
            if previous.uuid != link.uuid
                && active
                    .iter()
                    .any(|item| item.uuid == previous.uuid && item.device == previous.device)
            {
                if let Err(error) = query(&[
                    "device",
                    "modify",
                    &previous.device,
                    "ipv4.route-metric",
                    &previous.metrics[0],
                    "ipv6.route-metric",
                    &previous.metrics[1],
                ]) {
                    // Undo the new priority if restoring the old one failed; never report success.
                    let rollback = query(&[
                        "device",
                        "modify",
                        &link.device,
                        "ipv4.route-metric",
                        values[0],
                        "ipv6.route-metric",
                        values[1],
                    ]);
                    return Err(format!(
                        "Could not switch the preferred connection: {error}{}",
                        rollback
                            .err()
                            .map(|error| format!("; could not restore the new link: {error}"))
                            .unwrap_or_default()
                    ));
                }
            }
        }
        self.previous = Some(PreferredLink {
            uuid: link.uuid.clone(),
            device: link.device.clone(),
            metrics: [values[0].into(), values[1].into()],
        });
        Ok(())
    }
}

pub fn disconnect(targets: &[Target]) -> Result<(), String> {
    disconnect_with(targets, &mut run_query)
}

fn disconnect_with(targets: &[Target], query: Query<'_>) -> Result<(), String> {
    // Resolve again at click time, so a stale row cannot disconnect a different profile.
    for link in snapshot_with(query)?
        .iter()
        .filter(|link| targets.contains(&link.target))
    {
        query(&["--wait", "15", "device", "disconnect", &link.device])?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/connections/mod.rs"]
mod tests;
