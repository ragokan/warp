use super::*;

#[test]
fn lean_banner_keeps_errors_and_privacy_without_help_links() {
    let content = BannerTextContent::<()>::formatted_text(vec![
        FormattedTextFragment::plain_text("Shell error"),
        FormattedTextFragment::hyperlink("More info", "https://docs.warp.dev/terminal"),
        FormattedTextFragment::hyperlink("Privacy policy", "https://warp.dev/privacy"),
    ]);
    assert_eq!(content.text.len(), 2);
    assert_eq!(content.text[0].text, "Shell error");
    assert_eq!(content.text[1].text, "Privacy policy");
}
