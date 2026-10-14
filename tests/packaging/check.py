# Module: magic_hotspot.tests.packaging
# Purpose: Verify unified payload, source completeness and reproducible Rust builds.
# Created: 2026-10-13
# Architecture: Extract into a temporary directory; never install or touch hardware.
from pathlib import Path
import os
import subprocess
import sys
import tempfile
from zipfile import ZipFile

package = Path(sys.argv[1]).resolve()
archive = Path(sys.argv[2]).resolve()
def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)

with tempfile.TemporaryDirectory(prefix="magic-hotspot-package-check-") as temp:
    temp = Path(temp)
    payload = temp / "payload"
    control = temp / "control"
    run("dpkg-deb", "-x", str(package), str(payload))
    run("dpkg-deb", "-e", str(package), str(control))
    run("md5sum", "--quiet", "-c", str(control / "md5sums"), cwd=payload)
    for relative in ("usr/bin/magic-hotspot", "usr/bin/magic-usb",
                     "usr/libexec/magic-usb/csr8510",
                     "usr/libexec/magic-hotspot-usb-engine"):
        binary = payload / relative
        assert binary.read_bytes().startswith(b"\x7fELF"), relative
        assert os.access(binary, os.X_OK), relative
    unit = (payload / "usr/lib/systemd/system/magic-usb@.service").read_text()
    for command in ("ExecStart=/usr/bin/magic-usb serve %i",
                    "ExecStopPost=/usr/bin/magic-usb restore %i"):
        assert command in unit, command
    metadata = (control / "control").read_text()
    assert "Package: magic-hotspot\n" in metadata
    assert "Conflicts: magic-usb\n" in metadata
    assert "Replaces: magic-usb\n" in metadata
    for library in ("libusb-1.0-0", "libzstd1"):
        assert library in metadata, library
    assert (payload / "usr/share/doc/magic-hotspot/magic-usb/COPYING").is_file()
    with ZipFile(archive) as source:
        assert source.testzip() is None
        names = set(source.namelist())
        for relative in ("magic-usb/src/bin/magic-usb.rs",
                         "magic-usb/clients/csr8510/src/main.rs",
                         "magic-usb/COPYING", "magic-usb/Cargo.lock"):
            assert "magic-hotspot/" + relative in names, relative
        assert not any("/target/" in n or "/work/" in n or "/outputs/" in n
                       or n.startswith("magic-hotspot/magic-usb/bin/") for n in names)
        assert "magic-hotspot/magic-usb/package.py" in names
        source.extractall(temp / "source")
    root = temp / "source/magic-hotspot"
    run("cargo", "check", "--locked", "--offline", "--bin", "magic-hotspot",
        "--target-dir", str(temp / "gui-target"), cwd=root)
    run("cargo", "build", "--locked", "--offline", "--workspace",
        "--manifest-path", "magic-usb/Cargo.toml", "--target-dir",
        str(temp / "usb-target"), cwd=root)
    assert (temp / "usb-target/debug/magic-usb").is_file()
    assert (temp / "usb-target/debug/csr8510").is_file()
print("PASS: unified Debian payload/checksums, migration metadata, license notices, complete source and clean Rust builds.")
