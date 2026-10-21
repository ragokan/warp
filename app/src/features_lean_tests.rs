use super::*;

#[test]
fn lean_keeps_terminal_features_and_pins_services_off() {
    let features = enabled_features();
    for flag in [
        FeatureFlag::OscHyperlinks,
        FeatureFlag::SettingsFile,
        FeatureFlag::NativeShellCompletions,
        FeatureFlag::Ligatures,
        FeatureFlag::PluggableNotifications,
    ] {
        assert!(features.contains(&flag), "{flag:?}");
    }
    for flag in enum_iterator::all::<FeatureFlag>().filter(|flag| !is_allowed_in_lean(*flag)) {
        assert!(!features.contains(&flag), "{flag:?}");
        let was_enabled = flag.is_enabled();
        flag.set_enabled(true);
        assert!(!flag.is_enabled(), "server override revived {flag:?}");
        flag.set_enabled(was_enabled);
    }
    for flag in [
        FeatureFlag::CloudObjects,
        FeatureFlag::FetchGenericStringObjects,
        FeatureFlag::SkipFirebaseAnonymousUser,
        FeatureFlag::SettingsFile,
    ] {
        let was_enabled = flag.is_enabled();
        flag.set_enabled(true);
        assert!(
            flag.is_enabled(),
            "login/settings sync flag blocked: {flag:?}"
        );
        flag.set_enabled(was_enabled);
    }
}
