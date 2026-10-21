//! Terminal-only surface policy; stored preferences retain their upstream representation.

pub const IS_LEAN: bool = cfg!(feature = "lean");

pub const fn app_name() -> &'static str {
    if cfg!(feature = "lean") {
        "Warper"
    } else {
        "Warp"
    }
}

pub fn surface_is_allowed(name: &str, description: &str) -> bool {
    !cfg!(feature = "lean")
        || (!has_removed_words(name)
            && !has_removed_words(description)
            && !name.starts_with("code_view:")
            && !description
                .to_ascii_lowercase()
                .contains("'@' context menu")
            && ![
                "file editor",
                "new file",
                "save file",
                "open repository",
                "global search",
                "code as default editor",
                "cli responses",
                "go to line",
                "left panel",
                "open in warp",
                "view in warp",
                "edit in warp",
                "code editor",
                "code review",
                "command corrections",
                "error underlining",
                "project explorer",
                "block filter",
                "filter block",
                "tab color",
                "tab group",
                "pin tab",
                "unpin tab",
                "launch configuration",
                "tab config",
                "resource center",
                "warp essentials",
                "what's new",
                "github issues",
                "learn more",
                "more info",
                "contact support",
            ]
            .iter()
            .any(|phrase| description.to_ascii_lowercase().contains(phrase))
            && !matches!(
                name,
                "terminal:accept_prompt_suggestions"
                    | "requested_command:edit"
                    | "code_diff_view:edit_requested_edit"
                    | "workspace:init_project_rules"
                    | "workspace:create_personal_folder"
                    | "workspace:create_personal_env_vars"
            ))
}

fn has_removed_words(text: &str) -> bool {
    let mut normalized = String::with_capacity(text.len());
    let mut previous = ' ';
    for character in text.chars() {
        if previous.is_ascii_lowercase() && character.is_ascii_uppercase() {
            normalized.push(' ');
        }
        normalized.push(character.to_ascii_lowercase());
        previous = character;
    }
    normalized
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|word| {
            matches!(
                word,
                "ai" | "agent"
                    | "agents"
                    | "conversation"
                    | "conversations"
                    | "llm"
                    | "mcp"
                    | "oz"
                    | "drive"
                    | "notebook"
                    | "notebooks"
                    | "telemetry"
                    | "analytics"
                    | "crash"
                    | "share"
                    | "shared"
                    | "sharing"
                    | "billing"
                    | "upgrade"
                    | "pricing"
                    | "credit"
                    | "credits"
                    | "workflow"
                    | "workflows"
                    | "vim"
                    | "markdown"
                    | "bookmark"
                    | "bookmarks"
                    | "sticky"
                    | "compact"
                    | "changelog"
                    | "feedback"
                    | "slack"
                    | "community"
                    | "documentation"
                    | "docs"
                    | "help"
                    | "blog"
                    | "referral"
                    | "referrals"
                    | "invite"
                    | "team"
                    | "teams"
            )
        })
}

pub fn setting_is_allowed(hierarchy: Option<&str>, key: &str) -> bool {
    if !cfg!(feature = "lean") {
        return true;
    }
    let path = hierarchy.map_or_else(|| key.to_owned(), |h| format!("{h}.{key}"));
    !has_removed_words(&path)
        && !path.starts_with("code.")
        && !path.starts_with("appearance.vertical_tabs.")
        && !path.starts_with("cloud_platform.")
        && !path.starts_with("appearance.zero_state.")
        && !path.contains("tab_config")
        && !path.contains("at_context_menu")
        && !path.contains("open_in_warp")
        && !matches!(
            path.as_str(),
            "appearance.theme"
                | "general.autoupdate_enabled"
                | "terminal.show_terminal_zero_state_block"
                | "general.default_session_mode"
                | "appearance.tabs.header_toolbar_chip_selection"
                | "appearance.spacing"
                | "appearance.window.left_panel_visibility_across_tabs"
                | "general.snackbar_enabled"
                | "appearance.tabs.preserve_active_tab_color"
                | "appearance.tabs.directory_tab_colors"
                | "terminal.input.command_corrections"
                | "terminal.input.error_underlining_enabled"
                | "text_editing.code_editor_line_number_mode"
        )
}

pub fn slash_is_allowed(name: &str) -> bool {
    !cfg!(feature = "lean")
        || matches!(
            name,
            "/theme"
                | "/exit"
                | "/logout"
                | "/view-logs"
                | "/open-settings-file"
                | "/rename-tab"
                | "/open-file"
        )
}

pub fn link_is_allowed(url: &str) -> bool {
    !IS_LEAN
        || url.contains("/privacy")
        || (!url.starts_with("https://docs.warp.dev/") && url != "mailto:support@warp.dev")
}

pub fn widget_is_allowed(id: &str) -> bool {
    !cfg!(feature = "lean")
        || ![
            "DefaultSessionModeWidget",
            "AtContextMenuInTerminalModeWidget",
            "WorkflowsInCommandSearch",
            "ToolsPanelProjectExplorerWidget",
            "ToolsPanelStateScopeWidget",
            "CompactModeWidget",
            "SnackbarHeaderWidget",
            "ErrorUnderliningWidget",
            "CommandCorrectionsWidget",
            "VimModeWidget",
            "PreserveActiveTabColorWidget",
            "VerticalTabsWidget",
            "ShowVerticalTabPanelInRestoredWindowsWidget",
            "HideTitleBarSearchBarInVerticalTabsWidget",
            "UseLatestUserPromptAsConversationTitleInTabNamesWidget",
            "DirectoryTabColorsWidget",
        ]
        .iter()
        .any(|name| id.rsplit("::").next() == Some(*name))
}

#[cfg(test)]
#[path = "lean_tests.rs"]
mod tests;
