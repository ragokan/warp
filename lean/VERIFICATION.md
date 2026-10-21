# Verification — Linux arm64

Status: **DONE_WITH_CONCERNS**. Independent-review validation is complete on Linux arm64.
The final review results supersede the historical build/test/UI details preserved below.
Strict Clippy still reports eight unchanged-line diagnostics; the original burst CPU result
is about 5% higher than stable. Remaining verification limits are listed below.

## Independent review, 2026-10-21

| Item | Status and evidence |
| --- | --- |
| 1 | DONE: no installed ImageMagick/PIL; removed all ten PNGs by amending the unpushed evidence commit. No PNG is tracked in master..lean. Originals are private in `/tmp/warp-lean-evidence/committed-before-review/`. |
| 2 | Implemented: every excluded FeatureFlag is tested against `set_enabled(true)`; CloudObjects, FetchGenericStringObjects, SkipFirebaseAnonymousUser and SettingsFile have positive assertions. Lean Cargo result PASS (181 total tests). |
| 3 | Pure URI tables cover 16 allowed routes and 22 rejected cases, including all nine excluded hosts and both handoff spellings. One-off predicate red/green passed; real Cargo green PASS (74 URI tests, including both tables). Privacy test renamed to its effective-getter scope. No sync-filter predicate was extracted solely for tests: those guards are inline in model/GraphQL IO handlers. Existing preference-sync regressions are retained. |
| 4 | Implemented: lean does not register the MCP cloud subscription that can delete legacy objects; websocket content updates return before model/persistence access unless the object is a Preference. |
| 5 | Formatting preview addressed only lean-edited statements; upstream comments retained. Final non-mutating rustfmt --check PASS (exit 0) on all fifteen review-touched Rust files. No flagged diff on upstream or lean lines. Per reviewer, no in-place formatter ran while builds read the tree. |
| 6 | DONE: Project Explorer widget restored; README documents default-denied non-AI flags and ForceCloudToMatchLocal; check-tools requires rg/python3. Tooling self-check and Bash syntax pass. |
| 7 | DONE: five-minute logged-in viewer inspection found hidden cloud-environment `UserGithubInfo` discovery. Shared fetch-gate Cargo red/green PASS; final installed binary's bounded Settings-open log window contains only workspace metadata, no UserGithubInfo. Viewer find is unavailable; visual vertical inspection used instead. Account/theme/settings operations retained. |
| 8 | DONE: lean Cargo suite passed 181/181 across 23 nonempty invocations (14m14s initial compile), including all 44 environment-form tests. Final release build PASS (8m51s), install/ldd/handler PASS. Cua shell/five tabs/Settings/persisted login/sync On PASS; AI query has no AI entries but retains normal fuzzy matches. Final-tree DEFAULT-feature check PASS (36.04s) and tests PASS (151/151, five invocations). |
| 9 | sync.sh enables repository-local rerere and its fixture verifies this. Final history uses the local lean-pre-squash backup and two area commits. Finalization must assert identical tagged/current trees, one commit per upstream file, unchanged master/upstream and a clean worktree. Exact SHAs and post-rewrite results are recorded in the completion message and private history-final.json. |

The original dispatcher-bypass red run failed on a missing model and is superseded by the
pure-predicate assertion test. The one-off harness compiled the actual production enum,
predicate and real table tests using Rust 1.92.0 in the Docker builder:

```text
red: exit 101 — rejection assertion at Team/invite; 1 passed, 1 failed
green: exit 0 — 2 passed, 0 failed
```

The command was `docker run --rm --name warp-lean-review-uri-policy -v
/home/ragokan/coding/warp:/work warp-lean-builder python3 lean/check-uri-policy.py`.
The fragile extraction harness was then moved to
`/tmp/warp-lean-evidence/check-uri-policy.py` and is not committed. The actual regression
suite is `app/src/uri/uri_tests.rs`, run through Cargo with lean enabled.

