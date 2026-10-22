use std::time::Duration;

use warpui::r#async::Timer;
use warpui::platform::test::{set_family_name, system_font_loads};
use warpui::{App, AppContext, TypedActionView, ViewHandle};

use super::*;
use crate::settings_view::SettingsView;
use crate::settings_view::pane_manager::SettingsPaneManager;
use crate::settings_view::settings_page::SettingsPage;
use crate::workspace::WorkspaceAction;
use crate::workspace::view::tests::{initialize_app, mock_workspace};

const SETTLE: Duration = Duration::from_millis(50);

fn appearance_page(
    settings_view: &ViewHandle<SettingsView>,
    ctx: &AppContext,
) -> ViewHandle<AppearanceSettingsPageView> {
    let Some(SettingsPage {
        view_handle: SettingsPageViewHandle::Appearance(page),
        ..
    }) = settings_view
        .as_ref(ctx)
        .settings_page(SettingsSection::Appearance)
    else {
        panic!("the settings view has an Appearance page");
    };
    page.clone()
}

fn font_info(family_name: &str) -> FontInfo {
    FontInfo {
        family_name: family_name.to_string(),
        #[cfg(target_os = "macos")]
        font_names: vec![],
        is_monospace: true,
    }
}

#[cfg(feature = "lean")]
#[test]
fn lean_loads_system_fonts_when_the_font_dropdown_opens_not_when_the_page_is_selected() {
    App::test((), |mut app| async move {
        let loads_before = system_font_loads();
        initialize_app(&mut app);
        let workspace = mock_workspace(&mut app);
        let window_id = workspace.update(&mut app, |_, ctx| ctx.window_id());
        Timer::after(SETTLE).await;
        assert_eq!(system_font_loads(), loads_before);

        // Opening Settings creates the settings tab through the same `SettingsPane::new` that
        // session restore uses, which selects the default page and runs its selection hook.
        workspace.update(&mut app, |workspace, ctx| {
            workspace.handle_action(&WorkspaceAction::ShowSettings, ctx);
        });
        Timer::after(SETTLE).await;
        let settings_view =
            app.read(|ctx| SettingsPaneManager::as_ref(ctx).settings_view(window_id));
        app.read(|ctx| {
            assert_eq!(
                settings_view.as_ref(ctx).current_settings_section(),
                SettingsSection::Appearance
            );
        });
        assert_eq!(system_font_loads(), loads_before);

        let dropdown = app.read(|ctx| {
            appearance_page(&settings_view, ctx)
                .as_ref(ctx)
                .font_family_dropdown
                .clone()
        });
        for _ in 0..3 {
            dropdown.update(&mut app, |dropdown, ctx| dropdown.toggle_expanded(ctx));
            Timer::after(SETTLE).await;
            assert_eq!(system_font_loads(), loads_before + 1);
        }
    });
}

#[test]
fn font_list_arriving_during_a_search_keeps_a_match_selected() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let workspace = mock_workspace(&mut app);
        let window_id = workspace.update(&mut app, |_, ctx| ctx.window_id());
        let settings_view =
            app.read(|ctx| SettingsPaneManager::as_ref(ctx).settings_view(window_id));
        let page = app.read(|ctx| appearance_page(&settings_view, ctx));
        let dropdown = app.read(|ctx| page.as_ref(ctx).font_family_dropdown.clone());

        // The configured font is Hack, and the user searches for another one before the list
        // arrives, so only the placeholder item exists and nothing matches yet.
        let family_name = set_family_name("Hack");
        dropdown.update(&mut app, |dropdown, ctx| {
            dropdown.set_filter_query_for_test("dejavu", ctx);
        });

        page.update(&mut app, |page, ctx| {
            page.set_system_fonts(
                vec![
                    (None, font_info("DejaVu Sans Mono")),
                    (None, font_info("Hack")),
                ],
                ctx,
            );
        });
        Timer::after(SETTLE).await;

        // Enter acts on the selected item, so a match has to be selected.
        app.read(|ctx| {
            assert_eq!(
                dropdown.as_ref(ctx).selected_item_label().as_deref(),
                Some("DejaVu Sans Mono")
            );
        });

        // The override is scoped to the test, so later tests on this thread do not inherit it.
        let monospace_family = app.read(|ctx| Appearance::as_ref(ctx).monospace_font_family());
        let resolved_name =
            |app: &App| app.read(|ctx| ctx.font_cache().load_family_name_from_id(monospace_family));
        assert_eq!(resolved_name(&app).as_deref(), Some("Hack"));
        drop(family_name);
        assert_eq!(resolved_name(&app), None);
    });
}
