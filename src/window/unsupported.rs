//! Explicit non-Windows contract for the first Windows-only primitive.

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};

use super::{WindowConfig, WindowError, WindowEvent};

/// Placeholder preserving the public host contract on targets not yet implemented.
pub struct Window {
    _private: (),
}

impl Window {
    /// Reports the structured unsupported-platform result instead of pretending
    /// that a native target exists.
    pub fn new(_: WindowConfig) -> Result<Self, WindowError> {
        Err(WindowError::UnsupportedPlatform)
    }

    /// There are no native messages to dispatch on an unsupported target.
    pub fn poll_events(&self) -> Result<Vec<WindowEvent>, WindowError> {
        Err(WindowError::UnsupportedPlatform)
    }

    /// No platform window exists, so it cannot have requested closure.
    pub fn close_requested(&self) -> bool {
        false
    }

    /// There is no native allocation to release on an unsupported target.
    pub fn close(&mut self) -> Result<(), WindowError> {
        Err(WindowError::UnsupportedPlatform)
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Err(HandleError::NotSupported)
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Err(HandleError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
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
}
