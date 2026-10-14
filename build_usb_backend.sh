#!/usr/bin/env bash
# Module: magic_hotspot.build_usb_backend
# Purpose: Build a private USB mux backend with bounded endpoint recovery.
# Created: 2026-10-09
# Architecture: Compile vendored GPL upstream as a separate executable.
set -euo pipefail
PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$PROJECT_DIR/work/usb-backend/build"
mkdir -p "$BUILD_DIR" "$PROJECT_DIR/bin"
cp -a "$PROJECT_DIR/vendor/usbmuxd-1.1.1/src" "$BUILD_DIR/"
python3 "$PROJECT_DIR/usb_backend_build.py" "$BUILD_DIR"
cat > "$BUILD_DIR/config.h" <<'CONFIG'
#define HAVE_PPOLL 1
#define HAVE_STPCPY 1
#define HAVE_CLOCK_GETTIME 1
#define HAVE_LOCALTIME_R 1
#define HAVE_SYS_TYPES_H 1
#define PACKAGE_NAME "magic-hotspot-usb-engine"
#define PACKAGE_VERSION "1.1.1-magic1"
#define PACKAGE_STRING "Magic Hotspot private USB backend"
#define PACKAGE_URL "https://libimobiledevice.org"
#define PACKAGE_BUGREPORT "https://github.com/libimobiledevice/usbmuxd/issues"
CONFIG
if pkg-config --exists libusb-1.0 libplist-2.0; then
    read -r -a SDK_FLAGS <<< "$(pkg-config --cflags libusb-1.0 libplist-2.0)"
    read -r -a LINK_FLAGS <<< "$(pkg-config --libs libusb-1.0 libplist-2.0)"
else
    # Locally extracted, APT-verified Ubuntu headers permit a build without installation.
    SDK_DIR="$PROJECT_DIR/work/usb-backend/sdk/usr/include"
    [[ -f "$SDK_DIR/libusb-1.0/libusb.h" && -f "$SDK_DIR/plist/plist.h" ]] || {
        printf '%s\n' 'Install libusb-1.0-0-dev and libplist-dev to build the USB backend.' >&2
        exit 1
    }
    SDK_FLAGS=(-I"$SDK_DIR" -I"$SDK_DIR/libusb-1.0")
    LINK_FLAGS=(-l:libusb-1.0.so.0 -l:libplist-2.0.so.4)
fi
cc -O2 -fstack-protector-strong -D_FORTIFY_SOURCE=2 -DHAVE_CONFIG_H \
    -I"$BUILD_DIR" -I"$BUILD_DIR/include" "${SDK_FLAGS[@]}" \
    "$BUILD_DIR"/src/{main,client,device,preflight,log,usb,utils,conf}.c \
    -Wl,-z,relro,-z,now "${LINK_FLAGS[@]}" -lpthread \
    -o "$PROJECT_DIR/bin/magic-hotspot-usb-engine"
