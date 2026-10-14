<!-- Module: magic_hotspot.history; Purpose: Succinct authored changes; Created: 2026-10-11; Architecture: Release record. -->

- 0.2.0: BlueZ phone/NAP discovery and desktop-agent pairing; reuse/create
  NetworkManager PAN profiles. Automatic visible-window device refresh;
  serialized connection actions. Replace menu/AppIndicator with direct tray
  activation below the icon, Xwayland positioning and Ctrl+Q. Rust 2024.
  29 Rust tests and isolated real tray click/discovery/placement pass;
  actual phone PAN traffic unverified. Ubuntu package/source delivered.

- 0.2.1: Position tray-opened windows after mapping so GNOME initial placement
  cannot recenter them; translate native XEmbed coordinates when GNOME supplies
  no pointer position. Use the desktop symbolic tray icon at its native size.
  Phone Connect menus expose USB, Wi-Fi and Bluetooth; unavailable choices
  are disabled without explanation text.

- 0.2.2: Login startup never maps a window; manual launches center it. Remove
  unavailable/empty adapter rows. White transparent tray glyph and right-click Quit.
  Verified before packaging: 29 Rust tests; delayed-tray startup, fresh/existing
  manual centering, tray click placement, right-click Quit and Ctrl+Q.

- 0.2.3: Restore the original blue/white application badge in the tray. Keep
  device controls enabled during scans; scan Wi-Fi/Bluetooth concurrently and
  move periodic connectivity checks off GTK. Do not replace active connection
  controls when a background scan finishes.
  Tested before packaging: 29 Rust tests and normal/20-second-scan UI checks,
  including right-click Quit during the scan. Stopped the stale development
  process and launched the installed application.

- 0.2.4: Restore the native Ubuntu/XFCE indicator and unchanged scalable badge.
  Open uses logical monitor workarea placement by the panel with an edge margin;
  manual launches center. Close always hides; login startup stays hidden.
  Removed legacy tray/pixbuf sizing and pointer-translation code.
  Before packaging: 29 Rust tests and isolated XFCE UI checks at 1×/2×,
  including Close/reopen and Quit during a 20-second scan. Inspected both renders.

- 0.2.5: Retain the anchor when the native tray menu opens; forward its real
  activation timestamp to GTK so Ubuntu focuses the window instead of requesting
  attention. Dock activation preserves position; only explicit launches center.
  User confirmed the corrected build opens visibly below the icon on Ubuntu.
  Isolated XFCE checks pass at 1×/2× with pointer movement after menu opening,
  dock restoration and a slow scan.
  Add connection progress/error popups for USB, Wi-Fi and Bluetooth, with a compact
  Close button. Remove redundant startup-switch messages and nmcli journal hints.
  Read unescaped Bluetooth addresses so retries reuse saved PAN profiles.
  Pairing/PAN creation succeeded in the observed failure, but the link dropped
  before DHCP completed; that transport failure is not claimed fixed.

- 0.2.6: Restore a compact, centered, rounded Close button in connection popups
  and explicitly show an amber warning icon on failure. Retain error spacing.
  Bluetooth investigation: pairing remains saved; PAN closes before DHCP,
  followed by separate radio timeouts. No connection fix claimed.
  Verified 29 Rust tests and isolated popup/tray checks at 100%/200%; visually
  inspected both, including Ubuntu Yaru.

- 0.2.7: Lower the warning icon slightly and add space above the error message
  and centered Close button.
  Visually checked at 200% with Yaru; popup and tray checks pass.

- 0.2.8: Use a small GNOME 50 companion for actual icon/frame/workarea placement
  on Wayland; stop using stale Xwayland pointer positions there. Retain XFCE/X11
  positioning. Package the companion; enabling it needs a new GNOME session.
  Tethering comparison: IPv4-only still closes after about five seconds, before
  DHCP completes; captured Discover checksums/lengths are valid. Temporary
  profile removed. Installed iPhone diagnostic utilities; no USB cable available.
  Verified native GNOME menu Open and visual placement at 100%/200%, XFCE
  tray/popup at 200%, and 29 Rust tests. Companion remains pending next login.

- 0.2.9: Remove the connection spinner and compact its pending state. Keep a larger
  warning icon and lower error text/button in the failure state.
  Remove the redundant manual iPhone profile/card; use detected-device actions.
  Automatic switching retries the selected device/transport during this session;
  retain USB-only discovery when Bluetooth is unavailable. Reduce window height.
  Verified 29 Rust tests, popup/tray checks and renders at 100%/200%, and native
  GNOME tray placement at 200%. Current desktop companion still needs next login.

- 0.2.10: Apply the approved layout to connection progress, connection failures
  and open-Wi-Fi confirmation. Shared 64px SVG icons and vertical alignment;
  112×40 buttons centered under the text. Remove duplicate body headings; failure title is
  “Connection failed”. Keep network behavior unchanged.
  Verified 29 Rust tests, GTK interaction checks at 100%/200%, and all three
  native GNOME dialog layouts at 200%, including text-centered action groups.

