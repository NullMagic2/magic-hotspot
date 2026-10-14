// Module: magic_hotspot.tests.autostart
// Purpose: Verify safe startup file ownership, persistence and desktop Exec quoting.
// Created: 2026-10-09
// Architecture: Exercises autostart.rs using private temporary directories only.

#[path = "../../src/autostart.rs"]
mod autostart;

use autostart::Autostart;

#[test]
fn startup_entry_can_be_enabled_and_disabled_without_removing_other_files() {
    let directory = tempfile::tempdir().unwrap();
    let unrelated = directory.path().join("other.desktop");
    std::fs::write(&unrelated, "keep this").unwrap();
    let startup = Autostart::at(directory.path().to_path_buf(), "/tmp/Magic Hotspot".into());
    assert!(!startup.is_enabled().unwrap());
    startup.set_enabled(true).unwrap();
    assert!(startup.is_enabled().unwrap());
    let restored = Autostart::at(directory.path().to_path_buf(), "/tmp/Magic Hotspot".into());
    assert!(restored.is_enabled().unwrap());
    restored.set_enabled(false).unwrap();
    assert!(!startup.is_enabled().unwrap());
    assert_eq!(std::fs::read_to_string(unrelated).unwrap(), "keep this");
}

#[test]
fn unrelated_startup_entry_is_never_overwritten_or_removed() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("magic-hotspot.desktop");
    let content = "[Desktop Entry]\nType=Application\nName=Other app\nExec=other\n";
    std::fs::write(&file, content).unwrap();
    let startup = Autostart::at(directory.path().to_path_buf(), "/tmp/magic-hotspot".into());
    assert!(startup.set_enabled(true).is_err());
    assert!(startup.set_enabled(false).is_err());
    assert_eq!(std::fs::read_to_string(file).unwrap(), content);
}

#[test]
fn symbolic_link_startup_entry_is_never_followed() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("private.txt");
    std::fs::write(&target, "keep private").unwrap();
    std::os::unix::fs::symlink(&target, directory.path().join("magic-hotspot.desktop")).unwrap();
    let startup = Autostart::at(directory.path().to_path_buf(), "/tmp/magic-hotspot".into());
    assert!(startup.set_enabled(true).is_err());
    assert!(startup.set_enabled(false).is_err());
    assert_eq!(std::fs::read_to_string(target).unwrap(), "keep private");
}

#[test]
fn desktop_exec_quotes_special_characters_without_a_shell() {
    let directory = tempfile::tempdir().unwrap();
    let path = "/tmp/Magic % Hotspot $ \" \\ ` executable";
    let startup = Autostart::at(directory.path().to_path_buf(), path.into());
    startup.set_enabled(true).unwrap();
    let entry = glib::KeyFile::new();
    entry
        .load_from_file(
            directory.path().join("magic-hotspot.desktop"),
            glib::KeyFileFlags::NONE,
        )
        .unwrap();
    let exec = entry.string("Desktop Entry", "Exec").unwrap();
    let args = glib::shell_parse_argv(exec.as_str()).unwrap();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0].to_str().unwrap(), path.replace('%', "%%"));
    assert_eq!(args[1], "--background");
}

#[test]
fn newline_in_executable_path_cannot_inject_a_desktop_key() {
    let directory = tempfile::tempdir().unwrap();
    let startup = Autostart::at(
        directory.path().to_path_buf(),
        "/tmp/app\nExec=other".into(),
    );
    assert!(startup.set_enabled(true).is_err());
    assert!(!directory.path().join("magic-hotspot.desktop").exists());
}

#[test]
fn current_configuration_can_be_resolved_without_writing_it() {
    assert!(Autostart::current().is_ok());
}
