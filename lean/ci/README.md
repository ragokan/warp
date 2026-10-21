# Lean arm64 release pipeline

`lean-release.yml` builds on `lean-ci` pushes and `lean-v*` tag pushes. Both
platforms upload artifacts for 14 days. Only `lean-v*` tags create a **draft**
GitHub Release, after both builds pass. No release is created on branch pushes.
There is no `workflow_dispatch` trigger: a release tag must point to a commit
containing this workflow. Tags must use `lean-vX.Y.Z` with an optional nonempty
`-prerelease` or `+build` suffix; malformed tags fail packaging.

The **Build production lean binary** step runs `lean/build.sh`, which builds
`warp-oss` with `--no-default-features`, an explicit terminal feature list, and
the `lean` Cargo profile exactly as defined in `Cargo.toml`: ThinLTO, its default
codegen units, no debug symbols, and stripped symbols. CI has no profile
overrides; branch tests and tagged releases use the same production build.
Packaging accepts the binary path printed by the build script. It does not
invoke the private channel config installer or use Warp secrets.
`GIT_RELEASE_TAG` is set only during packaging, for the plist and bundled
`version.json`; it is unset during compilation. Archive verification uses
`strings -a` to reject embedded `lean-ci` or `lean-v<digit>` version markers.
Both archives are extracted into fresh directories and verified in CI; the
extracted macOS app also passes signature verification.

The Oz and skill documentation under `resources/bundled/skills` is inert in the
lean build. It remains bundled with the other resources; verification requires
the `bundled/` directory.

Workflows introduced by a future upstream sync can arrive enabled. After each
sync, list all workflows and disable newly introduced upstream workflows,
leaving **Lean release** active:

```bash
gh workflow list --all --repo ragokan/warp
gh workflow disable <new-workflow-id> --repo ragokan/warp
```

## macOS

Extract `lean-warp-macos-arm64.zip`, or mount the DMG, and move `WarpOss.app` to
Applications. The app is ad-hoc signed, **not notarized**. On macOS 15+, try
opening it once, then use **System Settings → Privacy & Security → Open Anyway**
if you trust the downloaded build. See [Apple's Gatekeeper instructions](https://support.apple.com/en-us/102445).
Alternatively, remove its quarantine attribute:

```bash
xattr -dr com.apple.quarantine /Applications/WarpOss.app
```

CI checks the bundle plist, URL scheme, document types, usage descriptions,
terminal services, authorized client, arm64 Mach-O header, resources, settings
schema, licenses, and signature. The bundle omits `Resources/bin` and its Warp/Oz
CLI entry point; CI asserts its absence. Packaging invokes the app executable
directly to generate the settings schema. `codesign -dv --verbose=4` is captured in logs.
Packaging checks do not establish successful GUI startup on a real Mac.

## Linux

Extract `lean-warp-linux-arm64.tar.gz`. For a portable run, keep the entire
extracted directory together and run `./warp-terminal-oss` there.

For installation, use the checkout's existing [`lean/install-linux.sh`](../install-linux.sh)
with the extracted binary, then copy the packaged resources alongside the real
installed binary. From the repository root, with default XDG paths:

```bash
package=/absolute/path/to/lean-warp-linux-arm64
bash lean/install-linux.sh "$package/warp-terminal-oss"
cp -R "$package/resources/." "$HOME/.local/lib/warp-lean/resources/"
update-desktop-database "$HOME/.local/share/applications"
xdg-mime default dev.warp.WarpOss.desktop x-scheme-handler/warposs
```

The installer places the real binary at `~/.local/lib/warp-lean/warp-oss` and
creates a `~/.local/bin/warp-lean` **symlink**. Keep `resources/` next to that real
binary; do not copy only the binary onto PATH. Ensure `~/.local/bin` is on PATH
for shell use. The installer writes a desktop entry with an **absolute Exec
path**, so desktop launch and login callbacks work independently of the GUI
session's PATH. The portable archive's reference desktop entry keeps the bare
`warp-terminal-oss` name for relocatable installations; use the installer's
generated desktop entry for this installation. The database refresh and
`warposs` MIME registration above enable the login callback. If you customize
`LEAN_INSTALL_PREFIX` or XDG directories, use the matching paths.

The build targets Ubuntu 24.04 arm64 and requires its standard system libraries;
older distributions may not satisfy the glibc requirement. CI checks the ELF
architecture and `ldd` output for missing libraries or paths outside system
library directories. A graphical session, fonts, and standard EGL/Wayland/X11
runtime libraries are required to launch it. The optional `.deb` is omitted:
upstream's packaging also configures Warp's apt repository and update scripts.

## Local packaging checks

```bash
for script in lean/ci/bundle-macos.sh lean/ci/bundle-linux.sh; do
  bash -n "$script"
done
python3 -m unittest discover -s lean/ci -p '*_tests.py'
python3 lean/ci/verify-bundle.py macos /path/to/WarpOss.app
python3 lean/ci/verify-bundle.py linux /path/to/lean-warp-linux-arm64
```
