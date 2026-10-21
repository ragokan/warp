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
