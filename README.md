# Magic Hotspot

A compact Rust/GTK3 app for connecting Linux to an iPhone hotspot over USB,
saved Wi-Fi or Bluetooth PAN. **Current release: 0.2.24.**

- Nearby-device discovery with Connect and Disconnect controls.
- Blue Wi-Fi tray icon and optional login startup.
- English, Portuguese, Spanish and Greek; remembers the selected language and
  uses the system locale initially.
- Bounded Bluetooth hotspot recovery when IP setup fails.
- Integrated `magic-usb/` transport for CSR8510 and TP-Link UB500, with graceful
  adapter swaps and conditional `btusb` restoration.

NetworkManager owns connections and credentials; BlueZ owns Bluetooth pairing
and profiles. The iPhone must have Cellular Data and Personal Hotspot enabled.

## Install

Download the `.deb` from [Releases](https://github.com/NullMagic2/magic-hotspot/releases).
Packages target **Ubuntu 24.04/26.04, Debian 13, and Mint 22.x amd64**.
See [PACKAGING.md](PACKAGING.md) for build environments and test scope.

```sh
sudo apt install ./magic-hotspot_0.2.24-1ubuntu26.04_amd64.deb
magic-hotspot --show
```

The single package includes the app, iPhone USB helper, `magic-usb` 0.1.3 and
shared CSR client 1.1.2. Driver service startup remains an explicit choice.

## Build from source

Install Rust/Cargo and the GTK3, libusb, libplist and packaging build tools:

```sh
sudo apt install build-essential pkg-config libgtk-3-dev libusb-1.0-0-dev libplist-dev libzstd-dev dpkg-dev python3
cargo fetch --locked
./build.sh
```

Use a current Rust toolchain with edition 2024 support. Outputs are written to
`bin/` and `outputs/`. The vendored USB backend and shared driver source are
included, so no Git submodule setup is needed.

## Tests

```sh
cargo test --locked
./magic-usb/build.sh
```

The source includes isolated UI, PAN and OBEX simulation tests under `tests/`.
OBEX simulation instructions are in [tests/obex/README.txt](tests/obex/README.txt).

40 app unit tests, 6 autostart tests and 46 driver tests passed. Physical CSR
initialization and CSR-to-UB500 swapping were verified. UB500 Bluetooth PAN
traffic was verified in the development environment. These checks do not imply
all phone models, distro versions, audio profiles or physical OBEX transfers
have been validated.

## Driver repositories

The complete driver snapshot stays in `magic-usb/` for integrated builds.
Adapter-focused repositories are [CSR8510](https://github.com/NullMagic2/csr8510)
and [TP-Link UB500](https://github.com/NullMagic2/tp-link-ub500).

For detailed implementation notes, see [ARCHITECTURE.md](ARCHITECTURE.md) and
[the development guide](docs/DEVELOPMENT.md).

## Licenses

Magic Hotspot's own application code is **MIT licensed**; see [LICENSE](LICENSE).
Third-party components and the existing driver retain their licenses:

- `magic-usb/`: MIT, including the shared CSR client.
- `vendor/usbmuxd-1.1.2/`: upstream GPL-2.0-or-3.0 terms; see its COPYING files.
- `assets/fluent/`: Microsoft Fluent System Icons, MIT.

The GPL USB engine is built as a separate executable. All bundled corresponding
source and notices are included.

Distribution builds and standalone driver package details: [PACKAGING.md](PACKAGING.md).
