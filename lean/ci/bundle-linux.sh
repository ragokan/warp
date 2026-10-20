#!/usr/bin/env bash
set -euo pipefail

binary="${1:?Usage: bundle-linux.sh BINARY OUTPUT_DIRECTORY}"
output="${2:?Usage: bundle-linux.sh BINARY OUTPUT_DIRECTORY}"
mkdir -p "$output"
output="$(cd "$output" && pwd)"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
package="$stage/lean-warp-linux-arm64"
mkdir -p "$package"
install -m755 "$binary" "$package/warp-terminal-oss"
install -m644 app/channels/oss/dev.warp.WarpOss.desktop "$package/"
install -m644 app/channels/oss/icon/no-padding/512x512.png "$package/dev.warp.WarpOss.png"
cp LICENSE-AGPL LICENSE-MIT "$package/"
SETTINGS_SCHEMA_EXECUTABLE="$package/warp-terminal-oss" \
  script/prepare_bundled_resources "$package/resources" oss
file "$package/warp-terminal-oss"
ldd "$package/warp-terminal-oss" | tee "$stage/ldd.txt"
python3 lean/ci/verify-bundle.py linux "$package" --ldd "$stage/ldd.txt"
tar -czf "$output/lean-warp-linux-arm64.tar.gz" -C "$stage" lean-warp-linux-arm64
