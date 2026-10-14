# Magic Hotspot

Compact Rust GTK3 hotspot helper with Microsoft's official Fluent icons.

1. Enable Personal Hotspot on your iPhone.
2. Connect the iPhone with a data-capable USB cable, unlock it and accept **Trust**
   if prompted. For wireless use, connect once using your desktop's Wi-Fi settings.
3. Start **Magic Hotspot**, choose **Open** from its tray icon, then choose a device and **Connect**. Phone menus
   offer USB, Wi-Fi and Bluetooth; unavailable options are disabled.
   A source build can also run `bin/magic-hotspot`.

The tray keeps the blue Wi-Fi application icon. In the popup, the left-hand language flag opens the same four-language picker as Chop Chop:
English, Português, Español and Ελληνικά. Selecting a language updates the UI
immediately and saves the choice for later launches. With no saved choice,
the app uses the desktop locale when supported, then English. Connect directly
activates the selected transport; the automatic-switch setting was removed.

Connection progress and failures appear in a small popup. Device scans run
automatically every 30 seconds while the window is visible; USB attachment and
connection state are checked every two seconds, including in the tray. Background
scans stay silent. There is no manual Refresh button.

The device list refreshes automatically while the window is visible. Bluetooth
phones and devices advertising NAP are shown alongside Wi-Fi. Pairing uses the
desktop Bluetooth confirmation prompt; Personal Hotspot must be enabled. The app
reuses a matching NetworkManager PAN profile or creates one without autoconnect.
PAN activation allows up to 90 seconds for network setup. If the selected PAN
reaches IP configuration but fails before completing it, the app retries once.
A repeated failure explains how to toggle Personal Hotspot or restart the phone.
Pairing, permission errors and manual disconnects are not retried.
A listed phone is a candidate, not proof that tethering is enabled. Discovery is
bounded and stops only this app’s sessions. Radios are not enabled automatically.

Nearby Wi-Fi uses saved connections. Passwords remain with NetworkManager.
Wi-Fi scanning requires a Wi-Fi adapter managed by NetworkManager; an Ethernet
connection does not appear in the nearby Wi-Fi list. Unavailable adapters add no rows. Scanning runs in the background so the window
remains responsive. USB attachment changes also refresh the device list. USB recognition uses the Linux `ipheth` driver, so an ordinary Ethernet port
is never treated as an iPhone. Connection activation also runs in the background.
Magic Hotspot has no telemetry and does not force outbound connectivity probes.
Open Wi-Fi requires confirmation. Network encryption and any VPN remain your
NetworkManager configuration; the app does not provide a VPN.

## Install on Ubuntu 26.04

Download the `.deb` for your architecture and run this from its directory:

```sh
sudo apt install ./magic-hotspot_0.2.22-1ubuntu26.04_amd64.deb
```

The supplied package is for amd64 (Intel/AMD 64-bit). It installs the application,
desktop launcher, official Fluent icon on a white badge and license notice. APT resolves GTK3,
NetworkManager, BlueZ, SVG-loader and iPhone USB tethering dependencies
(`usbmuxd` and `ipheth-utils`). It does not enable login
startup or change Wi-Fi settings during installation.

The package also installs an on-demand USB recovery service. When USB is inactive
and no Wi-Fi adapter is available, the device’s **USB** action runs that fixed operation.
An active local desktop user can start it without another administrator prompt;
remote and inactive sessions keep Ubuntu's normal authorization requirements.
The GTK app runs as your regular user. The helper briefly pauses `usbmuxd`, uses
a private backend with up to three receive-stall recoveries, performs Apple's
normal Trust handshake, restarts only the selected `ipheth` interface and restores
Ubuntu's service. Other phone USB applications may pause during that operation.
It does not request the hotspot password, reset phone settings, remove pairing
records or read phone files. Pairing keys remain in Ubuntu's lockdown storage.
The private protocol socket is root-only; the UI receives only a short status.
The service has bounded execution and cleanup after failure or interruption.
Version 0.1.4 fixes permission to stop the backend after it drops to the usbmux
account. Cleanup errors retain the original connection error, and filesystem
failures report their operation and errno without paths or identifiers.

USB status refreshes automatically every two seconds, including in the tray.
Disconnecting and reconnecting at the same port creates a new USB generation and
refreshes its status. Polling pauses during recovery and status refreshes again
after activation finishes. The decorative title-bar phone icon was removed.

USB Internet was verified on 13 October 2026 with the connected iPhone: both
USB-bound and ordinary HTTPS requests returned HTTP 200 after pairing recovered
and Personal Hotspot was enabled. The hotspot was subsequently disabled by the user.

## Integrated Bluetooth USB transport

Magic Hotspot 0.2.22 includes magic-usb and the shared CSR client in the same
Debian package. The complete source lives in the separate `magic-usb/` folder,
with its own Rust library, tests and workspace. No separate magic-usb package
is needed; installing the new package replaces the former standalone package.
The GUI and transport remain separate processes so closing the window does not
interrupt the adapter's service.

The transport handles the selected TP-Link UB 500 or CSR8510 adapter through
libusb and Linux VHCI, while BlueZ retains pairing and profiles. btusb stays
installed and continues to handle other adapters. Start only the chosen port:

```sh
sudo systemctl start magic-usb@1-4.service
sudo systemctl stop magic-usb@1-4.service
```

