import importlib.util
from pathlib import Path
import plistlib
import struct
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("verify_bundle", Path(__file__).with_name("verify-bundle.py"))
bundle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bundle)


class VerifyBundleTests(unittest.TestCase):
    def test_macos_package_rejects_wrong_arch_and_bundle_identifier(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            contents = root / "Contents"
            (contents / "MacOS").mkdir(parents=True)
            binary = contents / "MacOS/warp-oss"
            binary.write_bytes(struct.pack("<II", 0xFEEDFACF, 0x0100000C))
            binary.chmod(0o755)
            resources = contents / "Resources"
            (resources / "bundled").mkdir(parents=True)
            (resources / "THIRD_PARTY_LICENSES.txt").write_text("licenses")
            (resources / "settings_schema.json").write_text("{}")
            (resources / "WarpOss.icns").write_bytes(b"icon")
            (resources / "bin").mkdir()
            wrapper = resources / "bin/warp-oss"
            wrapper.write_text('#!/bin/bash\nexec "$script_dir/../../MacOS/warp-oss" "$@"\n')
            wrapper.chmod(0o755)
            plugin = contents / "PlugIns/WarpDockTilePlugin.docktileplugin/Contents/MacOS"
            plugin.mkdir(parents=True)
            (plugin / "WarpDockTilePlugin").write_bytes(b"plugin")
            plist = {
                "CFBundleIdentifier": "dev.warp.WarpOss",
                "CFBundleExecutable": "warp-oss",
                "CFBundlePackageType": "APPL",
                "CFBundleVersion": "0.1.0",
                "CFBundleIconFile": "WarpOss.icns",
                "NSDockTilePlugIn": "WarpDockTilePlugin.docktileplugin",
                "CFBundleURLTypes": [{"CFBundleURLSchemes": ["warposs"]}],
                "CFBundleDocumentTypes": [{"LSItemContentTypes": ["public.folder"]}],
                "NSServices": [{"NSMessage": "openTab"}, {"NSMessage": "openWindow"}],
                "SMAuthorizedClients": ["dev.warp.WarpOss"],
                "LSBackgroundOnly": False,
                "UIDesignRequiresCompatibility": True,
            }
            for permission in (
                "AppleEvents", "Camera", "Microphone", "Contacts", "Calendars",
                "CalendarsFullAccess", "Reminders", "RemindersFullAccess", "Location",
                "PhotoLibrary",
            ):
                plist[f"NS{permission}UsageDescription"] = "A program in Warp requests access."
            path = contents / "Info.plist"
            path.write_bytes(plistlib.dumps(plist))
            bundle.verify("macos", root)
            for key in (
                "CFBundleURLTypes", "CFBundleDocumentTypes", "NSServices", "SMAuthorizedClients",
                "NSCameraUsageDescription", "UIDesignRequiresCompatibility", "LSBackgroundOnly",
            ):
                with self.subTest(missing=key):
                    incomplete = dict(plist)
                    del incomplete[key]
                    path.write_bytes(plistlib.dumps(incomplete))
                    with self.assertRaises(AssertionError):
                        bundle.verify("macos", root)
            wrong_scheme = dict(plist, CFBundleURLTypes=[{"CFBundleURLSchemes": ["warp"]}])
            path.write_bytes(plistlib.dumps(wrong_scheme))
            with self.assertRaises(AssertionError):
                bundle.verify("macos", root)
            path.write_bytes(plistlib.dumps(plist))
            wrapper.chmod(0o644)
            with self.assertRaises(AssertionError):
                bundle.verify("macos", root)
            wrapper.chmod(0o755)
            binary.write_bytes(struct.pack("<II", 0xFEEDFACF, 0x01000007))
            with self.assertRaisesRegex(AssertionError, "arm64"):
                bundle.verify("macos", root)
            binary.write_bytes(struct.pack("<II", 0xFEEDFACF, 0x0100000C))
            plist["CFBundleIdentifier"] = "dev.warp.Warp-Stable"
            path.write_bytes(plistlib.dumps(plist))
            with self.assertRaises(AssertionError):
                bundle.verify("macos", root)

    def test_linux_package_rejects_wrong_arch_missing_resources_and_private_libraries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "warp-terminal-oss"
            header = bytearray(20)
            header[:6] = b"\x7fELF\x02\x01"
            struct.pack_into("<H", header, 18, 183)
            binary.write_bytes(header)
            binary.chmod(0o755)
            desktop = root / "dev.warp.WarpOss.desktop"
            entry = "Exec=warp-terminal-oss %U\nIcon=dev.warp.WarpOss\nMimeType=x-scheme-handler/warposs;\n"
            desktop.write_text(entry)
            (root / "dev.warp.WarpOss.png").write_bytes(b"icon")
            resources = root / "resources"
            (resources / "bundled").mkdir(parents=True)
            (resources / "THIRD_PARTY_LICENSES.txt").write_text("licenses")
            (resources / "settings_schema.json").write_text("{}")
            system = "libc.so.6 => /lib/aarch64-linux-gnu/libc.so.6 (0x1234)"
            bundle.verify("linux", root, system)
            for invalid in (entry.replace("warposs", "warp"), entry.split("MimeType")[0]):
                desktop.write_text(invalid)
                with self.assertRaises(AssertionError):
                    bundle.verify("linux", root, system)
            desktop.write_text(entry)
            for dependencies in ["libfoo.so => not found", "libfoo.so => /opt/warp/libfoo.so (0x1234)", ""]:
                with self.assertRaises(AssertionError):
                    bundle.verify("linux", root, dependencies)
            struct.pack_into("<H", header, 18, 62)
            binary.write_bytes(header)
            with self.assertRaisesRegex(AssertionError, "aarch64"):
                bundle.verify("linux", root, system)
            binary.write_bytes(header[:18] + struct.pack("<H", 183))
            (resources / "bundled").rmdir()
            with self.assertRaisesRegex(AssertionError, "resources missing"):
                bundle.verify("linux", root, system)
