# Module: magic_hotspot.source_archive
# Purpose: Save complete corresponding source alongside the unified package.
# Created: 2026-10-13
# Architecture: Include owned source folders; omit build outputs and local state.
from hashlib import sha256
from pathlib import Path
import tomllib
from zipfile import ZIP_DEFLATED, ZipFile

root = Path(__file__).resolve().parent
version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
out = root / "outputs"
out.mkdir(exist_ok=True)
archive = out / f"Magic-Hotspot-{version}-source.zip"
folders = ("src", "tests", "assets", "packaging", "gnome", "vendor", "magic-usb", "docs", ".github")
excluded = {"target", "work", "outputs", "__pycache__", ".git"}
files = [p for p in root.iterdir() if p.is_file() and not p.is_symlink()]
for folder in folders:
    files.extend(p for p in (root / folder).rglob("*")
                 if p.is_file() and not p.is_symlink()
                 and not excluded.intersection(p.relative_to(root).parts)
                 and p.relative_to(root).parts[:2] != ("magic-usb", "bin")
                 and p.suffix != ".pyc")
with ZipFile(archive, "w", ZIP_DEFLATED) as output:
    for path in sorted(files):
        output.write(path, "magic-hotspot/" + path.relative_to(root).as_posix())
checksum = sha256(archive.read_bytes()).hexdigest()
archive.with_name(archive.name + ".sha256").write_text(f"{checksum}  {archive.name}\n")
print(f"Complete source archive: {archive}")
