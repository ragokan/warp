# Warper

A terminal-focused, **fully offline** `warp-oss` build from branch `lean`. It never signs in,
never contacts Warp's servers and takes its settings only from the local `settings.toml`. Stable
Warp's files are read only. `master` stays at upstream; nothing is pushed by this tooling.

Display name: **Warper**; app IDs, data directories, URL scheme and icons are unchanged.
Releases use `warper-macos-arm64.zip` / `.dmg` and `warper-linux-arm64.tar.gz` (`warper` binary).
The installer preserves `warp-oss.lean-v0.1.0` for rollback.

## Build

With Rust 1.92.0 and the upstream native build prerequisites installed:

```sh
lean/build.sh
```

This platform-neutral interface builds `warp-oss`, with `--locked --no-default-features`,
profile `lean`, `lean` and `jemalloc`, and an explicit terminal feature list. It prints the binary
path. Profile `lean` inherits release, keeps unwinding, removes debug info and symbols, and uses
ThinLTO. Neither `release_bundle`, GUI/voice, crash reporting nor any NLD classifier/heuristic is
selected. `gui` is an upstream voice dependency group; the binary still has the desktop GUI.

On this Linux arm64 machine, build only in Docker:

```sh
docker build -f lean/Dockerfile.linux -t warp-lean-builder lean
docker run --rm --name warp-lean-build \
  -v "$PWD:/work" \
  -v warp-lean-registry:/home/builder/.cargo/registry \
  -v warp-lean-git:/home/builder/.cargo/git \
  -e CARGO_BUILD_JOBS=12 warp-lean-builder lean/build.sh
```

New Docker volumes can be root-owned. If Cargo reports permission denied on its caches, correct
ownership of those two volumes to uid/gid 1000. Never mount over `.cargo` or `.rustup`; that would
hide the image's Rust toolchain. The target directory is `target/lean`, already gitignored.
Do not run `script/run`, bootstrap, or reference-fork code for this build.

## Install

```sh
lean/install-linux.sh
```

The installer puts the binary in `~/.local/lib/warp-lean/warp-oss`, adds
`~/.local/bin/warper`, installs the OSS icon and a desktop launcher, generates the settings
schema beside the executable, and registers `warposs://` for opening tabs, windows and settings.
Terminal fonts, shell bootstrap and normal UI assets are embedded by the upstream release asset
provider. No Oz skills or remote-server binaries are installed.

OSS directories follow XDG overrides, otherwise:

- Config: `~/.config/warp-oss`
- Data: `~/.local/share/warp-oss`
- State/logs: `~/.local/state/warp-oss`
- Cache: `~/.cache/warp-oss`

The installer copies missing `settings.toml` and `keybindings.yaml` files and the `themes`
directory from the corresponding `warp-terminal` directories. Existing OSS copies are preserved.
Legacy `user_preferences.json` is omitted because it contains free-form saved prompts and cannot be
established to be secret-free. No auth files, tokens, keychain entries or databases are migrated.

## Offline

Warper has no account. Startup never reads the keychain, so the app is always signed out, and
there is no sign-in, sign-up, Log out, Account page or Settings sync toggle. `warposs://auth` links
and `warposs://settings/account` are ignored. Settings live only in `settings.toml`
(`~/.config/warp-oss/settings.toml`, `~/.warp-oss/settings.toml` on macOS); sync it between
machines with your dotfiles. Edits to the file are picked up while Warper runs. The
`account.is_settings_sync_enabled` key is hidden and ignored.

The gates, each reusing an upstream code path:

- The secure-storage service is the existing "unavailable" provider (the one the remote daemon
  uses): reads find nothing and writes/removals are no-ops. Warper therefore never signs in and
  can never delete credentials that an earlier Warper or Warp stored.
- Upstream already skips workspace polling, cloud-object polling, referral lookups and the
  login-gated prompts while signed out. Login-gated prompts and `auth::log_out` (which deletes the
  local database) are additionally inert under lean.
- The settings syncer runs in its inert mode (`SettingsMode::should_sync_to_cloud`, as for the
  TUI), so it neither reads from nor writes to the cloud.
- Telemetry, crash reporting, autoupdate and the changelog were already off.
- As a fail-safe, every `http_client::Client` resolves hostnames through a resolver that always
  fails and ignores proxies, so a request that bypasses the gates above is refused before it opens
  a connection. WebSocket users are all behind the disabled surfaces. The guard is name-based: it
  does not stop a request to a literal IP address, which is blocked in practice only because no
  lean code makes one.

