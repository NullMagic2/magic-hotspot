# USB protocol source

`usbmuxd-1.1.1/` is the unmodified upstream tag 1.1.1 from
https://github.com/libimobiledevice/usbmuxd.

Downloaded archive: https://codeload.github.com/libimobiledevice/usbmuxd/tar.gz/refs/tags/1.1.1
SHA-256: `e7ce30143e69d77fc5aa6fb0cb5f0cfcdbeff47eb1ac7fd90ac259a90de9fadd`

Retain the upstream copyright notices and COPYING.GPLv2/COPYING.GPLv3.
`usb_backend_build.py` applies Magic Hotspot modifications to a temporary build
copy; `build_usb_backend.sh` compiles that copy as a separate GPL executable.
The GTK frontend communicates with a fixed systemd helper rather than linking
the GPL protocol engine into the Rust executable.
