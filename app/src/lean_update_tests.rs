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

#[test]
fn installed_version_is_read_next_to_the_linux_binary() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("VERSION"), "0.3.5\n").unwrap();

    let version = installed_version_at(&directory.path().join("warp-oss"), false);

    assert_eq!(version.as_deref(), Some("v0.3.5"));
}

#[test]
fn installed_version_is_read_from_the_macos_bundle_plist() {
    let directory = tempfile::tempdir().unwrap();
    let contents = directory.path().join("Warper.app/Contents");
    fs::create_dir_all(contents.join("MacOS")).unwrap();
    fs::write(
        contents.join("Info.plist"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleShortVersionString</key>
	<string>0.3.5</string>
	<key>WarpVersion</key>
	<string>lean-v0.3.5</string>
</dict>
</plist>
"#,
    )
    .unwrap();

    let version = installed_version_at(&contents.join("MacOS/warp-oss"), true);

    assert_eq!(version.as_deref(), Some("v0.3.5"));
}

#[test]
fn a_source_build_has_no_installed_version() {
    let directory = tempfile::tempdir().unwrap();
    let exe = directory.path().join("warp-oss");

    assert_eq!(installed_version_at(&exe, false), None);
    assert_eq!(installed_version_at(&exe, true), None);
    fs::write(directory.path().join("VERSION"), "\n").unwrap();
    assert_eq!(installed_version_at(&exe, false), None);
}