Older Warper builds stored a login in the platform keychain. Warper leaves it alone; delete it
yourself if you want it gone. On Linux, remove the `dev.warp.WarpOss` entries in your keyring
(for example with Seahorse), and `~/.local/state/warp-oss/dev.warp.WarpOss-*` if the keyring was
unavailable. On macOS, delete the `dev.warp.WarpOss` items in Keychain Access.

## Hidden surfaces

Lean also hides AI/Drive/MCP/notebooks, analytics/crash-reporting actions, sharing, teams,
billing and referrals from bindings, slash commands, menus, tips, login copy, banners,
tooltips and the settings schema.
Hidden preferences remain stored: registration excludes them and the TOML backend refuses their
writes/deletes while preserving the loaded document.
Lean no longer performs the legacy privacy migration or GetUserSettings, and its settings
syncer is inert, so seventeen upstream syncer tests are lean-excluded, as are twenty-two in
`terminal/view_tests.rs` that assert Agent Mode, CLI-agent and Vim-banner behavior; all run with
default features.
Ten other upstream tests are lean-excluded: four in
`search/slash_command_menu/static_commands/commands_tests.rs` assert removed slash commands;
six in `util/openable_file_type_tests.rs` assert Warp editor/viewer behavior. All remain
enabled with default features.
`notifications.preferences` is the preservation exception: its full compound schema/value
remains so ordinary terminal notifications still persist; the agent row is hidden. Referral
themes that an earlier login already unlocked stay unlocked; no new referral lookup is made.

Also hidden: local/cloud workflows; Warp's editor/review/file tree and Markdown viewer;
tab/launch configurations, groups/pins/colours/vertical tabs; block bookmarks/filter/sticky
headers/compact mode; community/help links; Vim and command corrections (error underlining and suggested fixes). File and Markdown
links use the selected external editor, `$EDITOR`, or the system default. Plain tabs, rename,
blocks, Find, copy and syntax highlighting remain. The “Show Warp network log” binding
(`input:insert_network_logging_workflow`) is hidden with Workflows; the Privacy-page viewer stays.

## Avatar

The round button at the top right opens the avatar menu. It shows the Yoda image built into
Warper. To use your own, put `avatar.png`, `avatar.jpg` or `avatar.jpeg` (first match wins) in the
config directory that holds `settings.toml` (`~/.config/warp-oss` on Linux, `~/.warp-oss` on
macOS). The gear icon only shows while the image loads or if it cannot be decoded. The image is
looked up once per run and cropped to a circle; restart Warper after replacing it.

## Updates

Warper never looks for updates itself. **Check for Updates…** (macOS: Warper menu; Linux: avatar
menu; both: command palette) opens a tab that runs the bundled `warper-update` script. The script
asks the GitHub releases of `ragokan/warper` for the latest `lean-vX.Y.Z` and says "up to date"
unless it is newer than the installed version; otherwise it shows the version and release link
and asks before doing anything. If the installed version is unknown it says so and asks too,
instead of treating the release as an update. On yes it downloads the release archive and `SHA256SUMS`, aborts
on a missing or mismatching checksum, installs, then asks whether to restart Warper now (open tabs
close). On yes the script starts a detached relauncher that sends no signal: it waits up to 60 s
for Warper's process to exit, then starts the new version, and Warper quits through its normal
shutdown, so the usual state save runs. It skips the "Quit Warp?" dialog, because you just
confirmed; unsaved code changes still raise it. If Warper never exits (say, you cancel that
dialog) the relauncher gives up silently (on Linux it still deletes the hidden old binary once
Warper exits). Answer no and the update applies the next time you
launch it. The tab hands Warper's PID to the script, so run from another terminal the script just
installs.

- Linux: updates the install the script sits in (`warp-oss`, `warper-update`, `resources/` and
  `VERSION` in one directory, `~/.local/lib/warp-lean` after `lean/install-linux.sh`); a portable
  copy without `warp-oss` is refused. It stages the whole new install beside it, swaps the pieces
  with renames and puts the old ones back if any step fails. Once the new install is in place the
  old files are deleted, so no previous copy remains (a `warp-oss.previous` left by an older
  updater is removed too). Linux keeps the old binary, hidden as `.warp-oss.old` inside the
  install, only until the old Warper exits, so that Warper still finds the new resources and
  updater through its own path; a detached waiter, started before the restart prompt so closing
  the tab there is harmless, deletes it when that PID is gone (a run from another terminal has no
  PID and leaves it for the next update to replace). If it cannot
  restore the old files it leaves them in the hidden `.warper-update.*` folder next to the
  install and prints the path. If an update is interrupted (a SIGKILL or power loss between the renames), the
  previous files are in that folder; move them back. The script finds its install through
  symlinks too. `VERSION` comes
  from the release tag: the release archive ships it and `lean/install-linux.sh` copies it, but
  removes it for a source build, which the updater then reports as "installed version unknown
  (built from source)".
