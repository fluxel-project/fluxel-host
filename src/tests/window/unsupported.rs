use super::*;

#[test]
fn every_window_operation_reports_the_explicit_unsupported_contract() {
    let config = WindowConfig::new("unsupported", 320, 240).unwrap();
    assert!(matches!(
        Window::new(config),
        Err(WindowError::UnsupportedPlatform)
    ));

    // `Window::new` cannot construct a target that does not exist. Keep a
    // private placeholder solely to verify the remaining methods cannot
    // silently succeed if a caller somehow retains this type.
    let mut window = Window { _private: () };
    assert_eq!(
        window.poll_events().unwrap_err(),
        WindowError::UnsupportedPlatform
    );
    assert!(!window.close_requested());
    assert_eq!(
        window.close().unwrap_err(),
        WindowError::UnsupportedPlatform
    );
    assert!(window.window_handle().is_err());
    assert!(window.display_handle().is_err());
}
