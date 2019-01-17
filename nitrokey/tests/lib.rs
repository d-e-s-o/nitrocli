#[test]
fn get_library_version() {
    let version = nitrokey::get_library_version();

    assert!(version.git.is_empty() || version.git.starts_with("v"));
    assert!(version.major > 0);
}
