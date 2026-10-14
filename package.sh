#!/usr/bin/env bash
# Module: magic_hotspot.packaging
# Purpose: Package the release executable for Ubuntu 26.04.
# Created: 2026-10-09
# Architecture: Stage files privately, derive ELF dependencies, emit a root-owned .deb.
set -euo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
APP_BINARY="$PROJECT_DIR/bin/magic-hotspot"
USB_BINARY="$PROJECT_DIR/bin/magic-hotspot-usb-engine"
TRANSPORT_BINARY="$PROJECT_DIR/magic-usb/bin/magic-usb"
CSR_BINARY="$PROJECT_DIR/magic-usb/bin/csr8510"

for tool in dpkg-deb dpkg-shlibdeps dpkg readelf gzip md5sum sha256sum; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'Packaging requires %s. Install dpkg-dev, binutils and gzip.\n' "$tool" >&2
        exit 1
    fi
done

# Symbol versions must come from the release this package targets.
if ! [ -r /etc/os-release ]; then
    printf '%s\n' 'Package on Ubuntu 26.04 so its library dependencies match the target.' >&2
    exit 1
fi
source /etc/os-release
if [[ "${ID:-}" != ubuntu || "${VERSION_ID:-}" != 26.04 ]]; then
    printf '%s\n' 'Package on Ubuntu 26.04 so its library dependencies match the target.' >&2
    exit 1
fi
if [[ ! -x "$APP_BINARY" || ! -x "$USB_BINARY" || ! -x "$TRANSPORT_BINARY" || ! -x "$CSR_BINARY" ]]; then
    printf '%s\n' 'Build the executable first with ./build.sh.' >&2
    exit 1
fi

APP_VERSION="$(awk -F '"' '/^version = "/ { print $2; exit }' "$PROJECT_DIR/Cargo.toml")"
DEB_VERSION="${APP_VERSION}-1ubuntu26.04"
dpkg --validate-version "$DEB_VERSION"
DEB_ARCH="$(dpkg --print-architecture)"
ELF_MACHINE="$(LC_ALL=C readelf -h "$APP_BINARY" | awk -F ': *' '/Machine:/ { print $2 }')"
case "$DEB_ARCH:$ELF_MACHINE" in
    amd64:*X86-64*|arm64:*AArch64*) ;;
    *) printf 'Unsupported binary/host architecture: %s / %s\n' "$DEB_ARCH" "$ELF_MACHINE" >&2; exit 1 ;;
esac

mkdir -p "$PROJECT_DIR/work" "$PROJECT_DIR/outputs"
PACKAGE_STAGE="$(mktemp -d "$PROJECT_DIR/work/package.XXXXXX")"
trap 'rm -rf -- "$PACKAGE_STAGE"' EXIT
PAYLOAD_DIR="$PACKAGE_STAGE/payload"
DOC_DIR="$PAYLOAD_DIR/usr/share/doc/magic-hotspot"
install -d -m 0755 "$PACKAGE_STAGE/debian" "$PAYLOAD_DIR/DEBIAN" \
    "$PAYLOAD_DIR/usr/bin" "$PAYLOAD_DIR/usr/share/applications" \
    "$PAYLOAD_DIR/usr/share/icons/hicolor/scalable/apps" "$DOC_DIR"
install -m 0755 "$APP_BINARY" "$PAYLOAD_DIR/usr/bin/magic-hotspot"
install -m 0755 "$TRANSPORT_BINARY" "$PAYLOAD_DIR/usr/bin/magic-usb"
install -D -m 0755 "$CSR_BINARY" "$PAYLOAD_DIR/usr/libexec/magic-usb/csr8510"
install -D -m 0644 "$PROJECT_DIR/magic-usb/packaging/magic-usb@.service"     "$PAYLOAD_DIR/usr/lib/systemd/system/magic-usb@.service"
install -D -m 0644 "$PROJECT_DIR/magic-usb/README.txt" "$DOC_DIR/magic-usb/README.txt"
install -D -m 0644 "$PROJECT_DIR/magic-usb/COPYING" "$DOC_DIR/magic-usb/COPYING"
install -d -m 0755 "$PAYLOAD_DIR/usr/libexec" "$PAYLOAD_DIR/usr/lib/systemd/system" \
    "$PAYLOAD_DIR/usr/share/polkit-1/rules.d"
