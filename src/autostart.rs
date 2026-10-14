// Module: magic_hotspot.autostart
// Purpose: Manage the user's optional desktop login startup entry.
// Created: 2026-10-09
// Architecture: GTK controls call this safe XDG file boundary; no system settings change.

use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

const DESKTOP_GROUP: &str = "Desktop Entry";
const OWNER_KEY: &str = "X-Magic-Hotspot-Managed";
const FILE_NAME: &str = "magic-hotspot.desktop";

pub struct Autostart {
    directory: PathBuf,
    executable: PathBuf,
}

impl Autostart {
    pub fn current() -> Result<Self, String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        Ok(Self::at(
            glib::user_config_dir().join("autostart"),
            executable,
        ))
    }

    pub fn at(directory: PathBuf, executable: PathBuf) -> Self {
        Self {
            directory,
            executable,
        }
    }

    pub fn is_enabled(&self) -> Result<bool, String> {
        match self.owned_entry()? {
            Some(entry) => Ok(!entry.boolean(DESKTOP_GROUP, "Hidden").unwrap_or(false)
                && entry
                    .boolean(DESKTOP_GROUP, "X-GNOME-Autostart-enabled")
                    .unwrap_or(true)),
            None => Ok(false),
        }
    }

    /// Change only an entry owned by Magic Hotspot. Atomic replacement prevents
    /// partial desktop files; unrelated files and symbolic links are rejected.
    pub fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        let existing = self.owned_entry()?;
        let destination = self.directory.join(FILE_NAME);
        if !enabled {
            if existing.is_some() {
                std::fs::remove_file(destination).map_err(|error| error.to_string())?;
            }
            return Ok(());
        }

        let entry = glib::KeyFile::new();
        entry.set_string(DESKTOP_GROUP, "Type", "Application");
        entry.set_string(DESKTOP_GROUP, "Name", "Magic Hotspot");
        entry.set_string(
            DESKTOP_GROUP,
            "Exec",
            &format!("{} --background", quote_exec(&self.executable)?),
        );
        entry.set_boolean(DESKTOP_GROUP, "Terminal", false);
        entry.set_boolean(DESKTOP_GROUP, "StartupNotify", false);
        entry.set_boolean(DESKTOP_GROUP, "X-GNOME-Autostart-enabled", true);
        entry.set_boolean(DESKTOP_GROUP, OWNER_KEY, true);
        std::fs::create_dir_all(&self.directory).map_err(|error| error.to_string())?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(&self.directory).map_err(|error| error.to_string())?;
        temporary
            .write_all(entry.to_data().as_bytes())
            .map_err(|error| error.to_string())?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|error| error.to_string())?;
        temporary
            .persist(destination)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn owned_entry(&self) -> Result<Option<glib::KeyFile>, String> {
        let path = self.directory.join(FILE_NAME);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        if !metadata.file_type().is_file() {
            return Err("The startup entry is not a regular file.".to_string());
        }
        let entry = glib::KeyFile::new();
        entry
            .load_from_file(path, glib::KeyFileFlags::NONE)
            .map_err(|error| error.to_string())?;
        if !entry.boolean(DESKTOP_GROUP, OWNER_KEY).unwrap_or(false) {
            return Err("An existing startup entry belongs to another application.".to_string());
        }
        Ok(Some(entry))
    }
}

// Desktop Exec quoting is distinct from shell quoting. KeyFile serializes the
// second escaping layer, and a doubled percent denotes a literal filename percent.
fn quote_exec(path: &Path) -> Result<String, String> {
    let text = path
        .to_str()
        .ok_or("The executable path is not valid UTF-8.")?;
    if text.contains(['\n', '\r']) {
        return Err("The executable path cannot contain a newline.".to_string());
    }
    let mut quoted = String::from("\"");
    for character in text.chars() {
        match character {
            '\\' | '"' | '$' | '`' => {
                quoted.push('\\');
                quoted.push(character);
            }
            '%' => quoted.push_str("%%"),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    Ok(quoted)
}
