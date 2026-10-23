use std::process;

use warpui_core::App;
use warpui_core::r#async::FutureExt as _;

use super::*;

/// APP-5243 / WARP-CLIENT-DEV-XT3: an empty path must never reach the platform watcher, because
/// macOS turns it into a null-`CFError` release that traps and kills the process.
#[test]
fn empty_paths_never_reach_the_platform_watcher() {
    assert!(ensure_watchable_path(Path::new("")).is_err());

    let directory = std::env::temp_dir();
    assert_eq!(ensure_watchable_path(&directory).ok(), Some(directory));
}

fn new_test_directory(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("watcher-{name}-{}", process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn background_watcher_starts_with_the_first_registered_path() {
    let directory = new_test_directory("lazy-start");
    let watched_directory = directory.clone();

    App::test((), |mut app| async move {
        let watcher =
            app.add_model(|ctx| BulkFilesystemWatcher::new(Duration::from_millis(10), ctx));

        watcher.read(&app, |watcher, _| {
            assert!(
                watcher.pending_start.is_some(),
                "a watcher with no paths should not start its background thread"
            );
        });

        watcher
            .update(&mut app, |watcher, _| {
                watcher.register_path(
                    &directory,
                    WatchFilter::accept_all(),
                    RecursiveMode::NonRecursive,
                )
            })
            .with_timeout(Duration::from_secs(5))
            .await
            .expect("timed out registering the path")
            .expect("registering the path failed");

        watcher.read(&app, |watcher, _| {
            assert!(watcher.pending_start.is_none());
        });
    });
    std::fs::remove_dir_all(watched_directory).unwrap();
}

#[test]
fn changes_under_a_registered_path_are_reported() {
    let directory = new_test_directory("reports-changes");
    let file = directory.join("created.txt");
    let watched_directory = directory.clone();

    App::test((), |mut app| async move {
        let watcher =
            app.add_model(|ctx| BulkFilesystemWatcher::new(Duration::from_millis(10), ctx));
        let (events_tx, events_rx) = async_channel::unbounded();
        app.update(|ctx| {
            ctx.subscribe_to_model(&watcher, move |_, event, _| {
                let _ = events_tx.try_send(event.clone());
            });
        });

        watcher
            .update(&mut app, |watcher, _| {
                watcher.register_path(
                    &directory,
                    WatchFilter::accept_all(),
                    RecursiveMode::NonRecursive,
                )
            })
            .with_timeout(Duration::from_secs(5))
            .await
            .expect("timed out registering the path")
            .expect("registering the path failed");
        std::fs::write(&file, "contents").unwrap();

        let event = events_rx
            .recv()
            .with_timeout(Duration::from_secs(5))
            .await
            .expect("timed out waiting for the change")
            .expect("event channel closed");
        assert!(
            event
                .added_or_updated_iter()
                .any(|path| path.file_name() == file.file_name()),
            "expected {file:?} in {event:?}"
        );
    });
    std::fs::remove_dir_all(watched_directory).unwrap();
}