Final-source commands use the same builder/mount arrangement described below. Lean tests:

```sh
features=$(sed -n 's/^features=//p' lean/build.sh)
for filter in features::tests settings::ai::tests settings::privacy::tests uri::tests \
    ai::mcp::templatable_manager::tests settings_view::update_environment_form::tests; do
    cargo test --locked -p warp --lib --profile lean --no-default-features \
        --features "$features" "$filter" -- --nocapture
done
for test in $(sed -n 's/^fn \([^ (]*\)().*/\1/p' app/src/lib_tests.rs); do
    cargo test --locked -p warp --lib --profile lean --no-default-features \
        --features "$features" "tests::$test" -- --exact --nocapture
done
for filter in test_sync_cloud_pref_to_local_on_initial_load_or_collab_update \
    test_sync_local_pref_to_cloud_after_initial_sync_creates_prefs_setting; do
    cargo test --locked -p warp --lib --profile lean --no-default-features \
        --features "$features" "$filter" -- --nocapture
done
```

Default-feature commands, with `CARGO_TARGET_DIR=/work/target/nonlean` and six Cargo jobs:

```sh
cargo check --locked -p warp --bin warp-oss
for filter in settings::ai::tests settings::privacy::tests uri::tests \
    startup_without_api_key_loads_persisted_auth settings_view::update_environment_form::tests; do
    cargo test --locked -p warp --lib "$filter" -- --nocapture
done
```

The original default-feature loop passed 108 tests but recompiled while implementation
was changing. It is superseded by a detached confirmation on the frozen final tree:

```sh
nohup setsid bash /tmp/warp-lean-evidence/confirm-default.sh \
    > /tmp/warp-lean-evidence/review-nonlean-confirm.log 2>&1 < /dev/null &
```

The script runs the commands above in `warp-lean-builder`, with the same registry/git
volumes and separate `/work/target/nonlean`, then writes its status to
`/tmp/warp-lean-evidence/review-nonlean-confirm.exit`. Actual exit: **0**. Cargo check
passed in **36.04s**; each cached test invocation finished its build phase in 0.82–0.86s.
Counts: AI **34**, privacy **1**, URI **72**, persisted auth **1**, environment form **43**:
**151 passed, zero failed/ignored**. All 43 changed source/manifest SHA256 hashes still
matched the frozen-source record before and after confirmation. No Rust/source changes
followed these checks; subsequent edits are verification documentation only.

Network viewer inspection used Cua only, after 314 seconds logged-in idle. The split pane
was not widened; only vertical scrolling was used. No clipboard API ran, and no screenshot
shows a complete token. Numbered private captures end at page 66 (same image as page 65).
Only operation names were transcribed; HTTP methods are clipped at the approved width.

Distinct operations observed: `GetUser`, `GetUserSettings`, `GetUpdatedCloudObjects`,
`GetWorkspacesMetadataForUser`, `GetReferralInfo`, `UserGithubInfo`. The workspace operation's
visible prefix is uniquely completed by its existing source definition. The shared cloud-object
operation is retained for settings sync; workspace metadata supports account/sync and referral
status supplies terminal-theme unlocks. `UserGithubInfo` is a cloud-agent form discovery gap,
fixed by a three-line early return in the shared fetch method used by eager ambient/handoff form construction.
The real Cargo regression red exited 101 at its `!is_loading` assertion (14m24s build);
the full green form module passed 44/44 with the guard, alongside 137 other lean checks.

Final installed release: **8m51s**, stripped AArch64 ELF, **313969000 bytes**, SHA256
`ec5f52121ae8096cdf52c64fc495ef55607dd046227f03555e6a4886b6428fac`.
Reinstall passed, installed hash matches, `ldd` resolves all libraries, and the `warposs`
handler remains `dev.warp.WarpOss.desktop`. Cua reconfirmed shell output, five tabs,
Settings, persisted login, sync On, the expected six sidebar sections and no AI palette
commands. Private screenshots: `/tmp/warp-lean-evidence/review-netfix-*.png`.

