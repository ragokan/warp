use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use warp_core::paths::state_dir;
use warpui::ViewContext;
use warpui::r#async::Timer;
use warpui::platform::TerminationMode;

use crate::launch_configs::launch_config::{CommandTemplate, PaneMode, PaneTemplateType};
use crate::pane_group::PanesLayout;
use crate::quit_warning::UnsavedStateSummary;
use crate::workspace::Workspace;

const SCRIPT_NAME: &str = "warper-update";
const POLL_INTERVAL: Duration = Duration::from_secs(1);
// shortcut: stops watching after 30 minutes; a restart confirmed later still updates Warper, it
// just has to be quit by hand. Upgrade if updates need longer than that.
const POLLS: u32 = 1800;

/// Opens a tab running the bundled `warper-update` script, the only code that ever reaches the
/// network, and quits once the script has started its relauncher.
pub fn open_update_tab(workspace: &mut Workspace, ctx: &mut ViewContext<Workspace>) {
    let exe = std::env::current_exe().unwrap_or_default();
    let pid = std::process::id();
    let marker = state_dir().join(format!("restart-{pid}"));
    let _ = fs::remove_file(&marker);
    workspace.add_tab_with_pane_layout(
        PanesLayout::Template(PaneTemplateType::PaneTemplate {
            cwd: PathBuf::new(),
            commands: vec![CommandTemplate {
                exec: update_command(&exe, cfg!(target_os = "macos"), pid, &marker),
            }],
            is_focused: Some(true),
            pane_mode: PaneMode::Terminal,
            shell: None,
        }),
        Arc::new(HashMap::new()),
        Some("Update Warper".to_string()),
        ctx,
    );
    quit_when_restart_requested(marker, POLLS, ctx);
}

/// The installed Warper version, such as `v0.3.5`, read once from where packaging records it and
/// `warper-update` reads it. `None` for a source build, which records none.
pub fn installed_version() -> Option<&'static str> {
    static VERSION: OnceLock<Option<String>> = OnceLock::new();
    VERSION
        .get_or_init(|| {
            let exe = std::env::current_exe().ok()?;
            installed_version_at(&exe, cfg!(target_os = "macos"))
        })
        .as_deref()
}

/// Reads `CFBundleShortVersionString` from the app bundle's `Info.plist` on macOS, else the
/// `VERSION` file next to the executable.
fn installed_version_at(exe: &Path, in_app_bundle: bool) -> Option<String> {
    let dir = exe.parent()?;
    let version = if in_app_bundle {
        plist::Value::from_file(dir.join("../Info.plist"))
            .ok()?
            .as_dictionary()?
            .get("CFBundleShortVersionString")?
            .as_string()?
            .to_owned()
    } else {
        fs::read_to_string(dir.join("VERSION")).ok()?
    };
    let version = version.trim();
    (!version.is_empty()).then(|| format!("v{version}"))
}

/// The script sits next to the executable, or in the app bundle's `Resources` on macOS.
fn update_command(exe: &Path, in_app_bundle: bool, pid: u32, marker: &Path) -> String {
    let dir = exe.parent().unwrap_or(Path::new(""));
    let script = if in_app_bundle {
        dir.join("../Resources").join(SCRIPT_NAME)
    } else {
        dir.join(SCRIPT_NAME)
    };
    format!(
        "WARPER_PID={pid} WARPER_RESTART_FILE={} sh {}",
        quote(marker),
        quote(&script)
    )
}

fn quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"))
}

/// Consumes the file the script leaves to ask Warper to quit; true if it was there.
fn take_restart_request(marker: &Path) -> bool {
    fs::remove_file(marker).is_ok()
}

/// The script has already asked the user to restart, so the update tab's own running command must
/// not raise the Quit dialog; unsaved code is still protected by it.
fn restart_termination_mode(has_unsaved_code_changes: bool) -> TerminationMode {
    if has_unsaved_code_changes {
        TerminationMode::Cancellable
    } else {
        TerminationMode::ForceTerminate
    }
}

fn quit_when_restart_requested(marker: PathBuf, polls_left: u32, ctx: &mut ViewContext<Workspace>) {
    ctx.spawn(
        async move {
            Timer::after(POLL_INTERVAL).await;
            (take_restart_request(&marker), marker)
        },
        move |_, (requested, marker), ctx| {
            if requested {
                let unsaved = UnsavedStateSummary::for_app(ctx).has_unsaved_code_changes();
                ctx.terminate_app(restart_termination_mode(unsaved), None);
            } else if polls_left > 0 {
                quit_when_restart_requested(marker, polls_left - 1, ctx);
            }
        },
    );
}

#[cfg(test)]
#[path = "lean_update_tests.rs"]
mod tests;
