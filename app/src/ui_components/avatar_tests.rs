use super::*;

#[test]
fn avatar_file_prefers_png_then_jpg_then_jpeg() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(find_avatar_file(dir.path()), None);
    for name in ["avatar.jpeg", "avatar.jpg", "avatar.png"] {
        std::fs::write(dir.path().join(name), b"").unwrap();
        assert_eq!(find_avatar_file(dir.path()), Some(dir.path().join(name)));
    }
}

#[test]
fn avatar_file_ignores_other_names_and_directories() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("avatar.gif"), b"").unwrap();
    std::fs::write(dir.path().join("Avatar.PNG.bak"), b"").unwrap();
    std::fs::create_dir(dir.path().join("avatar.png")).unwrap();
    assert_eq!(find_avatar_file(dir.path()), None);
    std::fs::write(dir.path().join("avatar.jpg"), b"").unwrap();
    assert_eq!(
        find_avatar_file(dir.path()),
        Some(dir.path().join("avatar.jpg"))
    );
}

#[test]
fn avatar_source_is_the_built_in_image_without_a_config_file() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        avatar_source(dir.path()),
        AssetSource::Bundled {
            path: "bundled/jpg/warper_avatar.jpg"
        }
    );
}

#[test]
fn avatar_source_prefers_the_config_file_over_the_built_in_image() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("avatar.png");
    std::fs::write(&file, b"").unwrap();
    assert_eq!(
        avatar_source(dir.path()),
        AssetSource::LocalFile {
            path: file.to_string_lossy().into_owned(),
            content_version: None
        }
    );
}
