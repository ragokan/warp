use std::fs;
use std::path::Path;

use super::*;

#[test]
fn script_sits_next_to_the_linux_binary() {
    let exe = Path::new("/home/me/.local/lib/warp-lean/warp-oss");
    let marker = Path::new("/home/me/.local/state/warp-oss/restart-7");
    assert_eq!(
        update_command(exe, false, 7, marker),
        "WARPER_PID=7 WARPER_RESTART_FILE='/home/me/.local/state/warp-oss/restart-7' \
         sh '/home/me/.local/lib/warp-lean/warper-update'"
    );
}

#[test]
fn script_is_in_the_resources_of_the_macos_bundle() {
    let exe = Path::new("/Applications/Warper.app/Contents/MacOS/warp-oss");
    let command = update_command(exe, true, 7, Path::new("/m"));
    assert!(
        command
            .ends_with("sh '/Applications/Warper.app/Contents/MacOS/../Resources/warper-update'"),
        "{command}"
    );
}

#[test]
fn quotes_in_paths_cannot_end_the_shell_string() {
    let command = update_command(
        Path::new("/home/o'brien/warp-oss"),
        false,
        7,
        Path::new("/m'x"),
    );
    assert_eq!(
        command,
        r"WARPER_PID=7 WARPER_RESTART_FILE='/m'\''x' sh '/home/o'\''brien/warper-update'"
    );
}

#[test]
fn a_restart_request_is_consumed_once() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("restart-1");
    assert!(!take_restart_request(&marker));
    fs::write(&marker, "").unwrap();
    assert!(take_restart_request(&marker));
    assert!(!take_restart_request(&marker));
}

#[test]
fn the_restart_quit_skips_the_dialog_unless_code_is_unsaved() {
    assert!(matches!(
        restart_termination_mode(false),
        TerminationMode::ForceTerminate
    ));
    assert!(matches!(
        restart_termination_mode(true),
        TerminationMode::Cancellable
    ));
}