- macOS: replaces the running `Warper.app` in place, without sudo. The old app waits in the
  hidden `.warper-update.*` folder beside it until the new one is in place (and is moved back if
  that fails; if even that fails the folder is kept and named), then it is deleted, together with
  a `Warper.previous.app` left by an older updater.
  If the app's folder is not writable it says so and stops. The installed version is the bundle's
  `CFBundleShortVersionString`.

## Patches

The terminal-focused removals are gated by `lean`. The behavior-preserving performance
fixes listed below also apply to default builds.

| Path | Change and reason |
| --- | --- |
| `Cargo.toml` | Stripped release-derived `lean` profile with ThinLTO. |
| `app/Cargo.toml` | `lean` feature forwards the executor limit to `warpui_core`. |
| `crates/warpui_core/Cargo.toml` | Adds opt-in `lean` feature. |
| `crates/warp_assets/Cargo.toml`, `crates/warp_assets/src/lib.rs`, `app/Cargo.toml` | Lean does not embed `async/png` (onboarding, launch-modal and Oz imagery that no lean surface shows); `async/jpg` theme backgrounds and `bundled/` stay. A missing asset only fails that image's load. |
| `crates/warpui_core/src/lean.rs`, `keymap.rs`, `platform/menu.rs`, `crates/warp_core/src/context_flag.rs` | Central lean surface/settings policy, binding/menu filters and disabled legacy contexts; also hides Log out, Sign up, Settings sync, the Account page binding and the `account.is_settings_sync_enabled` key. |
| `crates/warpui/src/platform/mac/menus.rs` | Reads the shared display-name helper for macOS app-menu labels. |
| `crates/warpui_core/src/ui_components/link.rs` | Filters hidden help links while keeping privacy links. |
| `crates/settings/src/registration.rs`, `crates/warpui_extras/src/user_preferences/toml_backed.rs` | Excludes hidden settings from registration and prevents writes/deletes without changing the loaded document. |
| `app/src/root_view.rs` | Uses the display-name helper for transferred-tab window titles. |
| `crates/warpui_core/src/async/native/executor.rs` | Caps the shared background executor at four workers. |
| `app/src/features.rs` | Final terminal allowlist removes AI, MCP, Oz, cloud agents, sharing, recording, update, changelog and telemetry flags. Existing user-preference precedence pins exclusions off even if server experiments call `set_enabled`. Runtime feature menu is also disabled. |
| `app/src/settings/input.rs` | Keeps input-style schema copy terminal-focused; both stored styles still persist and sync. |
| `app/src/settings/ai.rs` | Effective AI availability is always false, including after login; stored choices remain intact. |
| `app/src/settings/privacy.rs` | Effective collection stays off; legacy privacy fetch/migration cannot overwrite stored or synced choices. Secret-regex initialization remains. |
| `app/src/drive/settings.rs` | Hides Drive surfaces without disabling settings sync. |
| `app/src/lib.rs` | Never uses the platform keychain (the existing "unavailable" secure storage), so the app is always signed out; disables loopback HTTP, outlining/index restoration, Oz/agent/remote CLI modes custom AI endpoint initialization and AWS AI-credential subscriptions. Normal terminal-server workers remain available. |
| `app/src/ui_components/avatar.rs`, `app/src/workspace/view.rs`, `app/assets/bundled/jpg/warper_avatar.jpg` | The avatar button shows the user's `avatar.png`/`.jpg`/`.jpeg` from the config directory, else the built-in Yoda image (an always-embedded bundled asset); the gear icon is only the loading and decode-failure placeholder. |
| `app/src/lean_update.rs`, `app/src/quit_warning/mod.rs`, `app/src/workspace/mod.rs`, `app/src/workspace/view.rs`, `app/src/app_menus.rs`, `app/src/util/bindings.rs` | Lean-only "Check for Updates…" command (macOS app menu, avatar menu, command palette) opening a tab that runs the bundled `warper-update`; Warper itself makes no request. The restart quit skips the Quit dialog unless code is unsaved. |
| `crates/settings/src/lib.rs` | GUI settings never sync to the cloud: the syncer runs in the same inert mode as the TUI's. |
| `app/src/server/cloud_objects/listener.rs` | Never starts the cloud-objects websocket, even with objects cached from an earlier login. |
| `crates/http_client/src/lib.rs` | Fail-safe: every client resolves hosts through a resolver that always fails and ignores proxies, so no request can leave the machine. |
| `app/src/auth/auth_manager.rs`, `app/src/auth/mod.rs`, `app/src/workspace/view.rs` | Login-gated prompts, sign-up and log-out are inert; log-out would delete the local database. |
| `app/src/menu.rs`, `app/src/settings_view/settings_page.rs`, `app/src/settings/schema_generation.rs` | Filters menu entries, settings widgets and schema keys through the central lean policy. |
| `app/src/util/openable_file_type.rs`, `app/src/banner/view.rs`, `app/src/tab.rs`, `app/src/tips/tip_view.rs` | Uses external file openers and hides removed-feature banners, tab actions and tips. |
| `app/src/system/info.rs` | Stops five-second process-stat sampling; retains the model and initial reading for normal consumers. |
| `app/src/ai/mcp/templatable_manager.rs` | Allows the unused legacy-conversion event only under lean, where its cloud producer is excluded. |
| `app/src/ai/mcp/templatable_manager/native.rs` | Skips the cloud subscription that can migrate/delete MCP objects, restore/migration, built-in Factory MCP and every actual MCP spawn, including explicit spawn requests. |
| `app/src/ai/mcp/file_mcp_watcher.rs` | Reuses the inert watcher; no MCP filesystem scans/subscriptions. |
| `app/src/ai/skills/file_watchers/skill_watcher.rs` | Empty skill watcher; no provider/repository scans or subscriptions. |
| `app/src/ai/llms.rs` | No public or authenticated model-catalog background fetch. |
| `app/src/ai/harness_availability.rs` | No harness/credential background fetch. |
| `app/src/ai/connected_self_hosted_workers.rs` | No cloud-worker discovery requests. |
| `app/src/ai/request_usage_model.rs` | No AI credit/usage requests after login or activation. |
| `app/src/ai/execution_profiles/profiles.rs` | Uses inert local CLI profile state; no AI profile migration or cloud writes. |
| `app/src/settings_view/mod.rs` | Constructs only Appearance, Features, Keybindings, Privacy and About (no Account page) and registers only their palette actions; excluded AI/billing pages cannot start fetches. |
| `app/src/settings_view/update_environment_form.rs` | Suppresses hidden cloud-environment GitHub discovery, including initialization and refocus triggers. |
| `app/src/settings_view/privacy_page.rs` | Hides controls for unavailable telemetry, crash and cloud AI collection. |
| `app/src/settings_view/appearance_page.rs` | Hides agent/notebook font, Drive and agent-tab options, including their palette toggles; the central widget filter also hides Project Explorer. |
| `app/src/settings_view/features_page.rs` | Hides changelog and agent notification controls; retains command notifications. |
| `app/src/terminal/view/init.rs` | Disables unconditional latest-agent-message and save-to-Drive shortcut commands. |
| `app/src/terminal/buy_credits_banner.rs` | Hides the AI-credit purchase banner, including the logged-in zero-credit state. |
| `app/src/terminal/view/use_agent_footer/mod.rs` | No third-party CLI-agent detection/footer or hidden-child wakeup notifications; rich input is separately flag-disabled. |
| `app/src/workspace/mod.rs` | Disables hidden settings/Drive search shortcuts and macOS Oz install actions. |
| `app/src/terminal/universal_developer_input.rs` | Hides the AI mode/context/model toolbar while keeping the normal command editor and saved input style. |
| `app/src/workspace/view.rs` | Hides and disables the legacy AI assistant fallback and removes changelog/billing/referral menu entries. |
| `app/src/uri/mod.rs` | Ignores AI, cloud-agent, MCP, Drive, shared-session and auth deep links and the Account settings route; keeps normal terminal URLs. |
| `app/src/changelog_model.rs` | Rejects every changelog request, including explicit actions. |
| `app/src/app_services/linux/mod.rs` | Enables existing single-instance URL forwarding so a second launch hands its arguments to the running Warper. |
| `app/src/terminal/view.rs` | Drops intercepted `warp://cli-agent` OSC 777 payloads without registering listeners. Lean Linux caps terminal wakeups at 30/s for software rendering; other builds keep 60/s. |
| `app/src/terminal/alt_screen/alt_screen_element.rs` | Lean skips background-color sampling for its hidden footer consumers. |
| `app/src/features_lean_tests.rs`, `app/src/lib_tests.rs`, `app/src/settings/{ai,privacy}_tests.rs`, `app/src/uri/uri_tests.rs`, `app/src/settings_view/update_environment_form_tests.rs` | Policy, startup, privacy and hidden network-fetch regression checks. |

