<!--
Module: iphone_hotspot_widget.documentation.architecture
Purpose: Describe the GTK3 hotspot helper and its boundaries.
Created: 2026-10-09
Architecture: Adapted from Hex-TK's architecture document for this application.
-->

## OVERVIEW

Magic Hotspot is a GTK3 tray application with a compact iPhone connection button.
NetworkManager owns saved credentials, association, routing and connectivity status.
The app never reads or stores passwords, and embeds Microsoft's official Fluent icons.
The user's iPhone must have Personal Hotspot enabled. Wireless use needs a saved
Wi-Fi profile; USB use needs a data-capable cable and the user's trust approval.
i18n.rs owns live text bindings and the saved four-language preference.
language.rs presents the Chop Chop flag picker without network operations.
network.rs recognizes USB tethering by the Linux ipheth driver. An explicit Connect action prefers a ready USB link, then the configured Wi-Fi profile.
An unavailable USB link without Wi-Fi can start the installed fixed USB helper.
The helper uses Apple's normal pairing handshake and never requests phone files.
Activation runs on Gio's worker pool.

packaging/usb_helper.py runs through the on-demand magic-hotspot-usb system service.
Polkit permits only starting that specific unit for an active local desktop user.
The helper accepts no caller-supplied paths, devices, commands or credentials.
It identifies exactly one Apple interface bound to ipheth, checks NetworkManager
ownership, records cleanup state in a root-only directory, temporarily masks and
stops usbmuxd, detaches ipheth, authenticates via a private mux socket, rebinds
ipheth and restores the original usbmuxd service state. ExecStopPost repeats
cleanup after interruptions. Public status contains controlled messages only.
The USB engine is a separate GPL executable built from vendored usbmuxd 1.1.1.
usb_backend_build.py applies address filtering, root-only runtime socket paths,
initial endpoint clearing and three bounded receive-stall recoveries outside
libusb callbacks. libimobiledevice supplies the normal Trust and TLS handshake.
The engine creates its root-only socket and then drops to Ubuntu's usbmux account;
pairing storage keeps Ubuntu's normal ownership. The controller holds only the
capabilities needed for interface control, protected-file access, that drop and
signaling its child afterward. Cleanup errors preserve the connection error.
Public OS errors report the operation and errno rather than private paths.
NetworkManager still owns IP configuration, routing, Wi-Fi credentials and VPNs.

usb.rs observes cable generations from USB bus metadata without reading serials.
Every two seconds a worker reads USB metadata and NetworkManager status, including
while in the tray. The GTK thread updates the hint, and actual attach/detach events
also update the status line. There is only one poll in flight; polls pause while
connecting. The same-port replug is distinguished by its new USB device address.
Activation follows explicit connection actions; there is no automatic-switch retry loop.

The native Ubuntu/XFCE indicator exports the original SVG and Open / Quit menu
through Ayatana. The desktop owns icon scaling and menu input. On GNOME Wayland,
gnome/extension.js positions the native window using the real indicator actor,
window frame and monitor workarea. The application calls it asynchronously.
XFCE/X11 retains the pointer anchor captured when the native menu opens.
Both paths keep a 12-logical-pixel margin. Missing GNOME integration preserves
the current position rather than using stale Xwayland coordinates. Native menu activation
forwards its click timestamp to GTK for normal window-manager focus approval.
Normal command-line/desktop launches stay hidden in the tray by default.
Activation explicitly completes the desktop startup notification without mapping a window.
Explicit --show and dock activation request native tray placement on GNOME;
X11 restoration retains the last placement. The application enables its companion
in the user’s extension list without changing other entries and reports placement errors.
Close hides the window; Minimize hides it when the tray is connected. Ctrl+Q exits.
window.rs clears taskbar/pager hints when restoring and observes native minimize
controls. GNOME 50 needs AppIndicator support and the packaged tray-placement
companion enabled after signing out and back in. XFCE requires no companion.