install -m 0755 "$USB_BINARY" "$PAYLOAD_DIR/usr/libexec/magic-hotspot-usb-engine"
install -m 0644 "$PROJECT_DIR/packaging/usb_helper.py" "$PAYLOAD_DIR/usr/libexec/magic-hotspot-usb-helper"
install -m 0644 "$PROJECT_DIR/packaging/magic-hotspot-usb.service" "$PAYLOAD_DIR/usr/lib/systemd/system/"
install -m 0644 "$PROJECT_DIR/packaging/49-magic-hotspot.rules" "$PAYLOAD_DIR/usr/share/polkit-1/rules.d/"
for script in postinst prerm postrm; do
    install -m 0755 "$PROJECT_DIR/packaging/$script" "$PAYLOAD_DIR/DEBIAN/$script"
done
install -m 0644 "$PROJECT_DIR/packaging/com.example.MagicHotspot.desktop" \
    "$PAYLOAD_DIR/usr/share/applications/com.example.MagicHotspot.desktop"
install -m 0644 "$PROJECT_DIR/assets/magic-hotspot.svg" \
    "$PAYLOAD_DIR/usr/share/icons/hicolor/scalable/apps/magic-hotspot.svg"
install -d "$PAYLOAD_DIR/usr/share/gnome-shell/extensions/magic-hotspot@local"
install -m 0644 "$PROJECT_DIR/gnome/metadata.json" "$PROJECT_DIR/gnome/extension.js" \
    "$PAYLOAD_DIR/usr/share/gnome-shell/extensions/magic-hotspot@local/"
install -m 0644 "$PROJECT_DIR/README.md" "$DOC_DIR/README.md"
install -m 0644 "$PROJECT_DIR/assets/fluent/LICENSE.txt" "$DOC_DIR/FLUENT-ICONS-LICENSE.txt"
install -m 0644 "$PROJECT_DIR/vendor/usbmuxd-1.1.1/COPYING.GPLv2" "$DOC_DIR/USB-BACKEND-GPLv2.txt"
install -m 0644 "$PROJECT_DIR/vendor/usbmuxd-1.1.1/COPYING.GPLv3" "$DOC_DIR/USB-BACKEND-GPLv3.txt"
{
    cat "$PROJECT_DIR/LICENSE"
    printf '\nMicrosoft Fluent System Icons included in Magic Hotspot.\n\n'
    printf '%s\n\n' 'Source: https://github.com/microsoft/fluentui-system-icons'
    cat "$PROJECT_DIR/assets/fluent/LICENSE.txt"
    printf "\nMagic USB and shared CSR client: GPL-2.0-only.\n"
    cat "$PROJECT_DIR/magic-usb/COPYING"
    printf '\nUSB backend: modified usbmuxd 1.1.1 (GPL-2.0-or-3.0).\nSource: https://github.com/libimobiledevice/usbmuxd\nCorresponding source is included in Magic-Hotspot.zip.\n'
} > "$DOC_DIR/copyright"

cat > "$PACKAGE_STAGE/debian/control" <<'CONTROL'
Source: magic-hotspot
Section: net
Priority: optional
Maintainer: Magic Hotspot Project <noreply@localhost>
Standards-Version: 4.7.0

Package: magic-hotspot
Architecture: any
Description: Compact GTK3 iPhone hotspot and Wi-Fi helper
 Connect by iPhone USB, saved Wi-Fi or Bluetooth PAN, browse nearby devices,
 and optionally keep the application in the desktop tray.
