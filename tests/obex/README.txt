Virtual OBEX file transfer test (2026-10-13)

Requirements: Ubuntu BlueZ obexd, btvirt from bluez-test-tools, Python3 GI,
Linux VHCI, and root privileges for two temporary virtual controllers.

Run under a private session bus:
  sudo dbus-run-session -- python3 tests/obex/simulate.py \
    --btvirt /absolute/path/to/btvirt --output /absolute/path/to/test-output

Creates two virtual adapters; configures only those new adapters. Bluetoothd
remains running. Pairing confirmation is restricted to their indices and peer
addresses. OBEX runs on a private bus with no access to the desktop session.
Synthetic files are sent through the real BlueZ OBEX client, kernel RFCOMM and
L2CAP, and btvirt to a minimal Python OBEX receiver. Both adapter directions,
0/127/1048576-byte payloads, completion signals, exact bytes, SHA-256, and UTF-16
filenames are checked. Child processes/controllers are removed in finally.

Scope: tests outgoing Object Push in both virtual directions, not BlueZ's
incoming-file authorization UI, SDP discovery, iPhone compatibility, radio,
physical USB, or magic-usb's USB/VHCI relay. No production transport is changed.
The explicit RFCOMM channel bypasses SDP. Only synthetic test data is sent.
