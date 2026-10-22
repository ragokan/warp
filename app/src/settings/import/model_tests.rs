use std::time::Duration;

use warpui::r#async::Timer;
use warpui::platform::test::system_font_loads;
use warpui::{App, UpdateModel};

use super::{ImportedConfigModel, SEARCH_STARTS};

const SETTLE: Duration = Duration::from_millis(50);

#[test]
fn search_starts_and_enumerates_system_fonts_once_when_first_requested() {
    App::test((), |mut app| async move {
        let loads_before = system_font_loads();
        let model = app.add_singleton_model(ImportedConfigModel::new);
        Timer::after(SETTLE).await;
        assert!(!app.update_model(&model, |model, _| model.is_started()));
        assert_eq!(system_font_loads(), loads_before);

        // The font cache serves a second search from memory, so only the start count shows whether
        // `ensure_search_started` skipped it.
        let starts_before = SEARCH_STARTS.with(|starts| starts.get());
        for _ in 0..2 {
            app.update_model(&model, |model, ctx| model.ensure_search_started(ctx));
            Timer::after(SETTLE).await;
            assert!(app.update_model(&model, |model, _| model.is_started()));
            assert_eq!(system_font_loads(), loads_before + 1);
            assert_eq!(SEARCH_STARTS.with(|starts| starts.get()), starts_before + 1);
        }
    });
}
