//! Explicit non-Windows contract for the first Windows-only primitive.

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};

use super::{WindowConfig, WindowError};

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
    pub fn poll_events(&self) -> Result<(), WindowError> {
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
