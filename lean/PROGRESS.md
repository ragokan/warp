# Lean build progress

## 2026-10-21 — Update Warper command

- New `lean/warper-update` (POSIX shell, the only networked code and only while it runs): latest
  `lean-vX.Y.Z` from the GitHub API, semver comparison, confirmation, archive + `SHA256SUMS`
  download, checksum verification (missing or mismatching aborts), install (Linux: the whole new
  install is staged, swapped in with renames and rolled back if a step fails; macOS: the old app
  stays as `Warper.previous.app`), optional restart:
  a detached relauncher waits up to 60 s for Warper's PID to exit (no signal is ever sent), then
  starts the new version, and Warper quits through its normal Quit action once the script leaves a
  marker file (Warper has no SIGTERM handler, so signalling it would skip the quit steps). It ships
  in the Linux archive and in `Warper.app/Contents/Resources`;
  `verify-bundle.py` requires it to be present and executable, and the release job attaches
  `SHA256SUMS` to the draft. The script updates the install it sits in (not a guessed prefix). The
  Linux archive ships `VERSION` (from the tag, checked by `verify-bundle.py`); `install-linux.sh`
  copies it or removes a stale one, and an unknown version is reported and confirmed, not
  treated as 0.0.0.
- Lean-only "Check for Updates…" (existing `WorkspaceAction::CheckForUpdate`, a new
  `CustomAction::CheckForUpdates` for the macOS menu, an avatar-menu item and a command-palette
  binding) opens a new terminal tab running the script from next to the executable
  (`Resources` inside the macOS bundle) with Warper's PID and a marker path; Warper polls the
  marker once a second for 30 minutes. Warper itself makes no request.
