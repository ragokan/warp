use settings::schema::SettingSchemaEntry;
use settings::{Setting, SettingSurfaces, SettingsMode};

use super::{IsCloudConversationStorageEnabled, IsCrashReportingEnabled, IsTelemetryEnabled};

#[test]
fn privacy_settings_apply_to_gui_and_tui() {
    for storage_key in [
        IsTelemetryEnabled::toml_key(),
        IsCrashReportingEnabled::toml_key(),
        IsCloudConversationStorageEnabled::toml_key(),
    ] {
        let entry = inventory::iter::<SettingSchemaEntry>
            .into_iter()
            .find(|entry| entry.storage_key == storage_key)
            .unwrap_or_else(|| panic!("missing schema entry for {storage_key}"));
        let surfaces = (entry.surfaces_fn)();

        assert_eq!(surfaces, SettingSurfaces::ALL, "{storage_key}");
        assert!(surfaces.includes(SettingsMode::Gui), "{storage_key}");
        assert!(surfaces.includes(SettingsMode::Tui), "{storage_key}");
    }
}

#[test]
#[cfg(feature = "lean")]
fn lean_privacy_snapshot_disables_effective_collection() {
    let snapshot = super::PrivacySettingsSnapshot::mock();
    assert!(snapshot.is_telemetry_enabled);
    assert!(snapshot.is_crash_reporting_enabled);
    assert!(snapshot.is_telemetry_force_enabled);
    assert!(snapshot.should_disable_telemetry());
    assert!(!snapshot.is_telemetry_enabled());
    assert!(!snapshot.is_crash_reporting_enabled());
    assert!(!snapshot.is_telemetry_force_enabled());
    assert!(!snapshot.should_collect_ai_ugc_telemetry());
}

#[test]
#[cfg(feature = "lean")]
fn lean_cloud_privacy_migration_preserves_hidden_values() {
    use std::sync::Arc;

    use warpui::{App, SingletonEntity};

    use super::{MockAuthClient, PrivacySettings, SyncedUserSettings};
    use crate::auth::auth_state::AuthState;
    use crate::server::cloud_objects::fake_object_client::FakeObjectClient;
    use crate::server::cloud_objects::test_utils::{create_update_manager_struct, initialize_app};
    use crate::settings::cloud_preferences::Platform;

    App::test(crate::ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let fake = FakeObjectClient::default();
        for key in [
            IsTelemetryEnabled::storage_key(),
            IsCrashReportingEnabled::storage_key(),
            IsCloudConversationStorageEnabled::storage_key(),
        ] {
            fake.seed_preference(key, "false", Platform::Global);
        }
        let manager = create_update_manager_struct(&mut app, Arc::new(fake.clone()));
        app.update(|ctx| {
            manager.update_manager.update(ctx, |manager, ctx| {
                manager.mock_initial_load(fake.snapshot_as_initial_load_response(), ctx);
            });
        });
        PrivacySettings::handle(&app).update(&mut app, |privacy, ctx| {
            privacy.auth_state = Arc::new(AuthState::new_logged_out_for_test());
            let mut auth_client = MockAuthClient::new();
            auth_client
                .expect_get_user_settings()
                .times(0)
                .returning(|| Ok(Some(SyncedUserSettings::default())));
            privacy.auth_client = Arc::new(auth_client);
            privacy.fetch_or_update_settings(ctx);
        });
        warpui::r#async::Timer::after(std::time::Duration::from_millis(100)).await;
        PrivacySettings::handle(&app).update(&mut app, |privacy, ctx| {
            privacy.handle_warp_drive_objects_loaded((), ctx);
            assert!(privacy.is_telemetry_enabled);
            assert!(privacy.is_crash_reporting_enabled);
            assert!(privacy.is_cloud_conversation_storage_enabled);
        });
    });
}
