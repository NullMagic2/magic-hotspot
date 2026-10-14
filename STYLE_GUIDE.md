<!--
Module: iphone_hotspot_widget.documentation.style_guide
Purpose: Define clear module boundaries, safe networking and readable Rust.
Created: 2026-10-09
Architecture: Adapted from Hex-TK's style guide for this GTK3 application.
-->

# Magic Hotspot style guide

## Modules and boundaries

Keep each module responsible for one coherent concept and give it a small interface.
Use Rust 2024. Keep GTK presentation in main.rs, NetworkManager commands in network.rs,
Bluetooth PAN discovery/pairing in bluetooth.rs, bounded PAN recovery in pan.rs and tray
integration in tray.rs, window restoration in window.rs and login startup files
in autostart.rs.
Native GTK3 tray calls require documented ownership and
main-thread invariants. Use Ayatana’s native Ubuntu/XFCE indicator and let the panel scale the SVG.
Use descriptive types for external states, and keep process argument construction explicit.
Avoid global mutable state and avoid shell command strings; pass each argument separately.

The application is Rust and GTK3, using the gtk-rs GTK3 bindings as its interface layer.
NetworkManager owns system network behavior and saved credentials; nmcli is the supported
integration boundary. Never read, copy, pass on command lines or log Wi-Fi passwords.
Run only the requested nmcli operation and show its returned error to the user.
Identify iPhone USB tethering through its ipheth driver, never an interface name
or Ethernet type alone. Preserve Apple's normal Trust and pairing validation.
The GUI uses only the installed USB recovery service for privileged application
operations. The integrated magic-usb transport runs as an administrator-selected
system service, scoped to that adapter; keep its source in magic-usb/ and preserve
its conditional ownership restoration. Ship all components in one Debian package.
Keep its interface fixed: no user-provided paths, commands, identifiers or secrets.
Limit authorization to the exact unit and operation for active local users.
Keep its mux socket and cleanup state root-only, bound retries and execution time,
and restore Ubuntu's service and driver after failures. Never weaken Trust or TLS.
Preserve the original failure when cleanup also fails. Report operation and errno
without private paths or identifiers. Observe USB reattachment from fresh bus
metadata without reading phone serials; refresh status off the GTK main thread,
serialize polls and pause them while a connection operation is running.
Run scans and hotspot activation off the GTK main thread. Keep titlebar controls
native, with content button styling scoped to the page.
Read current NetworkManager connectivity without forcing outbound probes, and clearly label
unencrypted Wi-Fi with confirmation before connecting.
Use the official Microsoft Fluent System Icons assets in assets/fluent/; keep their
MIT license alongside them and embed them so the executable stays self-contained.

## Reusable values and failure handling

Define shared UI dimensions and polling intervals as named constants in their owner.
Represent NetworkManager connectivity states explicitly and handle unknown values.
Validate empty profile names before invoking system commands.
Use bounded NetworkManager waits and present actionable failures in the status label.
Hide a minimized window only when its tray icon is accessible. Unmap it and set
the taskbar/pager hints; clear them and deiconify when restoring. Normal and automatic startup stay hidden while the tray host starts. Explicit --show, dock activation and native tray Open place it by the panel with a workarea margin.
Keep the native menu limited to Open and Quit.
Keep login startup opt-in. Change only the application's owned user autostart entry,
reject symbolic links and unrelated entries, and use XDG desktop Exec quoting.
Never use a shell to launch the autostart executable.

## Formatting and module comments

Use four-space Rust indentation, blank lines between declarations and readable functions.
Every handwritten source file begins with Module, Purpose, Created and Architecture comments.
Comments explain ownership, boundaries and non-obvious behavior; they do not repeat code.
Keep tests in the corresponding tests/<module>/ folder, outside production source.
Production modules may only reference their test modules with a cfg(test) path.
Keep the architecture tree synchronized when adding or removing source modules.

## Build and cleanup

Use build.sh for the release executable under bin/ and Ubuntu 26.04 .deb under outputs/.
Keep packaging in package.sh and launcher metadata under packaging/.
Derive ELF library requirements with dpkg-shlibdeps and include runtime-loaded components.
Stage as an ordinary user, assign root ownership in the archive and remove staging files.
Retain upstream USB backend licenses and corresponding source with its modifications.
Use clean_build.sh to delete Cargo intermediates while retaining the executable and .deb.
Do not put credentials, local connection settings or generated build files in source control.
