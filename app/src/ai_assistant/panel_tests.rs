use warpui::App;

use super::REFRESH_TICKS_ARMED;

#[test]
fn refresh_tick_is_not_armed_in_lean() {
    App::test((), |mut app| async move {
        crate::workspace::view::tests::initialize_app(&mut app);
        let armed_before = REFRESH_TICKS_ARMED.with(|armed| armed.get());

        // Every workspace builds an AI panel, shown or not.
        let _workspace = crate::workspace::view::tests::mock_workspace(&mut app);

        let armed_after = REFRESH_TICKS_ARMED.with(|armed| armed.get());
        assert_eq!(armed_after > armed_before, !cfg!(feature = "lean"));
    });
}