- 0.2.11: Remove forced centering from foreground launches and dock activation.
  Enable the packaged GNOME companion without replacing other extension settings.
  Retry placement while the panel icon maps and report missing integration instead
  of silently ignoring it. First-time GNOME discovery still requires a new login.
  Verified 29 Rust tests, native GNOME placement at 100%/200% for Open,
  dock activation and foreground launches, and XFCE tray lifecycle checks.

- 0.2.12: Make quiet tray startup the application and desktop launcher default.
  Keep --background compatible with existing login entries; provide --show for
  explicit foreground launch. Disable launcher startup notification since no
  window is mapped. Login startup remains controlled by its existing switch.
  Verified 29 Rust tests and native GNOME/XFCE startup, repeated launch,
  Open, dock restoration and Quit checks; visually inspected tray-only startup.

- 0.2.13: Clear GTK’s automatic first-control focus when opening the hidden
  window. Keep keyboard navigation and visible keyboard focus available.
  Visually verified Yaru on GNOME/XFCE: no initial highlight; Tab restores
  the normal focus outline. Tray startup and Open lifecycle checks pass.

- 0.2.14: Complete the desktop startup notification even when no window maps.
  Keep discovery on workers; disable device actions during refresh and show a
  compact loading dialog only for an explicit refresh while the app is focused.
  Automatic/background scans do not open dialogs or change the cursor.
  Remove the idle “Select a device to connect” footer.
  Verified 29 Rust tests, focused refresh feedback/dismissal with a delayed
  worker, and tray lifecycle. GNOME reported no pending launch sequence; the
  original intermittent busy cursor was not reproduced in the isolated check.

## 0.2.15 — 2026-10-11

Show live Connected (mode) labels and adjacent Disconnect controls; enable Disconnect only for active links. Explicit transport selection temporarily prefers that link and restores the previous app-selected route settings. Preserve other active links as fallback after disconnect. Move settings below connections, hide duplicate success messages, and left-align dialog text next to its icon. Place Rust module tests in corresponding test folders.

## 0.2.16 — 13 October 2026

Removed the automatic-switch row and its connectivity retry loop. Added Chop
Chop's flag picker in the titlebar's left position. The picker offers English, Brazilian Portuguese, Spanish and Greek, with
live labels, translated dialogs/tray actions, saved selection and system-locale
default. Language changes preserve the current connection and busy state.

Kept one titlebar: language flag on the left, Magic Hotspot centered, and native
window controls on the right. Removed the titlebar Wi-Fi icon. Restored Refresh
to the right of the Nearby devices heading inside the device card. The tray keeps
the blue Wi-Fi application icon.


## 0.2.17 — 13 October 2026

Integrated magic-usb and its shared CSR client into the Magic Hotspot package.
Kept transport source in magic-usb/ with its own library, tests and build script.
Updated release build, cleanup, ELF dependency derivation, GPL documentation,
service removal/upgrade handling and full source archiving. The unified package
replaces the old standalone magic-usb package. Existing service names, selected
ports, btusb handoff and original CSR executable remain compatible.


## 0.2.18 — 13 October 2026

Give Refresh, Connect and Disconnect visible button surfaces and borders.
Connect uses the app's blue accent; secondary actions use white surfaces.
Add hover, pressed, keyboard-focus and disabled styling. Preserve existing
connection handlers, sensitivity, labels, translations, layout and tray behavior.
UI-only package upgrades preserve the running Bluetooth transport when its
executable is unchanged.


## 0.2.19 — 13 October 2026

Align Refresh and Disconnect as a shared right-hand button column. Match their
widths through GTK size grouping, and give the card heading and all device rows
the same inset. Keep Connect beside Disconnect and preserve dynamic labels and
four-language sizing. The integrated Bluetooth transport remains unchanged.


## 0.2.20 — 13 October 2026

Remove Refresh and its manual scan dialog; automatic device scans and two-second
connection/USB monitoring continue. Replace the button-based scan guard with
shared scan state so automatic requests cannot overlap. Keep Connect and
Disconnect aligned and color the Nearby devices Wi-Fi icon blue.


## 0.2.21 — 13 October 2026

Add 16 logical pixels above the connection list, below the Nearby devices heading,
for more breathing room. Keep button alignment, automatic monitoring and the
integrated Bluetooth transport unchanged.

## 0.2.22 — 13 October 2026

Allow 90 seconds for Bluetooth PAN activation. Observe the selected NetworkManager
connection's IP setup and retry once only after a qualifying pre-IP failure.
Repeated failures show four-language Personal Hotspot/restart guidance. Preserve
manual disconnects and unrelated errors; no adapter, BlueZ or pairing reset.
UB500 Bluetooth Internet was verified over IPv4/IPv6 after restarting the iPhone.

- 0.2.23: magic-usb 0.1.2 and shared CSR client 1.1.1 read validated cached USB descriptors before ownership transfer/reset. The selected-port service waits through unplugging and unsupported devices, with restart backoff instead of permanent start-limit failure.
