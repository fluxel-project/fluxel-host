use super::*;

#[test]
fn rejects_zero_client_extent() {
    assert_eq!(
        WindowConfig::new("Fluxel", 0, 720).unwrap_err(),
        WindowError::ZeroClientExtent
    );
    assert_eq!(
        WindowConfig::new("Fluxel", 1280, 0).unwrap_err(),
        WindowError::ZeroClientExtent
    );
}

#[test]
fn rejects_embedded_nul_in_title() {
    assert_eq!(
        WindowConfig::new("Fluxel\0Host", 1280, 720).unwrap_err(),
        WindowError::TitleContainsNul
    );
}

#[test]
fn default_visibility_is_background_and_can_be_hidden() {
    let config = WindowConfig::new("Fluxel", 1280, 720).unwrap();
    assert_eq!(config.visibility(), WindowVisibility::Background);

    let config = config.with_visibility(WindowVisibility::Hidden);
    assert_eq!(config.visibility(), WindowVisibility::Hidden);
}
