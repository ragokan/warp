use warpui_core::AssetProvider;

use super::Assets;

#[test]
fn async_assets_follow_the_lean_and_standalone_features() {
    let standalone = cfg!(feature = "standalone");
    let lean = cfg!(feature = "lean");
    assert_eq!(
        Assets
            .get("async/png/onboarding/hoa_welcome_banner.png")
            .is_err(),
        standalone || lean
    );
    assert_eq!(Assets.get("async/jpg/marble_bg.jpg").is_err(), standalone);
    assert!(Assets.get("bundled/svg/droid.svg").is_ok());
}