CONTROL

LIBRARY_VARS="$(cd "$PACKAGE_STAGE" && LC_ALL=C dpkg-shlibdeps -O -e"$PAYLOAD_DIR/usr/bin/magic-hotspot" -e"$PAYLOAD_DIR/usr/libexec/magic-hotspot-usb-engine" -e"$PAYLOAD_DIR/usr/bin/magic-usb" -e"$PAYLOAD_DIR/usr/libexec/magic-usb/csr8510")"
LIBRARY_DEPENDS="${LIBRARY_VARS#shlibs:Depends=}"
if [[ "$LIBRARY_VARS" != shlibs:Depends=* || -z "$LIBRARY_DEPENDS" ]]; then
    printf '%s\n' 'Could not derive the shared-library dependencies.' >&2
    exit 1
fi
INSTALLED_SIZE="$(du -sk --apparent-size "$PAYLOAD_DIR/usr" | awk '{print $1}')"
cat > "$PAYLOAD_DIR/DEBIAN/control" <<CONTROL
Package: magic-hotspot
Version: $DEB_VERSION
Architecture: $DEB_ARCH
Section: net
Priority: optional
Maintainer: Magic Hotspot Project <noreply@localhost>
Installed-Size: $INSTALLED_SIZE
Conflicts: magic-usb
Replaces: magic-usb
Depends: $LIBRARY_DEPENDS, network-manager, bluez, librsvg2-common, usbmuxd, ipheth-utils, python3, libimobiledevice-1.0-6, polkitd, systemd, iproute2, kmod
Description: Compact GTK3 iPhone hotspot and Wi-Fi helper
 Connect by iPhone USB, saved Wi-Fi or Bluetooth PAN, browse nearby devices,
 and optionally keep the application in the desktop tray.
 Login startup is opt-in and passwords remain with NetworkManager.
CONTROL

PACKAGE_EPOCH="${SOURCE_DATE_EPOCH:-$(stat -c %Y "$APP_BINARY")}"
export SOURCE_DATE_EPOCH="$PACKAGE_EPOCH"
CHANGELOG_DATE="$(TZ=America/Sao_Paulo date --date="@$PACKAGE_EPOCH" -R)"
cat > "$DOC_DIR/changelog.Debian" <<CHANGELOG
magic-hotspot ($DEB_VERSION) resolute; urgency=medium

  * Wait for adapter swaps on the selected USB port and retry with bounded backoff.
  * Read cached USB configuration before CSR reset to recover stalled adapters.

 -- Magic Hotspot Project <noreply@localhost>  $CHANGELOG_DATE
CHANGELOG
gzip -9n "$DOC_DIR/changelog.Debian"
chmod -R go-w "$PAYLOAD_DIR"
chmod 0644 "$PAYLOAD_DIR/DEBIAN/control" "$DOC_DIR/copyright" "$DOC_DIR/changelog.Debian.gz"
(
    cd "$PAYLOAD_DIR"
    find usr -type f -print0 | sort -z | xargs -0 md5sum > DEBIAN/md5sums
)
chmod 0644 "$PAYLOAD_DIR/DEBIAN/md5sums"
if command -v desktop-file-validate >/dev/null 2>&1; then
    desktop-file-validate "$PAYLOAD_DIR/usr/share/applications/com.example.MagicHotspot.desktop"
fi
DEB_FILE="$PROJECT_DIR/outputs/magic-hotspot_${DEB_VERSION}_${DEB_ARCH}.deb"
dpkg-deb --root-owner-group --build "$PAYLOAD_DIR" "$DEB_FILE"
(
    cd "$PROJECT_DIR/outputs"
    sha256sum "$(basename -- "$DEB_FILE")" > "$(basename -- "$DEB_FILE").sha256"
)
printf 'Ubuntu 26.04 package: %s\n' "$DEB_FILE"

python3 "$PROJECT_DIR/archive_source.py"
