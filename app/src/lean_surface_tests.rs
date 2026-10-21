use regex::Regex;
use warpui::App;
use warpui::keymap::DescriptionContext;

use crate::search::slash_command_menu::static_commands::commands::COMMAND_REGISTRY;

#[test]
fn lean_registered_surfaces_exclude_removed_features() {
    App::test((), |mut app| async move {
        crate::test_util::terminal::initialize_app_for_terminal_view(&mut app);
        app.update(|ctx| {
            use crate::workspace::header_toolbar_item::HeaderToolbarItemKind;
            assert!(!HeaderToolbarItemKind::CodeReview.is_supported(ctx));
            assert!(!HeaderToolbarItemKind::ToolsPanel.is_supported(ctx));
            assert!(!crate::util::tooltips::should_show_open_in_warp_link(
                std::path::Path::new("/tmp/example.rs"), ctx,
            ));
            ctx.add_singleton_model(|_| crate::gpu_state::GPUState::new());
            crate::ai::init(ctx);
            crate::code::editor::find::view::init(ctx);
            crate::workspace::init(ctx);
            crate::pane_group::init(ctx);
            crate::terminal::init(ctx);
            crate::input::init(ctx);
            crate::editor::init(ctx);
            onboarding::init(ctx);
            crate::menu::init(ctx);
            crate::tips::tip_view::init(ctx);
            crate::launch_configs::init(ctx);
            crate::workflows::init(ctx);
            crate::themes::theme_chooser::init(ctx);
            crate::themes::theme_creator_modal::init(ctx);
            crate::themes::theme_deletion_modal::init(ctx);
            crate::root_view::init(ctx);
            crate::voltron::init(ctx);
            crate::auth::init(ctx);
            crate::reward_view::init(ctx);
            crate::view_components::find::init(ctx);
            crate::prompt::editor_modal::init(ctx);
            crate::ai::blocklist::agent_view::editor::init(ctx);
            crate::undo_close::init(ctx);
            crate::billing::shared_objects_creation_denied_modal::init(ctx);
            crate::tab_configs::new_worktree_modal::init(ctx);
            crate::tab_configs::params_modal::init(ctx);
            crate::ai::blocklist::init(ctx);
            crate::ai::blocklist::block::status_bar::init(ctx);
            crate::drive::index::init(ctx);
            crate::drive::sharing::dialog::init(ctx);
            crate::ai_assistant::panel::init(ctx);
            crate::settings_view::update_environment_form::init(ctx);
            crate::env_vars::env_var_collection_block::init(ctx);
            crate::context_chips::display_menu::init(ctx);
            crate::context_chips::node_version_popup::init(ctx);
            crate::env_vars::view::env_var_collection::init(ctx);
            crate::ai::agent::todos::popup::init(ctx);
            crate::terminal::view::init_environment::mode_selector::init(ctx);
            crate::coding_entrypoints::project_buttons::init(ctx);
            crate::notebooks::init(ctx);

            let forbidden = Regex::new(
                r"(?i)(?:^|[^a-z])(code[_ -]view|new[_ -]file|save[_ -]file|open[_ -]repository|global[_ -]search|code[_ -]as[_ -]default[_ -]editor|conversations?|cli[_ -]responses|go[_ -]to[_ -]line|left[_ -]panel|help|blog|docs|learn[_ -]more|more[_ -]info|open[_ -]in[_ -]warp|workflows?|vim|markdown|bookmarks?|sticky|compact mode|changelog|feedback|slack|community|documentation|vertical[_ -]tabs|tab[_ -]groups?|pinned[_ -]tabs|command[_ -]corrections|error[_ -]underlining|@.*context[_ -]menu|code[_ -]editor|code[_ -]review|project[_ -]explorer|file[_ -]editor|block[_ -]filter|filter[_ -]block|tab[_ -]configs?|launch[_ -]configurations?|tab[_ -]colou?rs?|pin[_ -]tab|ai|agents?|llm|mcp|oz|drive|notebooks?|telemetry|analytics|crash reporting|shar(?:e|ed|ing)|billing|upgrade|pricing|credits?|referrals?|invite|teams?)(?:$|[^a-z])",
            ).unwrap();
            let mut leftovers = Vec::new();
            for binding in ctx.get_key_bindings() {
                let description = binding.description.map(|description| {
                    description.in_context(DescriptionContext::Default)
                }).unwrap_or_default();
                if forbidden.is_match(binding.name) || forbidden.is_match(description) {
                    leftovers.push(format!("binding {}: {description}", binding.name));
                }
            }
            for command in COMMAND_REGISTRY.all_commands() {
                if forbidden.is_match(command.name) || forbidden.is_match(command.description) {
                    leftovers.push(format!("slash {}: {}", command.name, command.description));
                }
            }
            let schema = crate::settings::schema_generation::settings_schema_json(|_| false).unwrap();
            let schema: serde_json::Value = serde_json::from_str(&schema).unwrap();
            collect_forbidden_schema_names(&schema, &forbidden, &mut leftovers);
            assert!(leftovers.is_empty(), "removed surfaces remain:\n{}", leftovers.join("\n"));
        });
    });
}

