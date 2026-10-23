use std::process;

use warpui_core::App;
use warpui_core::lean::IS_LEAN;

use super::*;

#[test]
fn home_directory_is_watched_except_in_lean() {
    let home_dir = std::env::temp_dir().join(format!("watcher-home-{}", process::id()));
    std::fs::create_dir_all(&home_dir).unwrap();
    let watched_home_dir = home_dir.clone();

    App::test((), |mut app| async move {
        let home_watcher = app.add_model(|ctx| HomeDirectoryWatcher::new(home_dir, ctx));

        let is_watching = home_watcher.read(&app, |home_watcher, ctx| {
            home_watcher._watcher.as_ref(ctx).pending_start.is_none()
        });
        assert_eq!(is_watching, !IS_LEAN);
    });
    std::fs::remove_dir_all(watched_home_dir).unwrap();
}
