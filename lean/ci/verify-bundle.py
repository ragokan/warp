#!/usr/bin/env python3
"""Check unpacked release contents without running the packaged application."""

import argparse
import json
from pathlib import Path
import plistlib
import re
import struct


def verify(platform, root, ldd=None):
    if platform == "macos":
        contents = root / "Contents"
        with (contents / "Info.plist").open("rb") as stream:
            plist = plistlib.load(stream)
        assert plist["CFBundleIdentifier"] == "dev.warp.WarpOss", plist
        assert plist["CFBundleExecutable"] == "warp-oss", plist
        assert plist["CFBundlePackageType"] == "APPL", plist
        assert re.fullmatch(r"\d+\.\d+\.\d+", plist["CFBundleVersion"]), plist
        binary = contents / "MacOS" / plist["CFBundleExecutable"]
        resources = contents / "Resources"
        assert (resources / plist["CFBundleIconFile"]).stat().st_size > 0
        plugin = contents / "PlugIns" / plist["NSDockTilePlugIn"]
        assert (plugin / "Contents/MacOS/WarpDockTilePlugin").stat().st_size > 0
        with binary.open("rb") as stream:
            magic, cpu = struct.unpack("<II", stream.read(8))
        assert (magic, cpu) == (0xFEEDFACF, 0x0100000C), "Expected arm64 Mach-O"
        print(json.dumps(plist, indent=2))
    else:
        binary = root / "warp-terminal-oss"
        resources = root / "resources"
        with binary.open("rb") as stream:
            header = stream.read(20)
        assert header[:6] == b"\x7fELF\x02\x01", "Expected little-endian ELF64"
        assert struct.unpack_from("<H", header, 18)[0] == 183, "Expected ELF aarch64"
        desktop = (root / "dev.warp.WarpOss.desktop").read_text()
        assert "Exec=warp-terminal-oss %U" in desktop
        assert "Icon=dev.warp.WarpOss" in desktop
        assert (root / "dev.warp.WarpOss.png").stat().st_size > 0
        if ldd is not None:
            assert "not found" not in ldd, ldd
            libraries = re.findall(r"(/\S+)", ldd)
            assert libraries, "Expected dynamically linked system libraries"
            for library in libraries:
                assert library.startswith(("/lib/", "/lib64/", "/usr/lib/", "/usr/lib64/")), library
    assert binary.stat().st_mode & 0o111, "Binary is not executable"
    assert (resources / "bundled").is_dir(), "Bundled resources missing"
    assert (resources / "THIRD_PARTY_LICENSES.txt").stat().st_size > 0
    with (resources / "settings_schema.json").open() as stream:
        assert isinstance(json.load(stream), dict), "Expected settings schema object"
    print(f"Verified {platform} arm64 package: {root}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("platform", choices=["macos", "linux"])
    parser.add_argument("root", type=Path)
    parser.add_argument("--ldd", type=Path)
    args = parser.parse_args()
    verify(args.platform, args.root, args.ldd.read_text() if args.ldd else None)
