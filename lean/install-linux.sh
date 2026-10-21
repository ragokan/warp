#!/usr/bin/env bash
set -euo pipefail
[[ $(uname -s) == Linux ]] || { echo 'This installer is for Linux.' >&2; exit 1; }
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
binary=${1:-$repo/target/lean/warp-oss}
[[ -x $binary ]] || { echo "Build the binary first: $binary" >&2; exit 1; }
prefix=${LEAN_INSTALL_PREFIX:-$HOME/.local}
config=${XDG_CONFIG_HOME:-$HOME/.config}
data=${XDG_DATA_HOME:-$HOME/.local/share}
state=${XDG_STATE_HOME:-$HOME/.local/state}
app=$prefix/lib/warp-lean
install -d "$app/resources" "$prefix/bin" "$data/applications" \
    "$data/icons/hicolor/512x512/apps" "$config/warp-oss" "$data/warp-oss" "$state/warp-oss"
if [[ -f $app/warp-oss && ! -e $app/warp-oss.lean-v0.1.0 ]]; then
    cp -p "$app/warp-oss" "$app/warp-oss.lean-v0.1.0"
fi
install -m 755 "$binary" "$app/warp-oss"
install -m 755 "$repo/lean/warper-update" "$app/warper-update"
# A source build has no release version; a stale one would hide that from warper-update.
if [[ -f $(dirname "$binary")/VERSION ]]; then
    install -m 644 "$(dirname "$binary")/VERSION" "$app/VERSION"
else
    rm -f "$app/VERSION"
fi
install -m 644 "$repo/LICENSE-MIT" "$repo/LICENSE-AGPL" "$app/resources/"
"$app/warp-oss" dump-settings-schema "$app/resources/settings_schema.json"
install -m 644 "$repo/app/channels/oss/icon/no-padding/512x512.png" \
    "$data/icons/hicolor/512x512/apps/dev.warp.WarpOss.png"
ln -sfn "$app/warp-oss" "$prefix/bin/warper"
if [[ -L $prefix/bin/warp-lean ]]; then
    case $(readlink -f "$prefix/bin/warp-lean") in
        "$app"/*) unlink "$prefix/bin/warp-lean" ;;
    esac
fi
for file in settings.toml keybindings.yaml; do
    if [[ ! -e $config/warp-oss/$file && -f $config/warp-terminal/$file ]]; then
        cp -p "$config/warp-terminal/$file" "$config/warp-oss/$file"
    fi
done
for directory in themes; do
    if [[ ! -e $data/warp-oss/$directory && -d $data/warp-terminal/$directory ]]; then
        cp -a "$data/warp-terminal/$directory" "$data/warp-oss/$directory"
    fi
done
# Legacy preferences can contain saved prompts and credentials; never migrate that store.
cat > "$data/applications/dev.warp.WarpOss.desktop" <<DESKTOP
[Desktop Entry]
Version=1.0
Type=Application
Name=Warper
GenericName=Terminal Emulator
Exec="$app/warp-oss" %U
Icon=dev.warp.WarpOss
StartupWMClass=dev.warp.WarpOss
Categories=System;TerminalEmulator;
Terminal=false
MimeType=x-scheme-handler/warposs;
DESKTOP
if command -v update-desktop-database >/dev/null; then
    update-desktop-database "$data/applications"
fi
if command -v xdg-mime >/dev/null; then
    xdg-mime default dev.warp.WarpOss.desktop x-scheme-handler/warposs
fi
printf 'Installed %s\nConfig: %s/warp-oss\nData: %s/warp-oss\nState: %s/warp-oss\n' \
    "$prefix/bin/warper" "$config" "$data" "$state"
echo 'Legacy user_preferences.json and all authentication state were left behind.'