The final viewer has no working find: neither Ctrl+F nor its source-defined Linux
Ctrl+Shift+F opens a search UI. No absence claim is based on typed text. Per the final
review instruction, one unchanged-width vertical batch reached the first Settings-open
entries: 01:39:24 through the next request at 01:40:42 UTC. Captures
`review-netfix-walk-58.png` through `-60.png` show only `GetWorkspacesMetadataForUser`
there, with no `UserGithubInfo`. The Cargo red/green assertion is the primary regression
proof; this was a bounded runtime spot-check, not a second five-minute/full-log audit.

`ss -tnp` filtered to the two lean executable PIDs in each socket snapshot showed only
the peer `34.117.41.85:443`. Reverse lookup returned `85.41.117.34.bc.googleusercontent.com`.
This is an infrastructure PTR, not proof of the TLS virtual host; already-closed connections
are outside this socket snapshot. No header/body was extracted to recover hosts. Sanitized
records are `evidence/review-network-checks.json` and `evidence/review-runtime-checks.json`.
The final runtime scan covers 571 lines and finds no Factory/MCP, classifier, remote-server,
autoupdate, changelog or local HTTP activity. There are zero lean TCP listeners, no stable
process/Oz daemon, and 9/9 stable config/theme hashes still match.

Final targeted normal Clippy passed (2m34s); strict Clippy exited 101 with exactly the
same eight upstream diagnostics listed below. All six affected whole files remain equal
to master, so none are on lean-changed lines. Final non-mutating rustfmt --check passed
all fifteen review-touched Rust files with empty output. No upstream formatting changes
were produced or retained in this review pass.

Final build/lint commands inside the Docker builder were:

```sh
features=$(sed -n 's/^features=//p' lean/build.sh)
cargo clippy --locked -p warp --bin warp-oss --profile lean \
    --no-default-features --features "$features" -- -D warnings
cargo clippy --locked --no-deps -p warp --bin warp-oss --profile lean \
    --no-default-features --features "$features"
mapfile -t rust_files < <(git diff --name-only HEAD -- '*.rs')
RUSTC_BOOTSTRAP=1 rustfmt --check --edition 2024 \
    --config skip_children=true,imports_granularity=Module,group_imports=StdExternalCrate \
    "${rust_files[@]}"
lean/build.sh > lean/review-build-last.log 2>&1
```

The formatter file list was captured before checkpointing the fifteen review edits.
Strict Clippy and formatting exits were recorded without aborting the release build.
Host commands `lean/install-linux.sh`, `ldd ~/.local/lib/warp-lean/warp-oss`, and
`xdg-mime query default x-scheme-handler/warposs` passed. All Rust commands used Docker;
the final release/lint container used twelve Cargo jobs and the cache mounts shown below.

Review logs (`lean/review-*.log`) are gitignored and are not part of the commits.
No review fix changes a terminal rendering/input hot path; the two original benchmark rounds
below were not repeated, as requested.

## Previous acceptance round

Source `dd68ee356`: release build, installed desktop, logs/socket checks and both benchmark
rounds passed. Strict Clippy has unchanged upstream warnings; burst CPU is about 5% higher.

## Rust and tooling

- Docker builder: `warp-lean-builder`, Debian trixie arm64, Rust 1.92.0, 12 Cargo jobs.
- First `lean/build.sh` Docker release build passed (8m29s); the desktop smoke caught residual
  AI UI and missing single-instance login forwarding. Both were fixed before the final build.
- Source through `375a75198` passed seven actual Cargo test invocations in Docker: lean policy
  selection (11), privacy schema (1), persisted auth (1), process-stat construction (1), MCP
  lifecycle (8), cloud-to-local settings sync (1), and local-to-cloud preference creation (1):
  **24 passed, zero failed**. First compile 12m53s; cached invocations under one second each.
  The later palette/banner/Drive binding guards passed Clippy and direct Cua desktop verification.
  The 24 tests were not recompiled after those UI-only guards; a full workspace suite is not claimed.
