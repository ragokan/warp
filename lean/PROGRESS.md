# Lean build progress

## 2026-10-23 — faster macOS startup: icon write, launch work, startup timing in the log

- Findings that set the scope (code reading plus passive checks of the installed 0.3.3 on the user's Mac, which is this branch's HEAD 46e1a9955): the database there is tiny (12 blocks, 20 commands, one window), so session restore and the full startup SQLite read are not the cost; the log could not say what is, because file timestamps had whole seconds (every recent launch reached the first `Bootstrapped` 1-2 s after the first line). Kept as-is after checking: the Metal library is precompiled and loaded from memory, and the terminal server is spawned at the very start of `run_internal`, in parallel with the rest of startup.
- Instrumentation. Ungated: the log file records milliseconds (`buf.timestamp_millis()`). Lean: once the first frame is drawn, one line `Startup timing: NAME +marginal ms (at cumulative ms), …` lists the existing `IntervalTimer` phases (`LOG_FILE_SETUP_COMPLETE` through `FIRST_FRAME_DRAWN`); upstream only sends them as telemetry for signed-in users, so lean never saw them.
- Launch work removed under lean, each with no consumer that lean shows:
  - the download-method report, which on macOS ran `brew list --cask warp` (Homebrew's Ruby) at every launch to fill in a telemetry field;
  - `LocalShellState`'s login shell for `echo $PATH` (sources the user's profile while the first tab's shell starts); its only reader is the Drive secrets dialog, which falls back to the login shell's own `PATH`;
  - the `$HOME` watch (`HomeDirectoryWatcher` registers nothing; the singleton stays because AI-only code looks it up) and global-rules indexing; on macOS a non-recursive watch is an FSEvents stream over the whole home tree;
  - repository indexing on every detected repository (`LocalRepoMetadataModel` walks and recursively watches the tree); its readers (file tree, `@` menu, command-palette file search, AI context) are off under lean, and detection itself (git status) is unchanged. A lean binary run as the remote-server daemon would not index either; lean never starts that mode;
  - AVFoundation, linked only for the microphone permission voice input reads: it is now behind the new `warpui` feature `microphone_access`, enabled by the app's `voice_input` (part of upstream's `gui`). Without it the permission reads as not determined.
