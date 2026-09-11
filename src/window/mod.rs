//! A minimal fixed-size native window contract.
//!
//! `Window` deliberately exposes only creation, non-blocking event pumping,
//! close observation, explicit destruction, and standard raw handles.  It
//! does not establish a general host runtime or a resize/input policy.

use core::{fmt, num::NonZeroU32};

/// Configuration for one fixed-size native window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowConfig {
    title: String,
    client_width: NonZeroU32,
    client_height: NonZeroU32,
}

impl WindowConfig {
    /// Creates a configuration with a non-empty client extent.
    pub fn new(
        title: impl Into<String>,
        client_width: u32,
        client_height: u32,
    ) -> Result<Self, WindowError> {
        let title = title.into();
        if title.contains('\0') {
            return Err(WindowError::TitleContainsNul);
        }
        let client_width = NonZeroU32::new(client_width).ok_or(WindowError::ZeroClientExtent)?;
        let client_height = NonZeroU32::new(client_height).ok_or(WindowError::ZeroClientExtent)?;
        Ok(Self {
            title,
            client_width,
            client_height,
        })
    }

    /// Window title passed to the platform's creation API.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Fixed client-area width requested during creation.
    pub fn client_width(&self) -> NonZeroU32 {
        self.client_width
    }

    /// Fixed client-area height requested during creation.
    pub fn client_height(&self) -> NonZeroU32 {
        self.client_height
    }
}

/// Failure while creating or operating the minimal native window.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WindowError {
    /// A fixed client extent must have non-zero width and height.
    ZeroClientExtent,
    /// Win32 titles are NUL-terminated and therefore cannot contain an embedded NUL.
    TitleContainsNul,
    /// The requested host primitive has no implementation on this target.
    UnsupportedPlatform,
    /// A Win32 API call failed.
    Platform(String),
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroClientExtent => f.write_str("window client extent must be non-zero"),
            Self::TitleContainsNul => f.write_str("window title must not contain an embedded NUL"),
            Self::UnsupportedPlatform => {
                f.write_str("the minimal Fluxel window is currently supported only on Windows")
            }
            Self::Platform(message) => write!(f, "native window operation failed: {message}"),
        }
    }
}

impl std::error::Error for WindowError {}

#[cfg(windows)]
mod win32;
#[cfg(windows)]
pub use win32::Window;

#[cfg(not(windows))]
mod unsupported;
#[cfg(not(windows))]
pub use unsupported::Window;

#[cfg(test)]
mod tests {
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
}