`lean/` owns the Dockerfile, build/install/sync scripts, tooling self-check, benchmark sampler,
results, progress log and a verification report. Screenshots are kept privately under
`/tmp/warp-lean-evidence/`; none are committed because the account avatar is identifying.
The release workflow changes only artifact names for Warper.

The runtime allowlist also disables some non-AI, non-Drive release/experiment flags:

- `UseTantivySearch`: no Tantivy-backed search; ordinary terminal history remains available.
- `GlobalSearch`: no cross-file search tab in the tools panel.
- `TabbedEditorView`: no multi-file editor tab UI; the terminal remains available.
- `CodeFindReplace`: no code-editor or network-viewer find bar; terminal find remains available.
- `FileTree`: no experimental file-tree surface; Project Explorer is hidden by the lean widget/getter policy.
- `SelectablePrompt`: the experimental selectable command-prompt path stays off.
- `VideoRecording` / `WindowsVideoRecording`: no built-in video recording.

The opencode plugin's `OSC 777;notify;warp://cli-agent;<body>` is **dropped**: upstream intercepts
the sentinel before ordinary notifications; lean returns before parsing/registering a listener.
It is not converted into a desktop notification. Ordinary OSC 9/777 notifications are retained.
No plugin changes or support code are included.

The optional present-mode, flat-storage and reference-fork optimizations are omitted unless
later measurements justify them. No GPU timing or storage/selection behavior changes are made.