Stopping restores btusb only when it originally owned that adapter. If it was
inactive, the service only releases its own claims. Installation does not select
an adapter or enable startup; upgrades restart already-running instances only when the transport executable changes.
Ownership handoff can reset connections on that selected adapter. Details and
hardware validation limits are in `magic-usb/README.txt`. Bluetooth hotspot Internet
was verified on 13 October 2026 with UB500/magic-usb: after restarting the iPhone,
IPv4 and IPv6 HTTPS passed, including an in-app reconnect and route selection.
The same pre-restart failure occurred with stock btusb. Physical CSR hotspot use
and automatic recovery from interrupted cold firmware loading remain unverified.

## Ubuntu 26.04 build

Install Rust 1.85 or newer, Cargo, `libgtk-3-dev`, `pkg-config`, `dpkg-dev`,
`binutils`, `gzip`, a C compiler, Python 3, `libusb-1.0-0-dev` and `libplist-dev`.
Packaging derives library requirements on Ubuntu 26.04.
Runtime requirements are GTK3, NetworkManager (`nmcli`) and
`libayatana-appindicator3-1` for modern tray support.

```sh
./build.sh
./bin/magic-hotspot
./clean_build.sh
```

`build.sh` creates the GUI, iPhone USB engine and the two clients under
`magic-usb/bin/`, then saves one Magic Hotspot `.deb` and complete source ZIP in
`outputs/`. `package.sh` can repackage those existing executables without recompiling.
The source ZIP includes `magic-usb/` and its MIT license, tests and shared CSR code.
Native amd64 and arm64 packaging are supported; arm64 needs an arm64 Ubuntu 26.04 build.
Cleanup removes build intermediates and retains the executables, `.deb` and source.
The executable embeds its icons and can be moved before enabling login startup.
USB recovery requires the installed package's service. The separate backend uses
vendored usbmuxd 1.1.1 source (GPL-2.0-or-3.0); its corresponding source and build
modifications are included in the source archive.
XFCE supports its Status Tray panel plugin. GNOME requires the
[AppIndicator extension](https://extensions.gnome.org/extension/615/appindicator-support/).

## Checks

```sh
cargo test --locked --bin magic-hotspot --test autostart
cargo test --manifest-path magic-usb/Cargo.toml --locked --workspace --offline
python3 tests/packaging/check.py outputs/magic-hotspot_0.2.22-1ubuntu26.04_amd64.deb outputs/Magic-Hotspot-0.2.22-source.zip
```

Scan tests cover missing, disabled, unmanaged and unavailable adapters, scan errors,
multiple adapters and escaped SSIDs. They use simulated responses; physical Wi-Fi
discovery has not been validated. The runtime checks the adapter and radio before
scanning each usable interface, and omits unavailable adapters from the device list.
USB tests cover ready, active, unavailable and unmanaged iPhones, ordinary Ethernet,
saved Wi-Fi fallback and activation failures. They also use simulated responses.
All 24 Rust tests passed for version 0.1.4, including same-port reconnect, unrelated
USB device filtering, driver attachment and carrier changes. Separate C checks exercised receive
stalls, deferred endpoint clearing, receive-loop restoration, the retry limit and
clear-halt failure. Helper simulations covered authentication success, transport
failure, pending Trust and cleanup of the driver and Ubuntu service. A running
private backend passed socket-permission, address-filter and mux protocol checks.
Helper cleanup checks cover the required cross-user signaling capability, an
already exited backend, bounded kill escalation and sanitized operation errors.

Startup file tests and isolated tray integration checks passed, including an actual
XFCE panel. The Ubuntu package passed file checksum, launcher, shared-library and
APT install-simulation checks. The physical iPhone was detected through its USB
driver. End-to-end USB Internet access was verified on 13 October 2026 as
described above; Bluetooth hotspot Internet has not been verified.

Version 0.2.14 uses the native Ubuntu/XFCE indicator with the original scalable
blue/white badge. Tray Open retains its icon anchor and forwards the native
click timestamp so the window comes to the foreground. Automatic startup stays hidden. Scans and connectivity checks
run on workers. After updating, Quit the running copy and reopen Magic Hotspot.

On Ubuntu GNOME 50 Wayland, the package includes the native tray-placement
companion. After installing it, sign out and back in, then enable it with
`gnome-extensions enable magic-hotspot@local`. This places the window using the
actual panel icon and monitor geometry. Without it, Open preserves the window
position. XFCE continues using its native indicator and needs no companion.

Connection rows show `Connected (USB)`, `Connected (Wi-Fi)` or `Connected (Bluetooth)` only after NetworkManager activates the link. Selecting a mode makes it the preferred Internet route for the active session. Disconnect stops that device’s hotspot links; other active Ethernet or Wi-Fi links remain available as fallback. Saved connection profiles and passwords are unchanged. Settings appear below the connections.

Connect and Disconnect use visible bordered buttons. Connect uses the
blue accent; Disconnect uses a white surface, and disabled controls remain
grey. Hover, pressed and keyboard-focus states make the actions clear.

Adapter swaps: magic-usb waits for CSR8510 or UB500 on its selected USB port. Unplugging ends connections; a replacement controller is initialized automatically. Stopping the service still restores only the kernel interfaces it originally took over.