- `lean/check-tools.sh`, `bash -n lean/*.sh` and `git diff --check` passed. Tool checks exercise
  feature exclusions, safe/idempotent install, fast-forward/rebase, dirty refusal and conflict stop.
- Strict Clippy failed on the eight upstream release warnings below. All six affected paths
  are wholly unchanged in `git diff master..lean`; `git diff --quiet master..lean -- <six paths>`
  returned 0. No diagnostic falls on a changed line.
- Non-strict `cargo clippy --locked --no-deps -p warp --bin warp-oss --profile lean
  --no-default-features --features "$features"` passed on the completed source (2m18s), with those 8 diagnostics and
  2 unchanged warp_terminal warnings (`model/grid/resize.rs:5` unused `report_error`,
  `model/grid/flat_storage/index.rs:586` unnecessary `mut self`); no app diagnostic. Frozen-candidate Clippy also passed
  (`clippy-frozen.log`, 2m15s).

| File:line | Diagnostic |
|---|---|
| `crates/warpui_core/src/elements/gui/new_scrollable/dual_axis_config.rs:6` | Unused `ScrollableAxis` import |
| `crates/warpui_core/src/elements/gui/new_scrollable/single_axis_config.rs:5` | Unused `ScrollableAxis` import |
| `crates/warpui_core/src/core/app.rs:1769` | Unused `binding_validator` parameter |
| `crates/warpui_core/src/core/app.rs:1791` | Unused `binding_validator` parameter |
| `crates/warpui_core/src/elements/gui/new_scrollable/single_axis_config.rs:42` | Unused `axis` parameter |
| `crates/warpui_core/src/elements/gui/rect.rs:154` | Unnecessary `mut self` |
| `crates/warpui_core/src/scene.rs:520` | Unused `location` parameter |
| `crates/warpui_core/src/keymap/matcher.rs:18` | Unread `default_binding_validator` / `binding_validators` fields (second field at line 21) |

Scoped rustfmt ran after each changed candidate's checks. Every retained formatting hunk was checked against master-to-HEAD
changed-line maps. Upstream-line formatting was reverted; the final 6-file formatting diff
touches only lean-added/edited lines. The final two sync files were likewise reviewed after
their tests/lint; only lean-added guard lines changed. The line-map assertions passed.
The final UI formatting pass proposed two upstream-touching hunks, both reverted; formatting
the last two shortcut files produced no changes.

## Commands actually used

The release build and all Rust checks run in `warp-lean-builder`, mounted at `/work`, with
`CARGO_BUILD_JOBS=12` and named volumes only on Cargo's registry and Git caches:

```sh
docker run --rm --name warp-lean-build \
  -v /home/ragokan/coding/warp:/work \
  -v warp-lean-registry:/home/builder/.cargo/registry \
  -v warp-lean-git:/home/builder/.cargo/git \
  -e CARGO_BUILD_JOBS=12 warp-lean-builder \
  bash -c 'lean/build.sh > lean/build-final.log 2>&1'
```

Inside the same builder/mount arrangement, `features` was read from `lean/build.sh`:

```sh
features=$(sed -n 's/^features=//p' lean/build.sh)
cargo clippy --locked -p warp --bin warp-oss --profile lean \
  --no-default-features --features "$features" -- -D warnings
cargo clippy --locked --no-deps -p warp --bin warp-oss --profile lean \
  --no-default-features --features "$features"
cargo test --locked -p warp --lib --profile lean \
  --no-default-features --features "$features" "$filter" -- --nocapture
```

