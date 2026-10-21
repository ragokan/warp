# Warp Lean

A terminal-focused `warp-oss` build from branch `lean`. Account login and cloud settings sync
remain upstream implementations. Stable Warp's files are read only and are never used as the
OSS authentication store. `master` stays at upstream; nothing is pushed by this tooling.

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

## Install and login

```sh
lean/install-linux.sh
```

The installer puts the binary in `~/.local/lib/warp-lean/warp-oss`, adds
`~/.local/bin/warp-lean`, installs the OSS icon and a desktop launcher, generates the settings
schema beside the executable, and registers `warposs://` for login callbacks. Terminal fonts,
shell bootstrap and normal UI assets are embedded by the upstream release asset provider.
No Oz skills or remote-server binaries are installed.

OSS directories follow XDG overrides, otherwise:

- Config: `~/.config/warp-oss`
- Data: `~/.local/share/warp-oss`
- State/logs: `~/.local/state/warp-oss`
- Cache: `~/.cache/warp-oss`

Only missing `settings.toml`, `keybindings.yaml`, `themes` are copied from the
corresponding `warp-terminal` directories. Existing OSS copies are preserved. Legacy
`user_preferences.json` is omitted because it contains free-form saved prompts and cannot be
established to be secret-free. No auth files, tokens, keychain entries or databases are migrated.
Login therefore needs its own browser callback. The fixed loopback HTTP server is only for
installation detection and profiling; authentication uses the registered URL scheme.

Settings sync remains enabled when the copied `account.is_settings_sync_enabled`
setting is true. The Account page exposes it after login. Preference startup fetches, the 600-second poll with 20% jitter and uploads remain. The
cloud-object websocket is disabled; workspace metadata keeps initial/on-demand fetches only.
Other-device changes arrive within about 10–12 minutes. There is no catch-up refresh after
wake from sleep; network-online events still trigger a poll. Sync uses last-write-wins: a local
edit shortly after another device edits the same key can be overwritten at the next poll.
Workspace/enterprise metadata refreshes only at startup or on demand.

Turning Settings sync off and then on uses upstream `ForceCloudToMatchLocal`: cloud settings
are overwritten with this lean installation's local values. The first install copied the
stable installation's settings; review local choices before toggling sync back on.

Lean also hides AI/Drive/MCP/notebooks, analytics/crash-reporting actions, sharing, teams,
billing and referrals from bindings, slash commands, menus, tips, login copy, banners,
tooltips and the settings schema.
Hidden preferences remain stored: registration excludes them from sync and duplicate cleanup;
the TOML backend refuses their writes/deletes while preserving the loaded document.
Lean no longer performs the legacy privacy migration or GetUserSettings, so eleven upstream
tests are lean-excluded: ten passed at c13eb5653; duplicate-cleanup already failed there because
it expected a fourth operation for the cloud-agent timestamp fetch that lean skips.
`notifications.preferences` is the preservation exception: its full compound schema/value
remains so ordinary terminal notifications still persist and sync; the agent row is hidden.

Also hidden: local/cloud workflows; Warp's editor/review/file tree and Markdown viewer;
tab/launch configurations, groups/pins/colours/vertical tabs; block bookmarks/filter/sticky
headers/compact mode; community/help links; Vim and command corrections (error underlining and suggested fixes). File and Markdown
links use the selected external editor, `$EDITOR`, or the system default. Plain tabs, rename,
blocks, Find, copy and syntax highlighting remain. The “Show Warp network log” binding
(`input:insert_network_logging_workflow`) is hidden with Workflows; the Privacy-page viewer stays.

## Patches

Every upstream change is gated by `lean`; default upstream behavior is retained.

