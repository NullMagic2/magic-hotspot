#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
    printf '%s\n' "Rust and Cargo are required. Install them before building." >&2
    exit 1
fi

if ! command -v pkg-config >/dev/null 2>&1 || ! pkg-config --exists gtk+-3.0; then
    printf '%s\n' "GTK3 development files are required. On Ubuntu install libgtk-3-dev and pkg-config." >&2
    exit 1
fi

cargo build --locked --release --bin magic-hotspot
mkdir -p "$PROJECT_DIR/bin"
install -m 0755 "$PROJECT_DIR/target/release/magic-hotspot" "$PROJECT_DIR/bin/magic-hotspot"
printf 'Built binary: %s\n' "$PROJECT_DIR/bin/magic-hotspot"
"$PROJECT_DIR/magic-usb/build.sh"
"$PROJECT_DIR/build_usb_backend.sh"
"$PROJECT_DIR/package.sh"
