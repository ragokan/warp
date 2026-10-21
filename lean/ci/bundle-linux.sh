#!/usr/bin/env bash
set -euo pipefail

binary="${1:?Usage: bundle-linux.sh BINARY OUTPUT_DIRECTORY}"
output="${2:?Usage: bundle-linux.sh BINARY OUTPUT_DIRECTORY}"
mkdir -p "$output"
output="$(cd "$output" && pwd)"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
package="$stage/warper-linux-arm64"
mkdir -p "$package"
install -m755 "$binary" "$package/warper"
install -m755 lean/warper-update "$package/warper-update"
sed -e 's/^Name=.*/Name=Warper/' -e 's/^Exec=.*/Exec=warper %U/' \
  app/channels/oss/dev.warp.WarpOss.desktop > "$package/dev.warp.WarpOss.desktop"
install -m644 app/channels/oss/icon/no-padding/512x512.png "$package/dev.warp.WarpOss.png"
cp LICENSE-AGPL LICENSE-MIT "$package/"
# begin version
tag="${GIT_RELEASE_TAG:-development}"
if [[ $tag =~ ^lean-v([0-9]+\.[0-9]+\.[0-9]+)([-+].+)?$ ]]; then
  version="${BASH_REMATCH[1]}"
elif [[ ${GITHUB_REF_TYPE:-} == tag ]]; then
  echo "Invalid lean release tag: $tag" >&2
  exit 1
else
  version=0.1.0
fi
# end version
echo "$version" > "$package/VERSION"
SETTINGS_SCHEMA_EXECUTABLE="$package/warper" \
  script/prepare_bundled_resources "$package/resources" oss
tar -czf "$output/warper-linux-arm64.tar.gz" -C "$stage" warper-linux-arm64
extracted="$stage/extracted"
mkdir "$extracted"
tar -xzf "$output/warper-linux-arm64.tar.gz" -C "$extracted"
file "$extracted/warper-linux-arm64/warper"
ldd "$extracted/warper-linux-arm64/warper" | tee "$stage/ldd.txt"
python3 lean/ci/verify-bundle.py linux "$extracted/warper-linux-arm64" --ldd "$stage/ldd.txt"