fn collect_forbidden_schema_names(
    value: &serde_json::Value,
    forbidden: &Regex,
    leftovers: &mut Vec<String>,
) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                if key == "properties"
                    && let Some(properties) = value.as_object()
                {
                    for name in properties.keys() {
                        if name != "is_agent_task_completed_enabled" && forbidden.is_match(name) {
                            leftovers.push(format!("schema key {name}"));
                        }
                    }
                }
                if key == "enum"
                    && let Some(variants) = value.as_array()
                {
                    for variant in variants.iter().filter_map(serde_json::Value::as_str) {
                        if forbidden.is_match(variant) {
                            leftovers.push(format!("schema variant {variant}"));
                        }
                    }
                }
                if key == "is_agent_task_completed_enabled" {
                    continue;
                }
                if key == "description"
                    && let Some(description) = value.as_str()
                    && description != "Default verbosity level, includes help string."
                    && forbidden.is_match(description)
                {
                    leftovers.push(format!("schema description {description}"));
                }
                collect_forbidden_schema_names(value, forbidden, leftovers);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_forbidden_schema_names(value, forbidden, leftovers);
            }
        }
        _ => {}
    }
}

#[test]
fn lean_preserves_excluded_toml_values_without_rewriting() {
    use warpui_extras::user_preferences::UserPreferences;
    use warpui_extras::user_preferences::toml_backed::TomlBackedUserPreferences;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.toml");
    let original =
        "[agents.warp_agent]\nis_any_ai_enabled = true\n[privacy]\ntelemetry_enabled = true\n";
    std::fs::write(&path, original).unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    let (preferences, error) = TomlBackedUserPreferences::new(path.clone());
    assert!(error.is_none());
    preferences
        .write_value_with_hierarchy(
            "is_any_ai_enabled",
            "false".into(),
            Some("agents.warp_agent"),
            None,
        )
        .unwrap();
    preferences
        .remove_value_with_hierarchy("telemetry_enabled", Some("privacy"))
        .unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        modified
    );
    preferences
        .write_value_with_hierarchy("font_size", "14".into(), Some("appearance.text"), None)
        .unwrap();
    assert_eq!(
        preferences
            .read_value_with_hierarchy("is_any_ai_enabled", Some("agents.warp_agent"))
            .unwrap(),
        Some("true".into())
    );
    assert_eq!(
        preferences
            .read_value_with_hierarchy("telemetry_enabled", Some("privacy"))
            .unwrap(),
        Some("true".into())
    );
}

#[test]
fn lean_file_and_markdown_links_use_external_targets() {
    use crate::util::file::external_editor::settings::EditorChoice;
    use crate::util::openable_file_type::{
        EditorLayout, FileTarget, resolve_file_target_with_editor_choice,
    };
    for path in ["/tmp/example.rs", "/tmp/example.md"] {
        for choice in [
            EditorChoice::Warp,
            EditorChoice::SystemDefault,
            EditorChoice::EnvEditor,
        ] {
            let target = resolve_file_target_with_editor_choice(
                std::path::Path::new(path),
                choice,
                true,
                EditorLayout::NewTab,
                None,
            );
            assert_eq!(
                target,
                if choice == EditorChoice::EnvEditor {
                    FileTarget::EnvEditor
                } else {
                    FileTarget::SystemDefault
                }
            );
        }
    }
}

#[test]
fn lean_disables_legacy_product_contexts() {
    use warp_core::context_flag::ContextFlag;
    for flag in [
        ContextFlag::CreateSharedSession,
        ContextFlag::RunWorkflow,
        ContextFlag::LaunchConfigurations,
        ContextFlag::WarpEssentials,
        ContextFlag::ShowMCPServers,
    ] {
        assert!(!flag.is_enabled(), "{flag:?}");
    }
}