- Ungated: MetalKit is no longer linked (nothing uses it), and `BulkFilesystemWatcher` starts its background thread and the debouncer thread (which wakes every quarter of the debounce duration, watched paths or not) with the first path command instead of at construction. Lean created five such watchers at launch (`FileModel` 200 ms, `WarpManagedPathsWatcher`, `DirectoryWatcher`, `HomeDirectoryWatcher`, `LocalRepoMetadataModel`), which by their debounce durations (ticks of 50, 125, 125, 125 and 250 ms) add up to 48 wakeups/s while idle (computed, not measured). Now only watchers with a path run: the managed-paths watcher from launch (8/s), and `DirectoryWatcher` once a session enters a git repository (another 8/s).
- Tests, each red without its change: `background_watcher_starts_with_the_first_registered_path` (red with the start moved back into `new`: "a watcher with no paths should not start its background thread") and `changes_under_a_registered_path_are_reported` (a real inotify watch still delivers a created file after the lazy start) in `crates/watcher`; `home_directory_is_watched_except_in_lean` (red under lean with the gate removed, `left: true, right: false`); `detected_git_repositories_are_indexed_except_in_lean` (`repo_metadata`, red under lean with the gate removed, `left: true, right: false`). Both crates' suites pass with and without lean (`watcher` 4/4, `repo_metadata` 142 passed, 2 ignored). The brew, `$PATH` and global-rules gates are one-line conditions with no unit test; they are checked by watching launch child processes on the Mac (below).
- Linux checks (Docker builder): full lean `warp --lib` suite with the usual three hanging modules skipped: 6,983 passed, 337 failed against 326 at the 6d27b62ad baseline. The run used uid 1001 from `/tmp`, because the host's uid 1000 had 125 of its 128 inotify instances in use (watcher tests then fail with EMFILE); all 13 failures beyond the baseline pass when rerun as uid 1000 from `app/` (home-directory and working-directory assumptions), and two baseline failures (`file_based_manager`) pass. `warp_core` timer and `warp_logging` tests pass; lean Clippy over the five changed crates exits 0 with no warning in a changed file; default-feature `cargo check -p warp --lib --tests` passes.
- Mac baseline before the change (Mac16,11, macOS 27.0.1, 14 cores, other agents running; launch harness with a monotonic clock, new on-screen window IDs of the PID and the first `Bootstrapped` log line, runs with more than 20 ms of event uncertainty rejected; warm medians of six accepted runs, milliseconds from `open`): installed 0.3.3 process 41, window 594, shell ready 827; the same HEAD built locally 39 / 625 / 812; one cold-ish launch 106 / 859 / 1152. Idle over 180 s (installed, after one launch): GUI 0.39% CPU, 1.0 package-idle wakeups/s, 35.8 interrupt wakeups/s, 164.8 MiB footprint, 42 threads (six `Bulk Filesystem Watcher`, six debouncer, three FSEvents loops); terminal server 0%, 8.4 MiB. AVFoundation and MetalKit were loaded in both processes (1,145 and 534 images).
- Mac A/B (same harness, both built from 46e1a9955 with the patches; A = the two logging changes only, B = this entry's changes; six accepted warm runs each, medians, ms from `open`): B process 38, window 605, shell ready 796, first frame 516 ms after the timer origin; A 38 / 623 / 820, first frame 531. A ran under a higher load (1-minute load 11-17 against about 7 for B), so the startup difference is not attributable with confidence; it is at most a few percent. Clearly attributable: no `brew` and no `/bin/zsh -l -c echo $PATH` among the launch's children on B (a 3 ms descendant probe saw both on the HEAD bundle); `otool -L` 21 → 19 direct dependencies, AVFoundation and MetalKit gone; the terminal server loads 478 images instead of 534 (the GUI still loads 1,145, AVFoundation, MetalKit, CoreMedia and AudioToolbox included, through some runtime path not yet identified). Idle over about 180 s, B against the installed HEAD: 0.022% against 0.388% CPU, 0.14 against 1.00 package-idle wakeups/s, 4.8 against 35.8 interrupt wakeups/s, 30-31 against 42 threads (watcher/debouncer/FSEvents threads 6/6/3 → 1/1/1), footprint unchanged (164.8 MiB). HEAD began the idle window frontmost and B stayed in the background, which weakens the CPU comparison but not the thread count.
- Startup phases on B (median marginal ms): settings and user defaults 70.5, SQLite 35, everything up to `INITIALIZE_TELEMETRY_COLLECTION` 273, windows created 56, first frame 69.5. Diagnostic `sample` runs put about half of the main thread's startup samples (172 of 354 on A, 184 of 404 on B) in `NSWorkspace setIcon:forFile:options:`, mostly PNG compression: `AppearanceManager::set_app_icon` writes the selected custom icon into the app bundle at every launch (it is also why a launched bundle fails strict signature verification: it gains `Icon\r` and `com.apple.FinderInfo`).
- App icon (follow-up, `app/src/appearance.rs`, lean-gated): with a custom icon selected, `set_app_icon` wrote it into the bundle (`NSWorkspace setIcon:forFile:options:` plus `noteFileSystemChanged`) at every launch, on the main thread before the first window. Under lean it now records `[bundle path, icon file name]` in the private preference `WrittenBundleIcon` after a successful write and skips the write while the bundle still has its `Icon\r` file and the record matches; an update replaces the bundle and with it the icon file, so the next launch writes it again. With the default icon it skips the reset when there is no `Icon\r` to remove. The in-memory `setApplicationIconImage:` still runs at every launch.
- Mac round C (the user's settings select the `cow` icon; 63 launches alternating B and C, the first six precision-qualified runs of each, 1-minute load 7.9-11.6): B process 41, window 658, shell ready 858, first frame 543; C 37 / 381 / 559, first frame 285. Phase medians B → C: `INITIALIZE_TELEMETRY_COLLECTION` 266 → 71.5, settings 99 → 73.5, windows created 56 → 34.5, first-frame drawing 74.5 → 67.5. The first launch of a new bundle writes the icon and the record (`["…/test-bundle-C/Warper.app","cow"]`), after which `Icon\r` kept the same inode, mtime and ctime through all 63 launches; the running app's icon exported from `NSRunningApplication` matches the cow image. A warm C sample has no `setIcon:forFile:options:` frames. Not checked: the Dock's pixels and the default-icon branch.
- Why the GUI still loads AVFoundation and MetalKit (traced with `DYLD_PRINT_LINKS_WITH`/`DYLD_PRINT_LIBRARIES` on B): AppKit pulls them in itself, AVFoundation through `dlopen(AppIntents)` → CoreSpotlight → Contacts → ClassKit → CloudKit, MetalKit through `dlopen(WritingToolsUI)` → … → AuthenticationServices → LoginUIKit → `libswiftMetalKit`; CoreMedia through CoreGraphics → SkyLight; AudioToolbox is a direct AppKit dependency. The Writing Tools load comes from `finishLaunching` → `_customizeMainMenu` → `_addTextInputMenuItems` → `NSWritingToolsCoordinator isWritingToolsAvailable`; avoiding AppKit's automatic text-input menu items is an open experiment, its saving unmeasured.
- Compiled on the Mac only (not on Linux): the `warpui` and `app` build-script changes, the mac and headless delegates and the app-icon change; the patched builds were built and bundled with the CI settings and launched there. Not run: the Linux desktop with these changes.

## 2026-10-22 — no AI assistant panel tick

- `AIAssistantPanelView::new` (`app/src/ai_assistant/panel.rs`) started `tick`, which every 60 s notifies the panel and its transcript view and re-arms itself, to keep a "next refresh" time current. `Workspace` builds one panel per window whether or not AI is on, so every window repainted once a minute while idle. On macOS that repaint re-wires the roughly 116 MB of Metal driver memory for a few seconds each minute and costs about 0.036 points of idle CPU (the Mac check: with `panel.tick(ctx)` commented out the 60 s spikes are gone and graphics memory stays at 5.1 MB). On Linux it is the 0.19-0.20% left in the 60-120 s windows after the cursor blink stopped (idle cursor entry below): 0.2 CPU s per 60 s window is one llvmpipe repaint. That attribution is inferred from the period and cost; Linux was not re-measured after this change.
- Lean: `new` does not start the tick. Lean never shows the panel (`Workspace::toggle_ai_assistant_panel` returns at once under lean), so nothing reads what the tick keeps fresh. Not done: starting it only while the panel is open. The panel does not learn about being opened except through the workspace, so that needs a new event, and lean cannot open it anyway. Default builds are unchanged.
- Test `refresh_tick_is_not_armed_in_lean` (`app/src/ai_assistant/panel_tests.rs`): `tick` bumps a test-only thread-local counter; the test builds a real `Workspace` (one real per-window panel) and asserts the counter moves exactly when the build is not lean. There is no public way to count a view's pending spawned futures, hence the counter. Red with the lean gate removed, green with it.
- Other re-arming timers that can repaint a lean window, reported and not changed (found by grepping `Timer::after`, `Timer::interval` and sleeps in `app`, `warpui`, `warpui_core`, `editor` and `warp_core`): the context-chip refresh (`app/src/context_chips/current_prompt.rs`, only for chips in the prompt: date 30 min, git 30 s, time 1 s, so the time chip would repaint every second if a user adds it to the prompt), the summarization timer in `ai/blocklist/block/status_bar.rs` (1 s, only while a summarization runs), the shared-session inactivity countdown (1 s, only in a shared session), and the AI orchestration, SSE drain and checkpoint timers (only with agents running). The tip cooldowns in `ai/agent_tips.rs` are one-shot and do not notify. The terminal view's `interval(500).take(2)` is bounded.

## 2026-10-22 — system fonts load on demand

- Every startup enumerated and loaded the whole system font list twice, at the same time: `ImportedConfigModel::search_for_settings_to_import` (`app/src/lib.rs`) and `AppearanceSettingsPageView::new` each called `fonts::Cache::all_system_fonts`, and the cache only filled once the first future finished. On macOS that creates a CTFont for every face (445 mapped font files, about 97 MiB resident) and keeps them all. The Linux log shows "Computing available system fonts" and "Loaded 32 font families" twice at startup.
- Ungated: `Cache::all_system_fonts` keeps the in-flight enumeration (`pending_system_fonts`) and hands concurrent callers the same shared future, so it runs once. Test `concurrent_system_font_requests_share_one_enumeration` counts the test font DB's loads: red with the sharing removed (`left: 2, right: 1`), green with it.
- Lean: nothing enumerates at startup. The settings-import search starts when the import block is first added (`ImportedConfigModel::ensure_search_started`, called from `SettingsImportView::new`), and the Appearance page requests the list when its font dropdown is first opened (`FilterableDropdownEvent::ToggleExpanded`). Selecting the page only refreshes the dropdown's label, which is cheap, so restoring a session whose Settings tab sits on Appearance in the background enumerates nothing (the first version loaded on page selection and did). Lean has no Account page and its settings pane used to open on the missing Account section ("settings section Account not found", so no page hook ran at all); `SettingsSection::default()` is Appearance under lean, so the first open runs the hook.
- Tests, each red before the change and green after: `lean_loads_system_fonts_when_the_font_dropdown_opens_not_when_the_page_is_selected` (real `Workspace` and `SettingsView`; `WorkspaceAction::ShowSettings` opens the settings tab through `open_settings_pane` and `SettingsPane::new`, the constructor session restore uses, which selects lean's default page and runs its selection hook; the page is Appearance and nothing is loaded, then opening the font dropdown loads exactly once however often it is toggled; red when the hook requests the fonts, `left: 1, right: 0`), `lean_opens_settings_on_a_page_it_has` (the real `SettingsView` has a page for `SettingsSection::default()`; red with the old Account default, where the page is missing), `search_starts_and_enumerates_system_fonts_once_when_first_requested` (after two `ensure_search_started` calls the search started once and enumerated once; a test-only start counter is needed because the font cache serves a repeated search from memory, so the enumeration count alone passes without the `started` guard; red without the guard) and `font_list_arriving_during_a_search_keeps_a_match_selected` (below).
- Search while the list loads: with the list loaded lazily, a user can open the dropdown and type before it arrives. `set_items` then selects the first match, but `update_font_dropdown` re-selected the configured font by name; the query hides it, so the menu's selection was cleared and the queued `ItemSelected` cached none, leaving Enter with nothing to pick (arrows or a click recovered). `FilterableDropdown::set_selected_by_name_unless_filtered_out` skips the re-selection when the query hides that font, and both font dropdowns use it. The test delivers a non-empty list while a query is active (the test platform's `set_family_name` stands in for a loaded configured font, since it resolves no family names otherwise) and asserts a match is selected; red with the plain `set_selected_by_name`.
- Unchanged: the configured font and its fallbacks go through `get_or_load_system_font` and the platform's own fallback (CoreText cascade on macOS, fontconfig on Linux), neither of which depended on the preloaded list. On macOS the "Import External Settings" entry stays listed until it is used, because it is shown while the search has not finished (the block closes itself when nothing is found).
- Not done: returning names without keeping a CTFont per face when the list is requested. The dropdown renders each entry in its own font on macOS, so it needs the families loaded; the cost now only arises when the user opens the font settings.
- Checked on Linux GUI (isolated XDG dirs) with the first version of this change, which loaded on page selection: no font lines at startup (the 6d27b62ad build logs two enumerations), opening Settings (Ctrl+,) logged one enumeration, the Terminal font dropdown listed Hack, DejaVu Sans Mono, Liberation Mono and Noto Mono, and a terminal line with an emoji, CJK characters and symbols still rendered through fallback fonts. Linux does not memory-map fonts (no font mappings in `/proc/PID/maps`) and its list is small, so startup RSS did not move beyond noise (main process 268,356 kB before, 266,236 kB after, a third run 268,952 kB). The GUI was not re-run after the trigger moved to the dropdown, because the user's own Warper window is active on that desktop; the tests above exercise the new trigger through the real `Workspace` and `SettingsView`.
- Full lean `warp --lib` suite on clean snapshots (16 tests that hang under lean, in `ai::mcp::file_mcp_watcher`, `pane_group::pane::local_harness_launch` and `terminal::cli_agent_sessions::plugin_manager`, skipped): 6,987 passed and 326 failed at 6d27b62ad, 6,991 passed and 326 failed at this commit with an identical failing set (the four extra passes are the new tests). An earlier run on a worktree mount had four failures beyond the baseline, none a regression: `ai::agent_sdk::driver::git_credentials::tests::global_git_identity_reads_the_actual_global_config` passes alone (3/3) and in its module (5/5) on both trees, and fails 3/3 and 5/5 on both when the repository's `.git` is a worktree gitfile that dangles inside the container; `harness_usage_tests::publication_reuses_workload_auth_but_not_an_unrelated_ambient_task` passes alone (3/3, both trees) and fails in 4 of 5 parallel runs of `server::server_api` at 6d27b62ad and 5 of 5 at this commit, with a varying set of neighbours; `prewarmed_sandbox::tests::configuration_errors_return_after_one_request` and `configuration_uses_default_graphql_timeout_and_authentication` share one mock HTTP server whose `PrewarmedSandboxConfig` mock several tests register (that run saw an `Ok` where a 403/503 was expected and a 503 where success was expected); they pass alone (3/3 each, both trees) and did not fail again in 5 + 5 module reruns or three clean full-suite runs, so they are flaky but not reproduced.
- Unverified here: macOS (startup CPU, mapped font files, resident memory, fallback for emoji/CJK/symbols, the font dropdown). Expected: no font files mapped at launch, the first Settings > Appearance visit pays the one-time load.

## 2026-10-22 — macOS GPU memory (findings only, no code change)

- The 121 MB graphics footprint at launch is **not** Warper's allocations. A macOS trace (Metal System Trace, `footprint`, `vmmap`) shows 29 × 4 MiB driver "wire memory" events on the first render pass; a 60-line standalone Metal probe reproduces it, it is a fixed per-process cost (not proportional to the window), and the driver frees it about 1.5 s after the last GPU work. Warper kept it alive only because the 500 ms blink frames never stopped, so the cursor-blink commit above is the fix (a window that is unfocused or minimized already idled at 0.09% CPU and 5 MB).
- Explicit allocations, from the code (`crates/warpui/src/platform/mac/rendering/metal/renderer.rs`, `crates/warpui/src/rendering/glyph_cache.rs`, `crates/warpui/src/platform/mac/objc/host_view.m`):
  - glyph atlas: one `RGBA8Unorm` 1024×1024 texture (4 MiB) created on the first glyph (`glyph_cache.rs:13`, `:132`, `renderer.rs:1142-1156`); a second only when it fills. No 4096² atlas exists, so nothing to shrink.
  - the shared quad vertex/index buffers (`renderer.rs:225-240`) are a few bytes.
  - image and icon textures are `RGBA8Unorm` per asset (`renderer.rs:478`), dropped after 10 unused frames.
  - app-visible Metal memory is about 52.75 MiB, mostly the three `CAMetalLayer` drawables (15.9 MiB each, BGRA8 at the window's pixel size): `makeBackingLayer` (`host_view.m:283-292`) never sets `maximumDrawableCount`, so it is the default 3. Two drawables would save a third, but `presentsWithTransaction` plus `allowsNextDrawableTimeout = NO` makes a lower count a risk (a second render before the first transaction commits would block the main thread), so it is left alone.
  - per-frame `StorageModeManaged` buffers from `new_metal_buffer` (`renderer.rs:844-861`; call sites `:437` per image/icon, `:662` per layer of rects, `:795` per layer and atlas of glyphs): 20-21 per frame. By the struct layout in `shaders/shader_types.h` that is 224 B per rect and 80 B per glyph (about 1 MB for a full 200×60 screen of text; computed, not measured), each rounded up to a 16 KiB page and freed when its command buffer completes. Reuse would need a ring keyed to command-buffer completion; the per-image/icon buffer is only 224 B and could use `setVertexBytes` (as the uniforms already do), but both are minor once frames stop and neither was built blind.
- Linux (wgpu) has the same 1024² atlas (`texture_with_bind_group.rs`), no MSAA (`sample_count: 1`) and no preallocated render targets, so there is no equivalent to measure.

## 2026-10-22 — idle cursor stops blinking

- `EditorView` (`app/src/editor/view/mod.rs`; the prompt editor and every other editor view) blinks from a 500 ms spawned timer whose callback toggles the cursor and calls `notify()`, so each tick is a full redraw (Metal on macOS, wgpu/llvmpipe on Linux). Under lean a tick that finds no editor activity for 30 s (`last_cursor_activity`, refreshed by gaining focus, by `reset_cursor_blink_timer`, which window re-activation and every editor model event already call, and by every editor action in `handle_action`) leaves the cursor visible and does not reschedule: no timer is pending and nothing redraws. The next editor event or action restarts the blink, so input that changes nothing (Left at the start of the buffer, forward Delete at the end) restarts it as well. `cursor_blink = "disabled"` and non-lean builds are unchanged.
- An inactive window already stopped upstream: `blink_cursors` only reschedules while the editor is focused in the active window, and an inactive window draws no cursor. Nothing was added for that case; the test pins it.
- `crates/editor`'s own blink (`update_blink_state`, `repaint_after`) belongs to the code editor and notebooks, which lean hides; it is untouched. The other periodic repaint sources in the code are `LiveElement` (a running command's elapsed time) and animated images, neither present at an idle prompt. The Linux run below saw nothing else in a 30 s window, but the 0.20% it measures in the 60 s windows afterwards is one more periodic repaint: the AI assistant panel's 60 s refresh tick, removed under lean in the AI panel entry above.
- Tested: `test_cursor_blink_stops_when_idle_or_unfocused` (active editor keeps rescheduling; after the idle timeout the cursor is visible and `blink_epoch`, which advances whenever a timer is spawned, stays put; input that changes nothing and typing both restart it; an inactive window schedules nothing). Red with the idle check stubbed out (`left: 3, right: 2`), and the no-op-input stage red without the `handle_action` hook (`left: 2, right: 2`); green with both; the existing `test_cursor_blink` still passes.
- Linux (isolated XDG dirs checked in `/proc/PID/environ`, default settings so blink is on; the real config has `cursor_blink = "disabled"`, which is why the earlier idle benchmark read 0.35%; llvmpipe software rendering makes each blink frame cost about 135 ms of CPU). Desktop idle throughout, Warper the only window and the active one. One keystroke, then two consecutive sampler windows of about 100 s each (`bench.sh 60`: 60 one-second samples plus sampling overhead):

  | build | 0-100 s after typing | 100-200 s | unfocused |
  | --- | --- | --- | --- |
  | 6d27b62ad | 26.89%, 26.88% (second run) | 26.88% | 0.37% |
  | this commit | 7.04% (30 s of blink, then solid) | 0.20% | not re-measured |

  After the blink stops, a 30 s window used 0 CPU ticks in every thread (too short to see a 60 s timer; see the 0.20% above), typing one more key made the next 20 s run at 26.6% again, and the following 20 s idle window used 0 ticks.
- Unverified here: macOS. The Mac check is the same sequence on the real display: type one character, then sample CPU in 60 s windows (expect about half the blink cost in the first, the `cursor_blink = disabled` level of about 0.12% in the second), a window that is visible but not frontmost (should match the unfocused 0.09%), and typing again to confirm it resumes.

## 2026-10-22 — builder: parallel rustc frontend for workspace crates

- `lean/rustc-wrapper` (set as `RUSTC_WRAPPER` by `lean/Dockerfile.linux` only; CI does not use that image) adds `RUSTC_BOOTSTRAP=1 -Zthreads=${LEAN_RUSTC_THREADS:-8}` to workspace-crate compiles, recognised by `CARGO_MANIFEST_DIR` under `/work` plus `CARGO_CRATE_NAME` (set for compiles, not for build-script runs). A logging copy over a cold `cargo check -p asset_macro` showed 4 of 125 invocations touched (the workspace crates) and every registry crate, `rustc -vV` probe and proc-macro2/anyhow build-script probe passed through without `RUSTC_BOOTSTRAP`. Cargo fingerprints are unchanged: after a no-wrapper build, wrapper on and wrapper off were both `Fresh` (6.7 s and 0.8 s).
- Measured on the warm lean target, `touch app/src/lib.rs` first, containers capped at `--memory=32g`, other load on the host (load average 4–19, see `/tmp/warp-lean-evidence/offline/cores/results.jsonl`): `cargo check -p warp --lib` 105–116 s at 1.0 core and 6.3 GB → 57–59 s at 2.4 cores and 6.75 GB with 8 threads (69 s, 6.6 GB with 4); `lean/build.sh` 487 s at 6.3 cores → 363 s at 8.6 cores, the `warp` lib rustc 267 s → 162 s, max RSS 15.4 → 16.5 GB (`warp_oss` link 16.4 → 16.2 GB). Earlier uncapped pairs under heavier load: build 717 s → 340 s and 521 s → 388 s, so the build gain is 25–45 % depending on load. No ICE or deadlock in the wrapper-compiled workspace crates.
- Jobserver: with `CARGO_BUILD_JOBS=1` the `warp` check stayed at 1.0 core and 117 s (the frontend threads draw from the same jobserver); with 16 it took 58 s. Keep jobs at 8 or more.
- Docker runs of the builder now carry `--memory=32g --memory-swap=32g` (README); the frontend threads add about 0.45 GB (check) to 1.1 GB (build) at the peak.

## 2026-10-22 — updater: old-binary cleanup starts before the restart prompt

- Re-review: with `WARPER_PID` set, closing the update tab at "Restart now?" sends SIGHUP and kills the script before either helper started, leaving `.warp-oss.old` (~262 MB) until the next update. The cleanup-only waiter (no command, waits indefinitely, `rm -f` the old binary) now starts before the prompt; an accepted restart still starts the relauncher, which gives up at its limit and removes the file itself before starting the new binary (a second `rm -f` is harmless).
- Tests: 2 of 35 failed against the previous commit (new SIGHUP-at-the-prompt test; the own-session test now expects both helpers), 35 of 35 pass (also under `bash --posix`), whole `lean/ci` suite 39 of 39.

## 2026-10-22 — updater: keep the running Linux binary resolvable, keep the macOS recovery copy

- Review of the previous commit: on Linux, deleting the old binary made `/proc/self/exe` read `<stage>/old/warp-oss (deleted)`, and `current_exe()` callers (updater script lookup, `bundled_resources_dir`, ripgrep self re-exec, crash recovery) failed in the old Warper until it restarted. Now the old binary is renamed to `$app/.warp-oss.old` inside the install, so its parent is still the new install; a detached helper (own session, `kill -0` only) deletes it when the old PID exits. With a restart it does so before starting the new binary; without one it waits indefinitely; without `WARPER_PID` the file stays and the next update replaces it. The rest of the stage is still deleted at once. macOS is unaffected (`current_exe` is the launch path, already the new bundle) and still deletes immediately.
- macOS: if moving the new bundle in fails and restoring the old one fails too, the stage is kept and named instead of being deleted by the exit trap.
- Tests: 6 of the 34 in `warper_update_tests.py` failed against the previous commit (exe path of a running process, accepted and declined restart, no-PID run and next update, warn-only failure to keep the binary, macOS double failure), 34 of 34 pass now (also under `bash --posix`); whole `lean/ci` suite 38 of 38.

## 2026-10-22 — updater keeps no previous copy

- `warper-update` no longer leaves `warp-oss.previous` / `Warper.previous.app`: the old files live only in the hidden `.warper-update.*` stage while the swap can still be rolled back, and the stage is deleted as soon as the new install is in place (before the restart prompt, so closing the terminal there cannot leak the old binary; the EXIT trap stays as the fallback). A leftover from an older updater is removed (exact paths only; a failed removal only warns). The relauncher still starts the new path.
- Tests: 8 of 30 in `warper_update_tests.py` failed against the old script (flipped install expectations, stale-copy removal and warn-only failure on both platforms, stage gone at the restart prompt while a real process runs from the deleted binary and the new one relaunches), 30 of 30 pass now, also under `bash --posix`. The rollback and keep_stage tests are unchanged; a macOS "new bundle could not be moved in" rollback test was added. Known edge (not exercised): an old Warper that declined the restart probably cannot run Check for Updates again until restarted, because its executable path is gone.

## 2026-10-22 — update restart without the Quit dialog

- The restart marker now quits with `TerminationMode::ForceTerminate` (the mode behind the dialog's own "Yes, quit"; same shutdown path) unless a code editor or review has unsaved changes, then it stays `Cancellable`. Before, the update tab's own command could still be running when the marker was polled and raised "Quit Warp? You have 1 process running" (seen on the real 0.3.0 → 0.3.1 update); a dialog left open for 60 s made the relauncher give up.
- Unit test for the mode choice (red with a stub, green after). End to end with a local fake 0.3.2 release in isolated dirs, the script kept running after its restart request to force the race: 0.3.1 build shows the dialog; this build quits 2 s after the answer with the usual shutdown log, the relauncher starts the new binary 1 s later and the session is restored.

## 2026-10-22 — built-in Yoda avatar

- The avatar button shows the bundled `jpg/warper_avatar.jpg` (554×554, `bundled_asset!`, always embedded) unless `avatar.png`/`.jpg`/`.jpeg` exists in the config directory; the gear is only the loading/decode-failure placeholder. `AvatarContent::LocalFile` became the general `AvatarContent::Asset`; non-lean rendering is unchanged. Two unit tests (red with the built-in arm stubbed out, green after); lean release binary 262,390,328 B vs 262,455,832 B at 02f0f210c (−65,504 B despite the 35,213 B image). Checked in the GUI with isolated XDG dirs: built-in Yoda without a file, a replacement `avatar.png` wins.

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