- Tested: twenty-five script tests against a local fake release (older, equal and newer, declined,
  unknown or garbage version, portable copy refused, checksum mismatch and missing checksum abort
  without touching the install, an incomplete archive and a failure after the binary swap leave the
  old install intact, a failed restore or a failed backup placement keeps the staging folder (and
  names it), a symlinked script updates its own install, a Linux install keeps the previous binary
  and leaves no temporary files, the
  relauncher waits for the PID without signalling it and then starts the new binary, gives up on a
  PID that never exits, declining the restart, the relauncher's own session when `setsid` is
  missing as on macOS (Warper kills the finished command's process group), macOS bundle
  replacement keeping `Warper.previous.app` (also when placing it fails), mismatch, unwritable, `install-linux.sh` copying or
  removing `VERSION`); four `verify-bundle` tests (script and `VERSION` required, version from the
  release tag); four Rust tests for the command line, quoting and the marker; the lean surface
  test asserts the command is registered. Run against the previous script and installer, 24 of
  the 25 tests in `lean/ci` failed, covering each of these fixes.
- Checked on the real Linux desktop (isolated XDG dirs, fake release through the env overrides):
  the avatar-menu item opens the "Update Warper" tab, the install keeps `warp-oss.previous`, and on
  "restart" Warper quit through its normal shutdown (SQLite writer, terminal server, LSP teardown)
  and the new version started from the relauncher. A relauncher whose PID never exits gave up
  silently after 60 s and the app kept running.
- Unverified here: the macOS menu item, bundle replacement and relauncher (written but not compiled
  or run on Linux; the suite runs on the macOS runner in CI) and a real GitHub release.

## 2026-10-21 — unused bundled assets

- `warp_assets` gains a `lean` feature (enabled by the app's `lean` feature) that excludes
  `async/png` (50 MB: onboarding, launch-modal and Oz imagery) from the embedded assets. Onboarding
  needs `FeatureFlag::AgentOnboarding`, which no lean build enables, and the modals belong to
  removed surfaces; a missing asset only fails that image's load. `async/jpg` (theme backgrounds)
  and `bundled/` stay.
- Tested: `async_assets_follow_the_lean_and_standalone_features` failed with the exclusion removed
  and passes with it, with the default features and with `standalone` (which drops all of `async/`).
- Measured (Docker, `lean` profile, same toolchain, `lean/build.sh` features): baseline
  9ff726424 is 313,836,120 bytes, commits 1–3 together 262,259,240 (−51,576,880, −16.4%); the
  removed PNGs are 51,500,874 bytes, so the offline and avatar changes net about −76 KB.
- Not done: dropping tree-sitter/arborium grammars (hard-coded per language in
  `crates/languages/src/lib.rs`, so each would need its own edit and its own test) and a smaller
  `opt-level` (changes terminal hot-path speed; needs a GUI measurement first).

## 2026-10-21 — avatar from the config directory

- The avatar button shows `avatar.png`, `avatar.jpg` or `avatar.jpeg` from the config directory
  (first match, resolved once per run); the gear icon remains the placeholder while loading and
  the fallback when the file is missing or undecodable. A new `AvatarContent::LocalFile` variant
  reuses the upstream `Image` element with `AssetSource::LocalFile`; no asset ships and no
  dependency was added.
- Tested: two unit tests for the file-name precedence (red without the lookup, green with it);
  full lean `warp --lib` suite shows no failure beyond the 355 at baseline 9ff726424. Default-
  feature `cargo check -p warp --lib` passes.
- Unverified here: the real desktop rendering (circle crop, placeholder, undecodable file).

## 2026-10-21 — fully offline

- Warper no longer signs in or syncs. One lean gate registers the existing "unavailable" secure
  storage, so startup never reads the keychain (always signed out) and no code path can delete
  credentials an earlier Warper or Warp stored. Upstream's signed-out guards already stop workspace
  polling, cloud-object polling and referral lookups; the settings syncer runs in its inert
  (TUI) mode. Login-gated prompts, sign-up and log-out are inert (log-out deletes the local
  database). Log out, Sign up, Settings sync, the Account page and `warposs://auth` are gone.
- Fail-safe: `http_client` clients resolve hosts through a resolver that always fails and ignore
  proxies, so any path missed above is refused before a connection opens.
- Removed the now-dead poll-only/preference-only sync hunks (workspace poll, cloud-object filters,
  duplicate-cleanup gate), the login-copy hunks and the `lean_preservation` syncer tests; seven
  upstream tests that these hunks had broken under lean pass again. The cloud-objects listener keeps
  its lean gate: objects cached from a v0.2.0 login would otherwise make it retry its websocket
  (it fails at "missing authentication credentials" before connecting); a test covers cached objects.
- Red/green (Docker, lean profile): six new or updated lean tests failed on the old tree (secure
  storage, settings sync, login-gated prompt/log-out, loopback connection, registered surfaces,
  URI allowlist) and pass now. Full lean `warp --lib` suite (hung modules skipped): baseline
  9ff726424 has 355 failures, this tree 330; no test fails that did not fail at baseline (four
  mockito tests in `server_api` are flaky at baseline as well). Twenty-two `terminal/view_tests.rs`
  tests and six syncer tests that assert removed surfaces are lean-excluded.
- Unverified here: the real desktop (no sign-in UI, settings live reload, zero TCP connections).

## 2026-10-21 — terminal performance plan

- Selection-reset, cell-motion dedupe, glyph-cache hashing, PTY lock waiting, macOS
  occlusion and hover-reporting fixes apply to both builds. Hidden footer/sampler
  work and the 30/s Linux wakeup cap remain lean-only. No dependencies were added.
- Four new regressions failed real Cargo assertions before their fixes, then passed
  with lean and default features. Pending AI context without a selection is covered.
- Full view suite: final 163 passed/20 failed; baseline 161 passed/22 failed, including
  the two expected new failures. The same 20 baseline failures remain unchanged.
  Grid: 5 passed/1 ignored; alt-screen: 1; event-loop: 3; lean surface: 4; lean filters:
  16; cloud preferences: 9; URI: 74; warpui_core lean: 2. Normal targeted Clippy passed.
- PTY throughput gate passed on Linux: baseline median 2.362s, commit-2 median
  2.265s (-4.1% elapsed); CPU medians 23.69s and 23.34s (-1.5%). Three matched
  seq-3M runs per build. Other Warp build containers were paused/restored for each
  after sample; no concurrent build was present during the baseline samples.
- Scoped rustfmt preserves pre-existing formatting. The approved Linux builder has
  no clang-format; Objective-C follows surrounding style and requires macOS CI.
- Private logs and desktop/benchmark evidence stay under /tmp/warp-lean-evidence/perf/.
  Native macOS occlusion behavior and macOS bulk-output throughput require user hardware.

## 2026-10-21 — product removal, poll-only sync and Warper

- Four authorized areas: remaining AI/Drive/MCP/telemetry surfaces; selected optional
  terminal/editor features; poll-only settings sync; display/package rename to Warper.
  No dependencies, pushes, tags, stable-directory writes or committed screenshots.
- Central binding/menu/slash/widget/schema policies hide the removed areas. Deleted seven
  compile-feature entries and corresponding runtime allowlist entries. Effective getters
  disable behavior without changing stored values. File links keep external opening.
- Hidden preferences are excluded from registration, cloud uploads and duplicate cleanup;
  the TOML backend inhibits their writes/removals while retaining the full document.
  The legacy privacy fetch/migration is disabled after retaining secret-regex initialization.
  notifications.preferences remains a compound preservation exception; its agent UI is hidden.
- Red/green: registered-surface enumeration, orphan-action policy and legacy privacy
  migration each failed an assertion before its fix. Evidence stays private under
  /tmp/warp-lean-evidence/removal/; no source-extraction harness is committed.
- All original syncer helpers and test bodies are unchanged. Eleven migration-specific
  tests are excluded only under lean. Isolated c13 baseline: 16 passed, duplicate cleanup
  alone failed (4 expected operations vs 3; lean already skips agent timestamps).
  Ten exclusions follow this round's intentional legacy privacy migration/GetUserSettings removal.
- New lean tests cover hidden-key preservation, local upload, cloud application and no
  equal-value re-upload. Diagnostic actual queue was empty; its historical future vector
  was not. The final test uses upstream's unchanged-request-count assertion plus queue/revision checks.
- Frozen final source: 59 Rust hashes. Docker lean fast checks passed, full syncer module
  8 passed, selected modules/shared crates 304 additional executions passed, release build
  via lean/build.sh passed in 8m11s.
  Final default-feature app/shared checks pass; 124 tests pass (syncer17/privacy1/AI34/URI72),
  with all upstream syncer helpers and assertions unchanged.
- Non-mutating rustfmt reports six deferred statements: five span upstream lines; the
  lean-only URI expression is deferred by the explicit source-freeze instruction. No
  upstream reformatting was retained. Eight earlier strict-Clippy diagnostics are unchanged master lines.
- Cua final target binary and portable warper executable launched with the same OSS identity.
  Login persists, sync On untoggled, Monokai Pro Spectrum/SFMono Nerd Font17/110% unchanged.
  Full Appearance/Features/shortcuts scrolls, 19 palette queries and +/tab/block menus pass.
  Network-log content renders; its temporary screenshot was deleted. Generic input folding stays.
- Settled-pointer Ctrl-clicks on .html/.md/.rs all reached the temporary xdg-open probe.
  Initial clicks preceded link highlighting; no code fix was needed. Probe instance closed;
  the portable build runs with normal PATH. No Chromium window was closed or killed.
- Real TOML SHA and nanosecond mtime are identical after launch and harmless resize.
  toml_edit 0.25.6 supports this TOML 1.1 file; zero parse errors, startup preference load
  completion observed. All nine stable file hashes remain identical.
- Linux bundle fixture tests and actual ELF/resource/schema verifier pass. Embedded macOS
  plist differs only in the two display names; native macOS runtime remains unverified.
- Idle socket check after 217 seconds: both portable-build PIDs have zero TCP connections
  and zero listeners. Installed hash 5d955be2075bf75d95f632de97462636597aeb11c72bafa81f55dfe6b796b291
  matches the checked target. Old hash ec5f5212 is preserved as warp-oss.lean-v0.1.0.
- Cua confirms the installed launcher/window name Warper, same login and sync On, shell seq
  output, normal PATH and unchanged OSS identity. Installed PID1351346 is left running.
  Real settings SHA/mtime and nine stable hashes still match after installation.
- Four local area commits follow this record. Exact commands, results, screenshot paths,
  individual test-exclusion reasons and per-commit LoC are in the private removal/REPORT.md.
  No native macOS check, cross-device preference mutation or new performance benchmark.

## 2026-10-20 — baseline preserved

- Branch `lean` started at upstream/master `831f4fa9f`; master is untouched.
- Committed the supplied Dockerfile, benchmark script and stable measurements as `f205ad6a2`.
- Docker builder inspected: `warp-lean-builder`, arm64, image `fcc99f87b684`.
- Host builds are excluded; Cargo runs in Docker with registry/git cache volumes only.
- Scope: Cargo profile/feature, final runtime flag filter, AI/MCP/indexing startup guards,
  privacy accessors, UI entry gates, bounded executor, install/sync tooling and evidence.
- Preserve auth, settings sync and terminal functions. No dependencies, remote changes,
  workflow edits, reference-fork execution, or stable configuration writes.
- Cua Driver 0.37.0 CLI is available; no cua MCP tools are exposed in this session.
- macOS verification belongs to the separately coordinated CI agent; no delegation here.

Next: finish tracing startup, auth callbacks and UI gates; implement lean feature and build.

## Runtime gates implemented

- Added `lean/build.sh`, profile `lean` (ThinLTO, stripped, no debug info), and four-worker cap.
- OSS auth callbacks use `warposs://`; local HTTP routes only serve installation/profiling.
- Final flag allowlist pins excluded flags using the existing user-preference precedence,
  preventing server experiments (`set_enabled`) and the disabled runtime menu from reviving them.
- AI/Drive accessors preserve stored settings; MCP spawning, scans, skill watches, model-list
  fetches, local HTTP, outlining/index restoration and periodic process sampling are gated.
- Structured `warp://cli-agent` OSC notifications are consumed and dropped under lean;
  ordinary OSC 9/777 notifications remain enabled. No opencode support was added.
- First Docker build failed on root-owned cache volumes; fixed only those volume owners.
- Next build reached `warp-proto` then failed: missing `google/protobuf/descriptor.proto`.
  Added the missing `libprotobuf-dev` build prerequisite to the Dockerfile.

Next: rebuild image, complete install/sync checks, then finish build and desktop verification.

## Tooling and privacy checks

- Builder image rebuilt successfully with protobuf headers; dependency compilation resumed.
- Cua CLI must run outside the filesystem sandbox to reach the existing desktop daemon.
  Verified its window listing and capture API. Non-Warp capture moved to
  `/tmp/warp-lean-evidence/browser-before.png`; never staged.
- Installer targets `~/.local/lib/warp-lean`, `~/.local/bin/warp-lean`, OSS XDG dirs,
  icon and `warposs://` desktop handler. Existing OSS settings are never overwritten.
- Legacy JSON includes SavedPrompt; omit the entire store rather than copy possible secrets.
  Stable settings.toml explicitly has `is_settings_sync_enabled = true`.
- Added guards for AI usage/credit fetches and legacy AI profile initialization; these run
  on authentication even when AI UI is hidden.
- Added Rust policy/privacy/startup checks and a tooling self-check with isolated fixtures.

Next: finish runnable checks/build, install, test login and terminal with Cua, then benchmark.

## Build and self-review follow-up

- Full dependency build reached the app. Fixed a misplaced MCP constructor return caught by
  rustc (`expected (), found &mut TemplatableMCPServerManager`); rebuilding with warmed caches.
- Tool checks passed: build feature exclusions, copy boundaries, idempotent install, real Git
  fast-forward/rebase, dirty-worktree refusal. `bash -n lean/*.sh` and `git diff --check` passed.
- Disabled palette shortcuts for hidden settings and macOS global Oz install actions.
- Self-review found that effective false privacy accessors feed the legacy account-setting
  update method. Lean now skips that legacy write to preserve other installs' account choices.
- Docker stats/top polling stopped at coordinator request; wait on the existing build process.

Next: build completion, focused Cargo tests/lint, scoped formatting, installation and Cua checks.

## Implementation candidate complete

- Gated residual Appearance agent/notebook/Drive options and their palette actions.
- Prevented AWS AI-credential settings subscriptions; Gemini/Grok already flag-disabled.
- Kept Kitty/iTerm image flags alongside hyperlinks, ligatures, completions and history.
- New policy test tries `set_enabled(true)` after lean initialization and asserts that the
  effective flag remains false; post-login test retains the saved true AI preference.
- Upstream code changes are small guards/feature policy; no optional rendering/storage patches.
- Current build started before the final self-review edits. A final build of the completed
  candidate is required before installation/benchmarks; do not mistake its first output for final.

Next: focused unit tests and lint, scoped formatter, final release build, Cua acceptance/benchmarks.

## First release build and provisional desktop smoke

- Docker release build passed: `Finished lean profile [optimized] in 8m 29s`.
  Binary: arm64 ELF, stripped, 304 MiB; `ldd` resolved every library on the host.
- Installed this provisional binary through `lean/install-linux.sh`; schema generation passed.
  Recorded hashes of 9 stable config/theme/tab-config files in `/tmp/warp-lean-stable-before.json`.
- Launched via Cua: PID 240916, window 46137347, exactly 1280x800. Shell initialized.
- Runtime visual regression found: AI mode/context/model toolbar and legacy assistant icon
  remained visible even with effective AI disabled. Screenshot (no secrets):
  `/tmp/warp-lean-evidence/provisional-launch.png`.
- Cause: these render functions don't consult effective AI availability. Added lean render
  guards and disabled legacy panel activation. Kept normal universal command input and the
  saved `honor_ps1=false`/universal style; forcing Classic would change normal input behavior.
- Focused unit-test container `warp-lean-check` was queued behind the build and is compiling.
  Source changed after it started; final affected checks/build must include these UI guards.
- Tool self-check also passed a real rebase conflict scenario and retained its recoverable state.

Next: finish Rust checks, then build/install the corrected candidate and repeat visual acceptance.

## Login callback regression found before final build

- Existing browser Warp session is available; credentials are not required. Private browser
  evidence stays under `/tmp/warp-lean-evidence/`.
- Provisional build reproduced a real callback failure: browser launched a second Warp process
  (246942), while the initiating process (240916) kept waiting. Upstream only forwards startup
  URLs under `release_bundle`, so the second process cannot consume the first auth-state nonce.
- Extended the existing Linux forwarding cfgs to `lean`, avoiding all other release-bundle
  behavior. This is a directly observed before/final-after regression check, not an auth bypass.
- Removed changelog, billing and referral overflow entries; blocked the shared changelog request
  method so direct actions cannot fetch it either.
- First focused Cargo test attempt failed to compile a new test's App/AppContext mismatch;
  corrected the assertion context. The next compile is running; no Docker stats/top polling.

Next: finish/repeat affected checks, scoped formatting and final build; repeat callback and sync.

- Account plan/referral controls are also hidden; login, logout and settings sync stay visible.
  Both provisional Warp instances were closed through Cua window close controls. The first
  optimized test compile remains running with no new compiler errors; Clippy is queued on
  Cargo's shared target lock. Final acceptance and benchmarks are still pending.

- Follow-up source trace found eager hidden Settings page constructors: billing pages fetch AI
  usage at construction even when navigation hides them. Lean now returns a six-page Settings
  view before creating any AI/billing/Drive/cloud page. The optional environment overlay handle
  stays absent. This avoids those background requests and their view allocations entirely.

- Disabled external AI/Drive/MCP/sharing/Oz deep links at their shared URI handler, retaining
  authentication, saved terminal tab/launch configs, terminal session focus, normal new tab/window,
  file-editor and Docker terminal actions, and the retained Settings pages. They cannot bypass
  hidden UI by opening a browser URL. Final Cua checks will exercise a rejected agent URL.

## Focused Rust checks

- `cargo test --locked -p warp --lib --profile lean --no-default-features --features "$features"
  lean_ -- --nocapture` passed: 11 selected tests, including the four new lean tests. Optimized
  full app test linking took 14m25s; execution took 0.03s.
- Reused its compiled Cargo test harness inside Docker for existing coverage: privacy schema,
  persisted auth loading, process-stat construction, 8 MCP lifecycle tests, cloud-to-local sync,
  and local-to-cloud initial preference creation: 13 passed, zero failed.
- Strict Clippy stopped in unchanged upstream release code: 8 unused-import/variable/dead-code
  warnings in warpui_core became errors under `-D warnings`. These were already visible in the
  first release build. Running `--no-deps` Clippy without warning promotion for final type/lint
  coverage; no unrelated warning cleanup.

- Final `cargo clippy --locked --no-deps -p warp --bin warp-oss --profile lean
  --no-default-features --features "$features"` passed in 2m31s. Its 10 warnings are the 8
  documented warpui_core diagnostics plus 2 unchanged warp_terminal warnings; no app warning.
- All eight strict-Clippy locations are in six wholly unchanged paths. `git diff --quiet
  master..lean -- <those six paths>` returned 0; none is a changed line.

- Scoped rustfmt completed. Reviewed its uncommitted diff against master-to-HEAD changed-line
  maps. Reverted upstream-line formatting in Appearance, Features, Account, universal input
  imports and an environment overlay expression. The retained formatting diff changes only
  lean-added/edited lines in 6 files; assert-based line-map check passed. No upstream reformatting.

Next: final Docker release build, real callback/login/sync, Cua acceptance, logs/sockets and two
complete benchmark rounds with no builds running.

## Final sync boundary follow-up

- Source inspection of the retained sync path found an unconditional
  `fetch_environment_last_task_run_timestamps()` call after every cloud-object refresh.
- Intentionally stopped the owned in-progress build (Docker exit 137 after stop timeout);
  it is not the final candidate. Added an early return to that cloud-agent fetch.
- The shared settings-sync response includes AI/MCP generic objects. Lean now skips parsing
  them, only applies Preference generic objects, and ignores MCP gallery events. Folder and
  account metadata needed to create/sync preferences remain intact. Server-side deletion or
  mutation of excluded objects is not introduced.
- Re-running Cargo tests for the completed candidate, including both existing cloud-sync
  directions, auth, privacy, process stats and MCP lifecycle. Then final lint, scoped formatting,
  release build and desktop acceptance. No further optional optimizations are planned.

- Browser cleanup was not performed. Its old Cua session had ended, so the driver rejected
  the read before any interaction. Per coordinator instruction, leave the browser open unless
  a fresh inspection proves it contains only the login/callback tab; never kill its process.

## Completed candidate tests passed

- Final source at bdf6f3b5d passed seven actual `cargo test` invocations inside Docker:
  `lean_` (11), privacy schema (1), persisted auth (1), process stats (1), MCP lifecycle (8),
  cloud-to-local sync (1), local-to-cloud initial preference creation (1): 24 total, zero failures.
  First compile 12m53s; subsequent cached invocations each under one second plus execution.
- This replaces the earlier pre-UI/pre-sync-boundary harness limitation. Existing mock teardown
  logs include closed-channel messages, but every selected test exited successfully.
- Final non-strict Clippy is running against this completed source; strict-warning locations
  remain in the six wholly unchanged upstream files already documented.

- Completed-candidate non-strict Clippy passed in 2m18s. Warnings are still the same 8 unchanged
  warpui_core release diagnostics and 2 unchanged warp_terminal diagnostics; no app diagnostic.
- Formatting only the two new sync files, then checking every retained formatting line against
  master-to-HEAD added/changed lines before the final release build.

- Final sync formatting review passed: rustfmt touched only a lean-added match expression;
  manually wrapped the other guard inside upstream's rustfmt-skipped GraphQL implementation.
  No pre-existing upstream line was reformatted. Source is ready for final release compilation.

## Completed Docker release build

- Final `lean/build.sh` completed successfully in Docker in 8m02s, source commit 375a75198.
  Binary `/work/target/lean/warp-oss`, profile lean, explicit terminal features, no defaults.
- All 24 selected tests and completed-source Clippy passed before lean-only formatting.
- Beginning installation and fresh Cua acceptance, then two complete benchmark rounds with
  no build or stable Warp process running. No source changes are planned unless runtime fails.

## Post-login desktop regression

- Final release login succeeded through `warposs://` forwarding, with settings sync visibly On.
- Cua caught unconditional `Enable AI`, CLI-toolbar and latest-agent-message palette actions,
  plus a zero-credit purchase banner. Before-fix screenshots remain private under `/tmp`.
- Restrict settings action registration to the six retained pages, disable the standalone
  latest-agent command and agent-completion toggle, hide the credit banner and agent-only
  notification widget. Command/OSC notifications remain available.
- Closed the two login-test-created Warp browser tabs through Cua. The user's separate
  browser window was left untouched. Closed our lean test launch before replacement.
- Checking these UI guards, then scoped formatting and another release build; no benchmarks
  were taken against the failing UI candidate.
- UI-candidate Clippy passed (2m15s), with only the previously documented unchanged upstream
  warnings and no app diagnostic. Gated the now-unused agent notification mouse state too.
- Scoped rustfmt proposed two upstream-touching hunks; both were reverted by line-map review.
  No formatting-only change remains. `git diff --check` and tooling self-check pass.

## Code freeze

- Self-review traced two remaining keyboard entry points to Drive search and saving a selected
  command as a workflow. Added the same lean binding guard at each registration site.
- Stopped our unfinished build intentionally (exit 137); this was not an observed OOM.
- Orchestrator requested code freeze after these two guards. Further shortcut-only or edge-menu
  findings will be reported, not patched. Only new background/network work or AI UI visible by
  default in normal use may reopen source changes.
- No delegated review was run, as requested. Self-review covered the changed auth, sync,
  service/launch guards, settings and terminal UI, installer and sync-script boundaries.
- Frozen-candidate Clippy passed with no app diagnostics. Formatting the two shortcut files
  produced no changes. Source is frozen for the final Docker release build and Cua checks.

## Frozen release and installed desktop acceptance

- Frozen source `dd68ee356` Docker build passed in 8m00s. Stripped Linux aarch64 ELF,
  314100072 bytes, SHA256 af5c61295615e8671fb5af98bcb5f91e7289b62720bca2fbf7ef1adb2128b312.
- Reinstalled into ~/.local; installed hash matches. `ldd` resolves every dependency and
  `xdg-mime query default x-scheme-handler/warposs` returns dev.warp.WarpOss.desktop.
- Fresh Cua launch: persisted login works; Settings sync visibly On. Six settings pages,
  no AI mode toolbar or credit banner; palette AI/agent/MCP/Oz/Drive queries show only unrelated
  fuzzy matches. Shell `seq 1 20` renders correctly. Reviewed private-free captures copied to evidence/.
- Ordinary OSC9 and OSC777 toasts render. The CLI-agent sentinel is dropped without a toolbar,
  notification or raw JSON; shell script reaches completion. No support code was added.
- AI URL handoff tool reports a timeout after the short helper exits; existing instance/window
  stays unchanged with no agent tab. The helper rotated the log, so checked the GUI's real open
  descriptor: warp-oss.log.old.temp (361 lines), not the empty newer path.
- Active GUI log has zero Factory/MCP, ONNX/classifier, remote-server, autoupdate/changelog or
  local-HTTP activity mentions after excluding the static channel-config line. `ss -ltnp`
  shows zero lean TCP listeners. Exactly GUI + normal terminal worker; no Oz remote daemon.
- Beginning two fresh benchmark rounds now; no compiler/build or stable Warp remains running.

- First attached 60-sample benchmark was SIGTERM-terminated before any row (tool timeout);
  discarded it. Restarted a fresh app and the full protocol. Sampler remains byte-for-byte
  unchanged; runs now use nohup/setsid with per-scenario exit files outside the repo.

- Round 1 idle completed: 0.35% CPU, 0.36 CPU-s, 308 MB RSS, 242 MB PSS,
  121 threads, two processes (60 unchanged samples). Detached exit status 0.
- Next burst sampler launch was rejected by the approval layer ("rejected by user");
  no burst command was typed. Kept the existing app idle pending clarification.
- Orchestrator clarified the burst rejection prevented overlap and explicitly authorized
  continuing once idle exited. Burst then finished with exit 0: 4.21 CPU-s, 309 MB RSS,
  121 threads. Four Cua tab shortcuts 1.5 seconds apart and ten-second settle completed;
  round-one five-tab sampling is now running alone.
- Round 1 five-tab idle exited 0: 0.29% CPU, 0.20 CPU-s, 329 MB RSS, 263 MB PSS,
  125 threads, two processes. Captured seq reaching 300000 with five tabs, reviewed for
  private data and copied to evidence/seq-five-tabs.png. Quit through Cua.
- Round 2 fresh Cua launch, twenty-second settle, exact foreground input click completed;
  the 60-sample idle scenario is running alone.
- Round 2 idle exited 0: 0.35% CPU, 0.36 CPU-s, 309 MB RSS, 243 MB PSS,
  121 threads. Its subsequent burst exited 0: 4.23 CPU-s, 310 MB RSS, 244 MB PSS,
  121 threads. Four tabs opened 1.5 seconds apart, ten-second settle done; final five-tab
  sampling is running alone. No builds or stable app during either round.

## Final results and self-review

- Round 2 five-tab idle exited 0: 0.30% CPU, 0.21 CPU-s, 332 MB RSS, 265 MB PSS,
  125 threads. All six completion files are 0; all rows count two binary processes.
- Two-run means: idle one-tab CPU -44%, RSS -47%; idle five-tab CPU -81%, RSS -46%;
  13 fewer threads in both. Burst 4.22 CPU-s vs stable 4.02 (+5%): no speedup claimed.
- Post-benchmark ss/log check: zero lean TCP listeners, zero Oz daemons, no prohibited
  activity across 755 retained final-build log lines. Nine stable file hashes unchanged.
- Cua generic Open Left Panel action exposes no Drive UI; no further observed UI leftovers.
  No source changed after dd68ee356. Detailed checks, limits, commits and paths are in
  VERIFICATION.md. No workflows touched; master/upstream remain 831f4fa; no push.
- Final review found no new source/security/architecture blocker. Preserved auth/sync and
  local-file boundaries; strict Clippy warnings are wholly unchanged upstream paths.
  Known concerns: burst CPU +5%, strict upstream warnings, no full workspace/macOS or
  exhaustive edge-menu/cross-device sync verification. Independent review follows freeze.

## Independent-review fixes

- Item 1 first: host has neither ImageMagick nor Python PIL. Preserved all ten evidence
  PNGs only in /tmp/warp-lean-evidence/committed-before-review and removed them from the
  unpushed HEAD by amendment. No avatar-bearing evidence PNG remains in lean history.

- Review items 2–6/9: exhaustive feature-flag override test and positive login/sync checks;
  table-driven URI policy/dispatch tests; renamed the privacy getter test to match its scope.
  MCP cloud subscription is compile-time excluded under lean; websocket content updates
  reject non-Preference objects before touching models or persistence. Restored the Project
  Explorer setting, declared rg/python3 tooling prerequisites, documented default-denied
  terminal/editor flags and ForceCloudToMatchLocal, and enabled repo-local rerere in sync.sh.
- The actual URI dispatcher gate is temporarily bypassed for the requested Docker red run;
  /tmp/warp-lean-uri-green.rs holds the candidate to restore. No build may be installed while red.

- URI red run: actual Cargo test with the dispatch gate bypassed exited 101 (13m53s);
  positive routes passed, rejected-route dispatch failed on an unregistered telemetry model.
  Restored the gate, added the hyphenated handoff alias, and adapted three existing AI
  assertions to lean-disabled effective values while keeping default-feature expectations.
- Default-feature cargo check warm-up passed in 4m04s; not counted as final validation.
  The old lean GUI was closed through Cua; the user's Chromium window was left alone.

- Per follow-up review: restored both upstream AI comments unchanged; replaced URI AppContext
  dispatch testing with a pure rejection table. A one-off Rust harness compiled the actual
  production enum/predicate and both real table tests: true-returning mutant red exited 101
  at the Team/invite rejection assertion; actual predicate green exited 0, 2/2 tests passed.
  Harness moved to /tmp/warp-lean-evidence/check-uri-policy.py and will not be committed.
  Cancelled the superseded green build (own Docker container) before restarting final tests.

- Final candidate default-feature cargo check passed in 28.72s with no environment/library
  failure. Default-feature AI/settings, privacy, URI and persisted-auth tests are compiling
  in target/nonlean; the complete lean touched-module/root/MCP/sync suite is compiling
  against the restored pure-predicate source. Bash syntax and git diff --check pass.

- Final lean Cargo suite passed: 137 tests in 22 nonempty invocations, zero failures/ignored. Includes real URI policy tables, exhaustive feature overrides, all AI/privacy/root startup tests, MCP and two preference-sync regressions.

- Approval review rejected only the combined in-place formatter while default-feature
  compilation reads the tree. Per explicit follow-up, restarted Clippy/build with rustfmt
  --check, recording its exit and continuing the release build. No source is mutated.

- Final strict Clippy exited 101 on exactly the eight known warpui_core diagnostics.
  Compared each diagnostic's whole file with master: all unchanged. Normal targeted Clippy
  passed; rustfmt --check exited 0 for all thirteen review-touched Rust files, with no
  flagged upstream or lean diff and no in-place formatter run. Final release build started.

- Final Docker release build passed in 8m01s. Installed successfully; stripped aarch64 ELF,
  314296680 bytes, SHA256 2d8724c25b995861c6734725ec5a640e614ea373f74209b740d578403d24a094;
  installed hash matches, ldd resolves, warposs handler remains dev.warp.WarpOss.desktop.
- Cua final-build checks passed: shell seq 1..20 renders, five terminal tabs, Settings opens,
  login persists, Settings sync On. Sidebar has only Account/Appearance/Features/Keyboard
  shortcuts/Privacy/About. Palette AI contains no AI commands, but has normal fuzzy matches
  such as Activate Next Tab (not literally an empty results list). Screenshots stay private.
- Closed the palette and began logged-in idle observation at 2026-10-21 00:37:42 UTC.
  Nine stable config/theme hashes still match; no stable directories were written.

- Five-minute logged-in network inspection completed visually through the unchanged split
  viewer (vertical scrolling only; no clipboard access, no full-token screenshot). Distinct
  operations: GetUser, GetUserSettings, GetUpdatedCloudObjects,
  GetWorkspacesMetadataForUser (suffix confirmed in source), GetReferralInfo, UserGithubInfo.
  Socket snapshot: two TLS connections to 34.117.41.85:443; PTR
  85.41.117.34.bc.googleusercontent.com. HTTP methods remain clipped at the approved width.
- Found a gate gap: hidden cloud-environment form construction fetches UserGithubInfo,
  including on window refocus. Added an assertion regression before the shared fetch gate.
  GetReferralInfo serves terminal referral themes; workspace metadata supports account/sync.
- Final runtime audit before this last gate: zero TCP listeners, no MCP/classifier/remote/
  update/changelog activity in 630 runtime lines, no Oz/stable processes, 9/9 stable hashes
  unchanged. Sanitized result: lean/evidence/review-runtime-checks.json.

- Caller review refined the network finding: the lean SettingsView already omits its
  environment page. Terminal ambient first-time setup and the handoff modal also create
  UpdateEnvironmentForm eagerly. The shared fetch method is the right guard for all callers.

- GitHub fetch regression RED: cargo test lean_environment_form_does_not_fetch_github_repos
  exited 101 after 14m24s, failing the intended `!is_loading` assertion at test line 1324.
  Added only a three-line lean early return in fetch_github_repos_internal. Final green
  suite now includes the entire existing environment-form module plus all previous modules,
  root/startup tests and the two preference-sync regressions. Non-lean final rerun required.

- Final guard GREEN Cargo suite exited 0 at 01:24 UTC, including the actual GitHub assertion
  regression and full existing environment-form module. All previous lean/root/sync modules
  were rerun on the guard source. Final strict/normal Clippy, non-mutating rustfmt --check
  (15 review-touched Rust files) and release build started; default tests still compiling.
- Green total: 181 passed, 0 failed/ignored across 23 nonempty Cargo invocations; first
  compile 14m14s. The environment form module contributes 44 tests, including the new guard.

- Final guard Clippy: strict exited 101 with exactly the same eight upstream warpui_core
  diagnostics; each affected whole file equals master. Normal targeted Clippy passed.
  Non-mutating rustfmt --check passed all 15 review-touched Rust files (empty output);
  no upstream formatting hunk was produced or retained. Final release build is running.

Resume point: final release/lint container `warp-lean-review-last-build` (exec session 77884)
continues; default test container `warp-lean-review-default-final` (exec session 84030)
continues. Wait for exits, not Docker stats/top or compiler probes. Re-run default check/tests
with DEFAULT features in target/nonlean after that first container finishes, because it
started before the final GitHub guard. Next: close the owned old GUI with Cua, install the
new release, launch/smoke and five-minute visual network recheck, finalize docs, checkpoint
then tag lean-pre-squash and rewrite to two area commits; verify identical tagged tree.

- Final GitHub-guard release build passed in 8m51s. Installed successfully; 313969000 bytes,
  SHA256 ec5f52121ae8096cdf52c64fc495ef55607dd046227f03555e6a4886b6428fac; installed
  hash matches, ldd resolves, warposs handler correct. Closed only our prior lean GUI via
  Cua, launched new PID 747892/window 65011715. Cua confirms seq 1..20, five terminal tabs,
  Settings opens, login persists, sync On, six expected sidebar sections and no AI palette
  entries (normal fuzzy matches remain). All screenshots private: review-netfix-*.png.
  Five-minute idle/network recheck started after closing the palette.

- Final network recheck complete per the narrowed review instruction: the viewer has no
  working find, so one unchanged-width Cua vertical batch reached the first Settings-open
  entries (01:39:24 to next request 01:40:42 UTC). Only workspace metadata appears there;
  no UserGithubInfo. Screenshots review-netfix-walk-58..60.png are private in /tmp.
  The actual Cargo assertion red/green is the primary proof; no second five-minute audit.
- Final runtime audit at 01:46 UTC: installed hash ec5f5212 matches; two lean executable
  PIDs, zero TCP listeners, no stable/Oz processes, zero disabled-service activity mentions
  in 571 runtime lines. TLS peer remains 34.117.41.85:443, PTR
  85.41.117.34.bc.googleusercontent.com (infrastructure only). All 9 stable hashes match.
- Remaining: let default-feature test compilation finish without probes, rerun its
  check/tests incrementally on the final guard tree, finish report docs and rewrite history.

- Default-feature original container exited 0: 108 tests across four filters. Source edits
  during that early run caused 57m56s, 50m07s and 48m54s recompiles; these are not the final
  reported verification. No source was touched during its last compile or confirmation.
- Final-tree confirmation ran detached via nohup setsid, using the same target/nonlean
  cache, six jobs, default features and an exit file. Exit 0: cargo check 36.04s; AI 34,
  privacy 1, URI 72, persisted auth 1 and environment-form 43, total 151 passed, 0 failed/
  ignored. Cached build phases 0.82–0.86s. All 43 source/manifest hashes remain frozen.
  Exact commands and output/exit paths are recorded in VERIFICATION.md.
- All code, build, test, desktop, network and benchmark milestones are complete. Final
  documentation records the eight unchanged-line strict-Clippy diagnostics and remaining
  non-AI find, network-host visibility, cross-device, macOS and workspace-test limits.
- Last administrative step: checkpoint the reviewed tree, tag it locally lean-pre-squash,
  then rewrite master..lean into source/tests and lean tooling/docs/evidence commits. The
  step must assert identical tag/current trees, one commit per upstream file, clean status,
  no PNG/log/harness/workflow changes and master still equal to upstream at 831f4fa9fa.
  To preserve that identical-tree check, post-rewrite results/SHAs go in the completion
  message and /tmp/warp-lean-evidence/history-final.json rather than changing this snapshot.


### Review-fix batch (2026-10-21)

- Restored Input type registration/sync; hid Default session mode; file resolution now changes
  only the Warp editor choice and Markdown preference before upstream fallback selection.
- Removed redundant poll constants/test, tab-config copying and the unneeded embedded lean
  macOS plist. Transferred-tab titles use the shared name helper. URI/keymap lean statements
  were formatted without moving upstream code.
- Referral themes remain reachable when unlocked (`themes/theme_chooser.rs`), so the conditional
  review request leaves GetReferralInfo enabled. The current theme is custom Monokai Pro Spectrum.
- Added fresh-returning-device cloud sync coverage with unchanged upstream helpers. Build,
  test, install/desktop and rewritten-commit verification results are recorded privately in
  `/tmp/warp-lean-evidence/review-fixes/REPORT.md`; no screenshots or raw runtime logs are committed.
- Final lean cycle passed: 333 test executions across overlapping filters; fresh-device
  zero-upload test passes with cloud CustomSecretRegexList seeded from upstream defaults.
  No production initialization or upstream test helper changed. Release build: 10m07s.
- Installed the matching release, preserving rollback. Cua PID1477366: logged in, sync On,
  Input type visible/Warp selected, Default session mode absent. Theme/font/zoom unchanged;
  settings SHA/mtime and rollback hash unchanged after scrolling and resize/restore.
- Formatting check retains four deferred upstream-spanning hunks; URI, keymap and new tests
  are clean.
- Final default-feature app/shared checks and all selected module tests passed on the
  frozen source: 153 test executions, including all 17 upstream syncer tests.
- Folded fixes into the four area commits; the saved candidate tree and all 60 frozen source
  hashes matched afterward, with no conflict markers. The authorized CI-only fast-forward
  triggered https://github.com/ragokan/warper/actions/runs/39628464139 at 26e3fb465.
- Commits 1a1ad4f66, b8bbcc2d2 and 1ef77b6b1 each passed lean app/lib/tests, lean
  warpui_core/lib/tests and default-feature app/lib compile checks (nine checks, exit 0).
  The isolated worktree was removed and pruned; exact commands/logs are in
  `/tmp/warp-lean-evidence/review-fixes/commit-check/`. The final doc-only amendment was not pushed.
