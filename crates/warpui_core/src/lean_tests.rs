use super::*;

#[test]
fn lean_surface_and_setting_policy_preserves_terminal_preferences() {
    for (name, description) in [
        ("workspace:new_agent", "New Agent"),
        ("terminal:share", "Share Selected Block"),
        ("", "Enable App Analytics"),
        ("", "Invite People…"),
        ("", "Enable '@' Context Menu in Terminal Mode"),
        ("", "Enable Error Underlining"),
        ("", "Disable Command Corrections"),
        ("", "Open in Warp"),
        ("", "New File"),
        ("", "Save File As"),
        ("", "Open Repository"),
        ("", "Open Global Search"),
        ("", "Enable Code as Default Editor"),
        ("", "Toggle Conversation Details Panel"),
        ("", "Toggle Hide CLI Responses"),
        ("", "Go to Line"),
    ] {
        assert_eq!(
            surface_is_allowed(name, description),
            !cfg!(feature = "lean")
        );
    }
    for (hierarchy, key) in [
        ("agents.warp_agent", "is_any_ai_enabled"),
        ("privacy", "telemetry_enabled"),
        ("appearance.text", "ai_font_name"),
        (
            "terminal.input",
            "outline_codebase_symbols_for_at_context_menu",
        ),
    ] {
        assert_eq!(
            setting_is_allowed(Some(hierarchy), key),
            !cfg!(feature = "lean")
        );
    }
    assert!(surface_is_allowed("input:copy", "Copy Command"));
    assert!(setting_is_allowed(Some("notifications"), "preferences"));
    assert!(setting_is_allowed(Some("appearance.text"), "font_size"));
    assert!(setting_is_allowed(
        Some("terminal.input"),
        "input_box_type_setting"
    ));
    assert_eq!(widget_is_allowed("DefaultSessionModeWidget"), !IS_LEAN);
    assert!(slash_is_allowed("/theme"));
    assert_eq!(slash_is_allowed("/mcp"), !cfg!(feature = "lean"));
    for url in ["https://docs.warp.dev/terminal", "mailto:support@warp.dev"] {
        assert_eq!(link_is_allowed(url), !cfg!(feature = "lean"));
    }
    assert!(link_is_allowed("https://docs.warp.dev/privacy"));
    assert!(link_is_allowed("https://app.warp.dev/account"));
}

#[test]
fn display_name_preserves_upstream_outside_lean() {
    assert_eq!(
        app_name(),
        if cfg!(feature = "lean") {
            "Warper"
        } else {
            "Warp"
        }
    );
}