The seven actual test filters were `lean_`, `privacy_settings_apply_to_gui_and_tui`,
`startup_without_api_key_loads_persisted_auth`, `test_memory_usage_stats_construction`,
`ai::mcp::templatable_manager::tests`,
`test_sync_cloud_pref_to_local_on_initial_load_or_collab_update`, and
`test_sync_local_pref_to_cloud_after_initial_sync_creates_prefs_setting`.

`lean/build.sh` is eight lines: resolve the repository, use the explicit 38-feature terminal
list with `lean`/`jemalloc`, invoke locked Cargo without defaults on the `lean` profile, pass
through extra Cargo arguments, and print the binary path. It contains no Docker dependency.

## Desktop, login and sync

Final installed release build passed in **8m00s**. Binary: stripped aarch64 ELF, 314100072 bytes;
SHA256 `af5c61295615e8671fb5af98bcb5f91e7289b62720bca2fbf7ef1adb2128b312`.
`lean/install-linux.sh` passed; installed hash matches; `ldd` resolves all shared libraries.
The desktop handler query returns `dev.warp.WarpOss.desktop` for `warposs`.

Cua verified shell output (`seq 1 20`, `seq 1 300000`), five tabs, Settings, no mode toolbar/credit banner, and no AI entries
in palette queries AI/agent/MCP/Oz/Drive (unrelated fuzzy matches remain). Settings has Account,
Appearance, Features, Keyboard shortcuts, Privacy and About only. Login succeeded through the
browser callback, persists after restart, and Settings sync is visibly **On**. The filtered sync
capture hides the account identity. Private browser/account images stay only in `/tmp`.
The two test-created browser tabs were closed through Cua; the user's separate window was left.

Ordinary OSC9 and OSC777 notification toasts render. The plugin sentinel is **dropped**, with
no raw payload, notification or CLI-agent toolbar; the shell reaches test completion. No plugin
support code was added. An agent deep-link helper exited without changing the existing window;
Cua called this a handoff timeout because no new window appeared. No second app remained.

Screenshot evidence is private-only under `/tmp/warp-lean-evidence/committed-before-review/`:
`settings-sidebar.png`, `settings-sync.png`, `palette-ai.png`, `palette-agent.png`,
`palette-mcp.png`, `palette-drive.png`, `osc9.png`, `osc777.png`, `cli-agent-dropped.png`,
and `seq-five-tabs.png`. These show the account avatar. Neither ImageMagick nor Python Pillow
was installed, so all ten PNGs were removed by amending the unpushed evidence commit rather
than installing an image tool. No evidence PNG remains committed. The generic Open Left Panel
palette action exposed no Drive UI and left the terminal unchanged.

## Network and processes

Ran `ss -ltnp` on the host: **zero TCP listeners owned by either lean process** (GUI plus
normal terminal worker); no Oz remote daemon. The final post-benchmark scan covers 755 retained
log lines from this build and finds zero
Factory/MCP, ONNX/input-classifier, remote-server, autoupdate, changelog or local-HTTP activity
mentions after excluding static channel configuration. This combines observed log/socket state
with the hard service guards and feature-override tests; it is not TLS packet inspection.

At the URI test, its short helper rotated the log: the GUI FD pointed to
`warp-oss.log.old.temp` while the newer `warp-oss.log` was empty. The check followed the real
descriptor rather than accepting an empty log. The final scan also covers the rotated and current logs from subsequent
benchmark launches. Sanitized counts: `evidence/runtime-checks.json`; raw logs/`ss` output only under
`/tmp/warp-lean-evidence/` because they may contain private metadata.

## Benchmarks

Both complete lean rounds used the unchanged sampler and 60/25/40 sample counts, twenty-second
initial settle, foreground `(640,760)` click, immediate `seq 1 300000` after starting the burst,
four Ctrl+Shift+T presses 1.5 seconds apart, and ten-second five-tab settle. Font SFMono Nerd Font
17, zoom 110%, window 1280x800. No build/stable app and no overlapping samplers. Every row counts
both GUI and terminal-worker processes. All six detached completion files report exit 0.