## Ungated performance fixes

- Terminal writes skip empty selection resets while still clearing pending AI context;
  alt-screen motion skips duplicate cell reports, and the cell glyph cache uses hashbrown.
- PTY reads wait for the terminal lock when the input buffer is full or drained, instead
  of spinning. Buffered EOF data is parsed before reading stops.
- macOS windows skip drawing after an occlusion notification and request a redraw
  when visible again; new windows default to visible.
- Alt-screen hover reports respect the mouse-reporting setting, SGR mode and shared-session
  reader restrictions, as clicks and drags already do.

## Checks and benchmarks

```sh
lean/check-tools.sh
bash -n lean/*.sh
```

The self-check requires `rg` and `python3` in addition to Bash, Git and standard Unix tools.
It checks these prerequisites before creating fixtures. Temporary Cargo/desktop stubs assert
the build interface and installer copy boundaries/idempotence; real local Git repositories
exercise upstream sync and conflict handling. It does
not replace the release build or the real desktop checks. Actual run results and gaps are in
`PROGRESS.md` and the final verification report.

Use Cua Driver for all launch/input/screenshots. Keep screenshots privately under
`/tmp/warp-lean-evidence/`; never commit screenshots. Stable Warp must be closed and no build may be running during benchmarks.
Use the copied settings, matching font, zoom and default 1280x800 window. For **each** run:

1. Launch and settle about 20 seconds. Click window-local `(640,760)` with foreground delivery.
2. `lean/bench.sh <binary> 60 lean-idle-1tab`
3. Start `lean/bench.sh <binary> 25 lean-burst-seq300k` in the background, then type
   `seq 1 300000` and Enter.
4. Press Ctrl+Shift+T four times, 1.5 seconds apart. Wait ten seconds.
5. `lean/bench.sh <binary> 40 lean-idle-5tabs`
6. Quit and repeat the full sequence at least once.

The sampler matches **all** processes by `/proc/PID/exe`. It records CPU seconds and CPU percent
against measured elapsed time, endpoint RSS/PSS/thread count and sampled peak RSS. Its inspection
adds time beyond nominal seconds; compare the same script/protocol and prefer CPU seconds for
the burst. The supplied stable baseline stays unchanged in `bench-results.tsv`.

## Sync with upstream

Start on `lean` with a clean worktree:

```sh
lean/sync.sh
```

It fetches `upstream`, refuses a diverged master, fast-forwards master to upstream/master,
enables repository-local Git `rerere`, then rebases lean onto master. A conflict stops with
the normal rebase state for manual resolution or abort. It never pushes and never edits remotes. A master checked out in another
worktree stops at `git switch master`; resolve that worktree ownership before retrying.
