# Lean arm64 release pipeline

`lean-release.yml` builds on `lean-ci` pushes and `lean-v*` tag pushes. Both
platforms upload artifacts for 14 days. Only `lean-v*` tags create a **draft**
GitHub Release, after both builds pass. No release is created on branch pushes.

The build currently uses the stock OSS binary with the release profile, bundled
resources, no LTO, and no debug symbols. In the single **Build production OSS
binary** step, switch the `binary=...` line to:

```bash
binary="$(bash lean/build.sh)"
```

The build script must write only its binary path to stdout and diagnostics to
stderr. Packaging accepts that path, including a custom Cargo profile directory.
It does not invoke the private channel config installer or use Warp secrets.

## macOS

Extract `lean-warp-macos-arm64.zip`, or mount the DMG, and move `WarpOss.app` to
Applications. The app is ad-hoc signed, **not notarized**. On first launch,
right-click the app and select **Open**. If macOS still blocks it, and you trust
the downloaded build, remove its quarantine attribute:

```bash
xattr -dr com.apple.quarantine /Applications/WarpOss.app
```

CI checks the bundle plist, arm64 Mach-O header, resources, settings schema,
licenses, and signature. `codesign -dv --verbose=4` is captured in its logs.
Packaging checks do not establish successful GUI startup on a real Mac.

## Linux

Extract `lean-warp-linux-arm64.tar.gz` and run `./warp-terminal-oss` from the
extracted directory. Keep `resources/` next to the executable. The included
desktop file expects `warp-terminal-oss` on PATH; install the PNG under the icon
name `dev.warp.WarpOss` and the desktop file under your applications directory
if you want launcher integration.

The build targets Ubuntu 24.04 arm64 and requires its standard system libraries;
older distributions may not satisfy the glibc requirement. CI checks the ELF
architecture and `ldd` output for missing libraries or paths outside system
library directories. A graphical session, fonts, and standard EGL/Wayland/X11
runtime libraries are required to launch it. The optional `.deb` is omitted:
upstream's packaging also configures Warp's apt repository and update scripts.

## Local packaging checks

```bash
bash -n lean/ci/bundle-macos.sh lean/ci/bundle-linux.sh
python3 -m unittest discover -s lean/ci -p '*_tests.py'
python3 lean/ci/verify-bundle.py macos /path/to/WarpOss.app
python3 lean/ci/verify-bundle.py linux /path/to/lean-warp-linux-arm64
```