The first attached idle sampler hit the tool timeout (SIGTERM), produced no row, and was
excluded. The full first round was restarted fresh. Subsequent samplers ran via `nohup setsid`
with logs/exit files in `/tmp`; `bench.sh` is byte-for-byte unchanged from baseline commit
`f205ad6a2` (`git diff --quiet f205ad6a2 -- lean/bench.sh` returned 0).

| Scenario/run | Time | CPU % | CPU-s | RSS MB | PSS MB | Peak RSS MB | Threads |
|---|---|---:|---:|---:|---:|---:|---:|
| Stable: idle 1 tab | 21:16:08 | 0.62 | 0.62 | 583 | 517 | 583 | 134 |
| Lean 1: idle 1 tab | 23:09:57 | 0.35 | 0.36 | 308 | 242 | 310 | 121 |
| Lean 2: idle 1 tab | 23:17:45 | 0.35 | 0.36 | 309 | 243 | 311 | 121 |
| Stable: seq 300k | 21:16:56 | 9.41 | 4.02 | 585 | 519 | 585 | 134 |
| Lean 1: seq 300k | 23:12:12 | 9.67 | 4.21 | 309 | 242 | 309 | 121 |
| Lean 2: seq 300k | 23:20:26 | 9.27 | 4.23 | 310 | 244 | 310 | 121 |
| Stable: idle 5 tabs | 21:18:37 | 1.54 | 1.02 | 607 | 539 | 607 | 138 |
| Lean 1: idle 5 tabs | 23:14:18 | 0.29 | 0.20 | 329 | 263 | 331 | 125 |
| Lean 2: idle 5 tabs | 23:22:51 | 0.30 | 0.21 | 332 | 265 | 332 | 125 |

Against the supplied stable baseline, the two-run mean one-tab idle CPU is 44% lower,
RSS 47% lower; five-tab idle CPU is 81% lower, RSS 46% lower. Both scenarios use 13 fewer
threads. Mean burst CPU is **4.22 CPU-s vs 4.02 CPU-s**, about **5% higher**. No burst speedup
is claimed. Nominal samples take longer in wall time because of `/proc` scans; percentages
use actual elapsed time, and the burst comparison uses CPU seconds.

Actual invocations were `lean/bench.sh /home/ragokan/.local/lib/warp-lean/warp-oss` followed by
`60 lean-idle-1tab`, `25 lean-burst-seq300k`, and `40 lean-idle-5tabs`, twice in that order.

## Limits

macOS build/packaging belongs to the separate CI worktree; it was not verified here.
No optional present-mode, grid-storage, font-scan or reference-fork patches were applied.
The full workspace and every shortcut/edge-menu path were not exhaustively tested; the code
freeze requested a separate independent review next. No additional observed AI/Drive leftover
remained in the exercised UI. Cross-device cloud-sync propagation was not tested; the switch,
retained cloud connection, and both existing preference-sync unit tests were checked.

The default-denied `CodeFindReplace` flag also removes network-viewer find
(`app/src/code/editor/view/actions.rs:605`); runtime inspection used vertical scrolling.
This is a non-AI feature limitation, not a newly enabled network path.

The desktop notification parser/toast paths were exercised; an unfocused OS-level notification
was not separately captured. The source preserves that upstream routing branch. The generic
agent deep-link Cua handoff reported timeout rather than a success return, as noted above.

Stable safety: nine baseline config/theme files have identical SHA256 hashes; no stable app
process or Oz daemon remained. `master` and `upstream/master` both remain
`831f4fa9fa1585219b57331f1b17253309f5840d`; workflows and remote configuration were not modified.
No commits were pushed. No stale daemon needed termination.

## Historical implementation checkpoints