The app does not force connectivity probes; open Wi-Fi requires confirmation.
Wi-Fi discovery first checks NetworkManager's adapters and radio state, then
explicitly scans each usable Wi-Fi interface. A missing or unavailable adapter
is an error rather than a successful empty scan. nmcli uses the C locale so
machine-readable device and connectivity states are stable across desktop languages.
The scan parser is exercised with simulated responses in network.rs unit tests.
Scans run on Gio's worker pool; GTK applies the result on its main thread and
serializes scans with shared in-flight state until completion. The window uses GTK's native HeaderBar and
an explicit decoration layout with the titlebar app icon omitted. Content button CSS is scoped to the page so native
window controls retain the desktop theme.
Tray native calls are isolated in tray.rs and operate only on the GTK main thread.
The tray exports the unchanged embedded SVG from a private temporary directory.
assets/magic-hotspot.svg wraps the original Fluent geometry with a white background
and supplies the launcher, task-switcher and tray icons. The language flag is
the titlebar picker. The blue Nearby devices Wi-Fi icon heads the device card;
Connect and Disconnect are its only action buttons, with automatic device scans.

Login startup is off by default. The window owns the startup toggle.
autostart.rs writes an owned XDG autostart entry in the user's configuration directory,
using an atomic replacement and a quoted executable path. Disabling removes only that
owned entry. Startup uses --background and stays hidden while the tray host starts.
Keep the executable at the same path after enabling startup.

The Ubuntu 26.04 package installs the binary under /usr/bin and registers a launcher
whose desktop ID matches the GTK application ID. package.sh derives shared-library
requirements from the ELF file and explicitly includes NetworkManager, BlueZ
the SVG loader, usbmuxd and ipheth-utils for Ubuntu's USB tethering support.
It stages files without root, creates a root-owned .deb and
removes its staging directory. Maintainer scripts reload systemd on installation
and stop the recovery service on removal; installation never starts USB recovery.
Autostart remains a user preference.

magic-usb/ is a separate Rust workspace within this source tree. It owns the
shared libusb transport, Realtek firmware parser, CSR compatibility client, VHCI
bridge and root-only ownership journal. It remains a separate system-service
process but is installed by the same Magic Hotspot package. The existing
magic-usb@PORT service and command paths remain compatible. It transfers only
that selected adapter's interfaces and restores btusb only when previously bound;
it does not stop BlueZ or alter other adapters. The CSR client is installed under
/usr/libexec/magic-usb/csr8510 without changing the original /usr/bin/csr8510.

The unified package Conflicts/Replaces the former standalone magic-usb package.
All four ELF clients participate in dependency derivation; the transport GPL
license and documentation ship under /usr/share/doc/magic-hotspot/magic-usb/.
On upgrades postinst restarts active instances only when their executable changed, never
starts an inactive adapter, and never enables boot startup. Removal stops those
instances before removing their executable so ExecStopPost can restore recorded
bindings. The source archive contains both workspaces and excludes generated or
local runtime state.

## TREE

