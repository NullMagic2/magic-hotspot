#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_DIR"

# Remove Cargo intermediates while preserving the binary and installable package.
rm -rf -- "$PROJECT_DIR/target" "$PROJECT_DIR/magic-usb/target"
rm -rf -- "$PROJECT_DIR/work/usb-backend/build" "$PROJECT_DIR/__pycache__" "$PROJECT_DIR/packaging/__pycache__"
printf 'Removed build intermediates. Preserved bin/ and installable packages in outputs/.\n'