| Path | Change and reason |
| --- | --- |
| `Cargo.toml` | Stripped release-derived `lean` profile with ThinLTO. |
| `app/Cargo.toml` | `lean` feature forwards the executor limit to `warpui_core`. |
| `crates/warpui_core/Cargo.toml` | Adds opt-in `lean` feature. |
| `crates/warpui_core/src/lean.rs`, `keymap.rs`, `platform/menu.rs`, `crates/warp_core/src/context_flag.rs` | Central lean surface/settings policy, binding/menu filters and disabled legacy contexts. |
| `crates/warpui_core/src/ui_components/link.rs` | Filters hidden help links while keeping privacy links. |
| `crates/settings/src/registration.rs`, `crates/warpui_extras/src/user_preferences/toml_backed.rs` | Excludes hidden settings from registration and prevents writes/deletes without changing the loaded document. |
| `crates/warpui_core/src/async/native/executor.rs` | Caps the shared background executor at four workers. |
| `app/src/features.rs` | Final terminal allowlist removes AI, MCP, Oz, cloud agents, sharing, recording, update, changelog and telemetry flags. Existing user-preference precedence pins exclusions off even if server experiments call `set_enabled`. Runtime feature menu is also disabled. |
| `app/src/settings/input.rs` | Keeps input-style schema copy terminal-focused; both stored styles still persist and sync. |
| `app/src/settings/ai.rs` | Effective AI availability is always false, including after login; stored choices remain intact. |
| `app/src/settings/privacy.rs` | Effective privacy snapshot refuses all telemetry and crash/AI collection without overwriting stored or synced choices. |
| `app/src/drive/settings.rs` | Hides Drive surfaces without disabling settings sync. |
| `app/src/lib.rs` | Keeps Linux login callback forwarding without `release_bundle`; disables loopback HTTP, outlining/index restoration, Oz/agent/remote CLI modes custom AI endpoint initialization and AWS AI-credential subscriptions. Normal terminal-server workers remain available. |
| `app/src/server/server_api/object.rs` | Skips parsing AI/MCP/cloud-agent generic objects from the shared settings-sync response. |
| `app/src/server/cloud_objects/update_manager.rs` | Applies only preference generic objects, rejects non-preference websocket content updates, ignores the MCP gallery, and stops cloud-agent environment timestamp fetches. Folder/account metadata and settings synchronization remain enabled. |
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
| `app/src/settings_view/mod.rs` | Constructs only Account, Appearance, Features, Keybindings, Privacy and About and registers only their palette actions; excluded AI/billing pages cannot start fetches. |
| `app/src/settings_view/update_environment_form.rs` | Suppresses hidden cloud-environment GitHub discovery, including initialization and refocus triggers. |
| `app/src/settings_view/privacy_page.rs` | Hides controls for unavailable telemetry, crash and cloud AI collection. |
| `app/src/settings_view/appearance_page.rs` | Hides agent/notebook font, Drive and agent-tab options, including their palette toggles; keeps the Project Explorer visibility setting. |
| `app/src/settings_view/main_page.rs` | Hides AI-plan upsells/referrals while retaining login, logout and settings sync. |
| `app/src/settings_view/features_page.rs` | Hides changelog and agent notification controls; retains command notifications. |
| `app/src/terminal/view/init.rs` | Disables unconditional latest-agent-message and save-to-Drive shortcut commands. |
| `app/src/terminal/buy_credits_banner.rs` | Hides the AI-credit purchase banner, including the logged-in zero-credit state. |
| `app/src/terminal/view/use_agent_footer/mod.rs` | No third-party CLI-agent detection/footer; rich input is separately flag-disabled. |
| `app/src/workspace/mod.rs` | Disables hidden settings/Drive search shortcuts and macOS Oz install actions. |
| `app/src/terminal/universal_developer_input.rs` | Hides the AI mode/context/model toolbar while keeping the normal command editor and saved input style. |
| `app/src/workspace/view.rs` | Hides and disables the legacy AI assistant fallback and removes changelog/billing/referral menu entries. |
| `app/src/uri/mod.rs` | Ignores AI, cloud-agent, MCP, Drive and shared-session deep links; keeps auth and normal terminal URLs. |
| `app/src/changelog_model.rs` | Rejects every changelog request, including explicit actions. |
| `app/src/app_services/linux/mod.rs` | Enables existing single-instance URL forwarding for lean Linux login callbacks. |
| `app/src/terminal/view.rs` | Drops intercepted `warp://cli-agent` OSC 777 payloads without registering listeners. |
| `app/src/features_lean_tests.rs`, `app/src/lib_tests.rs`, `app/src/settings/{ai,privacy}_tests.rs`, `app/src/uri/uri_tests.rs`, `app/src/settings_view/update_environment_form_tests.rs` | Policy, startup, privacy and hidden network-fetch regression checks. |

`lean/` owns the Dockerfile, build/install/sync scripts, tooling self-check, benchmark sampler,
results, progress log and a verification report. Screenshots are kept privately under
`/tmp/warp-lean-evidence/`; none are committed because the account avatar is identifying.
`.github/workflows/` belongs to the separate CI worktree and is not changed here.

The runtime allowlist also disables some non-AI, non-Drive release/experiment flags:

- `UseTantivySearch`: no Tantivy-backed search; ordinary terminal history remains available.
- `GlobalSearch`: no cross-file search tab in the tools panel.
- `TabbedEditorView`: no multi-file editor tab UI; the terminal remains available.
- `CodeFindReplace`: no code-editor or network-viewer find bar; terminal find remains available.
- `FileTree`: no experimental file-tree surface; the Project Explorer visibility setting is retained.
- `SelectablePrompt`: the experimental selectable command-prompt path stays off.
- `VideoRecording` / `WindowsVideoRecording`: no built-in video recording.

The opencode plugin's `OSC 777;notify;warp://cli-agent;<body>` is **dropped**: upstream intercepts
the sentinel before ordinary notifications; lean returns before parsing/registering a listener.
It is not converted into a desktop notification. Ordinary OSC 9/777 notifications are retained.
No plugin changes or support code are included.

The optional present-mode, flat-storage and reference-fork optimizations are omitted unless
later measurements justify them. No GPU timing or storage/selection behavior changes are made.

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

Use Cua Driver for all launch/input/screenshots. Keep non-Warp screenshots in
`/tmp/warp-lean-evidence/`, and review Warp screenshots for emails/tokens before committing them
under `lean/evidence/`. Stable Warp must be closed and no build may be running during benchmarks.
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