```text
magic-hotspot/
├── ARCHITECTURE.md      This overview and annotated source tree
├── STYLE_GUIDE.md       Adapted coding and documentation rules
├── Cargo.toml           GTK3 Rust application manifest
├── build.sh             Build the release executable and Ubuntu 26.04 .deb
├── package.sh           Stage all clients, derive dependencies and create one package
├── archive_source.py    Save complete source ZIP, including magic-usb/
├── build_usb_backend.sh Build the private USB engine as a separate executable
├── usb_backend_build.py Apply recovery/isolation changes to a build copy
├── vendor/usbmuxd-1.1.1/ Upstream protocol source and GPL licenses
├── clean_build.sh       Remove Cargo intermediates while preserving bin/
├── bin/                 GUI and iPhone USB engine, retained across clean_build.sh
├── magic-usb/           Integrated transport source in its own Rust workspace
│   ├── src/             USB/VHCI bridge, firmware, ownership and shared library
│   ├── clients/csr8510/ CSR client using the same library
│   ├── tests/           Transport/firmware/ownership unit tests
│   ├── packaging/       magic-usb@.service installed by the parent package
│   ├── COPYING          MIT license
│   └── build.sh         Test and build clients; no separate .deb
├── assets/fluent/       Embedded official Microsoft Fluent System Icons and MIT license
├── assets/dialog-information.svg  Approved information artwork, shared 64px viewport
├── assets/dialog-warning.svg      Approved warning artwork, shared 64px viewport
├── assets/magic-hotspot.svg  Official Wi-Fi glyph on a white app/tray badge
├── packaging/          Launcher, fixed USB helper, systemd unit and Polkit rule
├── gnome/              GNOME 50 companion, packaged only for native tray placement
│   ├── metadata.json    Extension identity and supported Shell version
│   └── extension.js     Scoped D-Bus placement using compositor geometry
├── outputs/            Installable .deb and user-facing previews and archives
├── tests/
│   ├── packaging/check.py Unified payload, migration metadata and clean source builds
│   ├── connections/     Live identity, switching rollback and scoped disconnect tests
│   ├── network/         USB and Wi-Fi NetworkManager boundary tests
│   ├── usb/             Attachment metadata tests
│   ├── autostart/main.rs     Isolated startup-file safety and desktop-command tests
│   ├── bluetooth/mod.rs Candidate/service filtering and PAN profile tests
│   ├── gnome/smoke.sh   Isolated native GNOME Wayland placement and screenshot check
│   ├── gnome/probe.js   Test-only geometry/screenshot wrapper, never installed
│   └── ui/              GTK control state and isolated Xvfb/XFCE smoke tests
│       ├── controls.rs  Native Connected (mode) and Disconnect sensitivity tests
│       └── smoke.py          Isolated Xvfb/XFCE tray placement and fake-device discovery
└── src/
    ├── main.rs          Compact GTK window, controls, lifecycle and Wi-Fi list
    ├── window.rs        Minimize-to-tray, dock visibility and window restoration
    ├── connection_dialog.rs Connection progress and readable failure popup
    ├── connections.rs   Live transport identity, route preference and disconnect
    ├── network.rs       NetworkManager scan, connectivity query and activation
    ├── i18n.rs          Live translations and saved locale choice
    ├── language.rs      Left-side flag picker and language popover
    ├── usb.rs           Observe USB attachment generations and carrier metadata
    ├── autostart.rs     Optional user login entry and safe desktop Exec serialization
    ├── bluetooth.rs     BlueZ discovery/pairing; NetworkManager PAN profile activation
    └── tray.rs          Native Ayatana SVG indicator and Open / Quit menu
```

Icon source: [Microsoft Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons),
icons `WiFi 1/SVG/ic_fluent_wifi_1_24_regular.svg` and
`Phone/SVG/ic_fluent_phone_32_regular.svg`.

Runtime tray support uses libayatana-appindicator3.
XFCE uses its Status Tray panel plugin; no desktop-specific implementation is needed.
On GNOME, enable [AppIndicator and KStatusNotifierItem Support](https://extensions.gnome.org/extension/615/appindicator-support/).

Bluetooth scans and activation run off the GTK thread. Scans own bounded BlueZ
discovery sessions and display NAP services/phone-class candidates. Pairing is
confirmed by the desktop agent. NetworkManager owns PAN profiles and IP routing.
The existing connection guard serializes user actions and pauses refresh during
activation; window-visible device refresh runs every 30 seconds.

Connections share one worker-backed action and remember the selected transport for route preference and live row status. Discovered devices replace the manual profile field.
Connection actions and open-Wi-Fi confirmation share a GTK dialog layout with
64px scalable icons, vertically centered messages and 112×40 buttons centered beneath the message column.
The window title carries the status; there is no duplicate body heading or spinner. Success closes
it; failure shows a concise error with a compact Close button. Native library
diagnostics stay out of the main window. PAN profile queries disable nmcli address
escaping so retries reuse an existing Bluetooth profile.

PAN recovery lives in pan.rs. A bounded worker observes NetworkManager device
state and the active profile UUID during nmcli's 90-second activation wait.
Only observed IP configuration followed by a Bluetooth/IP setup failure while
disconnected qualifies for one retry. Unknown observations, a different profile,
manual interruption and completed IP setup suppress retries. The original error
is preserved; repeated qualifying failures add localized recovery guidance.

USB hotplug: service startup waits for two matching enumeration samples on the selected port. Removal restarts into this wait; genuine startup errors back off from 3 to 30 seconds without a permanent restart limit. Cached sysfs configuration is checked against the opened descriptor before claiming, allowing CSR reset even when live descriptor requests stall.
