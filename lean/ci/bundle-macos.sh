#!/usr/bin/env bash
set -euo pipefail

binary="${1:?Usage: bundle-macos.sh BINARY OUTPUT_DIRECTORY}"
output="${2:?Usage: bundle-macos.sh BINARY OUTPUT_DIRECTORY}"
mkdir -p "$output"
output="$(cd "$output" && pwd)"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
app="$stage/Warper.app"
contents="$app/Contents"
mkdir -p "$contents/MacOS" "$contents/Resources" "$contents/PlugIns"
cp "$binary" "$contents/MacOS/warp-oss"
chmod 755 "$contents/MacOS/warp-oss"
cp -R "$(dirname "$binary")/WarpDockTilePlugin.docktileplugin" "$contents/PlugIns/"
plutil -insert MainAppBundleIdentifier -string dev.warp.WarpOss \
  "$contents/PlugIns/WarpDockTilePlugin.docktileplugin/Contents/Info.plist"

python3 - "$contents/Info.plist" <<'PY'
import os
import plistlib
import re
import sys

tag = os.environ.get("GIT_RELEASE_TAG", "development")
match = re.fullmatch(r"lean-v(\d+\.\d+\.\d+)(?:[-+].+)?", tag)
if os.environ.get("GITHUB_REF_TYPE") == "tag" and match is None:
    raise SystemExit(f"Invalid lean release tag: {tag}")
version = match[1] if match else "0.1.0"
with open(sys.argv[1], "wb") as stream:
    plistlib.dump({
        "CFBundleDevelopmentRegion": "English",
        "CFBundleName": "Warper",
        "CFBundleDisplayName": "Warper",
        "CFBundleExecutable": "warp-oss",
        "CFBundleIdentifier": "dev.warp.WarpOss",
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundlePackageType": "APPL",
        "CFBundleVersion": version,
        "CFBundleShortVersionString": version,
        "CFBundleIconFile": "WarpOss.icns",
        "LSApplicationCategoryType": "public.app-category.developer-tools",
        "LSMinimumSystemVersion": "11.0",
        "NSHighResolutionCapable": True,
        "NSPrincipalClass": "NSApplication",
        "NSDockTilePlugIn": "WarpDockTilePlugin.docktileplugin",
        "LSBackgroundOnly": False,
        "SMAuthorizedClients": ["dev.warp.WarpOss"],
        "NSServices": [{
            "NSMenuItem": {"default": f"New Warper {kind} Here"},
            "NSMessage": message,
            "NSRequiredContext": {"NSTextContent": "FilePath"},
            "NSSendTypes": ["NSFilenamesPboardType", "public.plain-text"],
        } for kind, message in [("Tab", "openTab"), ("Window", "openWindow")]],
    }, stream)
PY
WARP_PLIST_PATH="$contents/Info.plist" WARP_SCHEME_NAME=warposs script/update_plist

iconset="$stage/WarpOss.iconset"
mkdir "$iconset"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" app/channels/oss/icon/no-padding/512x512.png \
    --out "$iconset/icon_${size}x${size}.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$contents/Resources/WarpOss.icns"
SETTINGS_SCHEMA_EXECUTABLE="$contents/MacOS/warp-oss" \
  script/prepare_bundled_resources "$contents/Resources" oss
cp LICENSE-AGPL LICENSE-MIT "$contents/Resources/"
install -m755 lean/warper-update "$contents/Resources/warper-update"

codesign --force --deep -s - "$app"
codesign --verify --deep --strict --verbose=2 "$app"
codesign -dv --verbose=4 "$app" 2>&1
plutil -lint "$contents/Info.plist"
file "$contents/MacOS/warp-oss"
otool -L "$contents/MacOS/warp-oss"
python3 lean/ci/verify-bundle.py macos "$app"

ditto -c -k --keepParent "$app" "$output/warper-macos-arm64.zip"
extracted="$stage/extracted"
mkdir "$extracted"
ditto -x -k "$output/warper-macos-arm64.zip" "$extracted"
python3 lean/ci/verify-bundle.py macos "$extracted/Warper.app"
codesign --verify --deep --strict --verbose=2 "$extracted/Warper.app"
disk="$stage/disk"
mkdir "$disk"
ditto "$app" "$disk/Warper.app"
ln -s /Applications "$disk/Applications"
hdiutil create -volname Warper -srcfolder "$disk" -ov -format UDZO \
  "$output/warper-macos-arm64.dmg"