```text
f205ad6a2 lean: record Linux builder and stable benchmark baseline
6427a2735 lean: add explicit terminal build and bounded executor profile
103436ae9 lean: pin AI and cloud-agent services off while retaining auth and sync
08c08210b lean: hide AI and Drive settings and disable CLI-agent UI
799da3763 lean: supply protobuf headers and record startup decisions
cfdea12ad lean: prevent post-login AI usage and profile work
af4a674c0 lean: add isolated user install, upstream sync and tooling checks
b799ab52e lean: close settings shortcuts and preserve account privacy preferences
a565e9e9f lean: gate remaining appearance controls and verify post-login policy
b786042f8 lean: fix visible AI controls and Linux login callback forwarding
34ba539f5 lean: avoid constructing AI settings pages and reject agent deep links
417d2ed21 lean: format only changed lines and record targeted verification
bdf6f3b5d lean: restrict cloud sync to preferences and stop agent timestamp fetches
375a75198 lean: finish sync guard formatting and record completed Rust checks
4bff6a7fd lean: remove post-login AI palette actions and credit banner
dd68ee356 lean: close Drive search and workflow keyboard shortcuts
```

These pre-review implementation checkpoints will remain reachable through the local
`lean-pre-squash` tag. The final branch history contains one source/test commit and one
lean tooling/docs/evidence commit; obtain their hashes with `git log --oneline master..lean`.
Nothing is pushed.

History finalization is intentionally performed after writing this report, so
`git diff --exit-code lean-pre-squash lean` can remain empty. Its final assertions and
commit hashes are saved outside the tagged tree at
`/tmp/warp-lean-evidence/history-final.json` and in the completion message. The last step
also verifies every changed upstream path appears in exactly one commit and that
`master`/`upstream/master` remain at the recorded upstream revision.

## Changed paths

```text
Cargo.toml
app/Cargo.toml
app/src/ai/connected_self_hosted_workers.rs
app/src/ai/execution_profiles/profiles.rs
app/src/ai/harness_availability.rs
app/src/ai/llms.rs
app/src/ai/mcp/file_mcp_watcher.rs
app/src/ai/mcp/templatable_manager.rs
app/src/ai/mcp/templatable_manager/native.rs
app/src/ai/request_usage_model.rs
app/src/ai/skills/file_watchers/skill_watcher.rs
app/src/app_services/linux/mod.rs
app/src/changelog_model.rs
app/src/drive/settings.rs
app/src/features.rs
app/src/features_lean_tests.rs
app/src/lib.rs
app/src/lib_tests.rs
app/src/server/cloud_objects/update_manager.rs
app/src/server/server_api/object.rs
app/src/settings/ai.rs
app/src/settings/ai_tests.rs
app/src/settings/privacy.rs
app/src/settings/privacy_tests.rs
app/src/settings_view/appearance_page.rs
app/src/settings_view/features_page.rs
app/src/settings_view/main_page.rs
app/src/settings_view/mod.rs
app/src/settings_view/privacy_page.rs
app/src/settings_view/update_environment_form.rs
app/src/settings_view/update_environment_form_tests.rs
app/src/system/info.rs
app/src/terminal/buy_credits_banner.rs
app/src/terminal/universal_developer_input.rs
app/src/terminal/view.rs
app/src/terminal/view/init.rs
app/src/terminal/view/use_agent_footer/mod.rs
app/src/uri/mod.rs
app/src/uri/uri_tests.rs
app/src/workspace/mod.rs
app/src/workspace/view.rs
crates/warpui_core/Cargo.toml
crates/warpui_core/src/async/native/executor.rs
lean/.gitignore
lean/Dockerfile.linux
lean/PROGRESS.md
lean/README.md
lean/VERIFICATION.md
lean/bench-results.tsv
lean/bench.sh
lean/build.sh
lean/check-tools.sh
lean/evidence/review-network-checks.json
lean/evidence/review-runtime-checks.json
lean/evidence/runtime-checks.json
lean/install-linux.sh
lean/sync.sh
```
