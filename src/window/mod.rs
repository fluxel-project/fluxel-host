//! A minimal native window and ordered lifecycle-event contract.
//!
//! `Window` deliberately exposes only creation, non-blocking event pumping,
//! ordered close/size observation, explicit destruction, and standard raw
//! handles. It does not establish a general host runtime, DPI policy, input,
//! clock, or renderer/swapchain ownership.

use core::{fmt, num::NonZeroU32};

/// One native-window event observed by [`Window::poll_events`].
///
/// Width and height are Win32 client-pixel dimensions. A resize may contain a
/// zero dimension: it is an observable platform transition, not permission to
/// create a zero-sized rendering surface. `Minimized` is separate because
/// Win32 identifies it explicitly through `WM_SIZE`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WindowEvent {
    /// A platform supplied a native drawable. The window/display handle is
    /// valid until the matching [`Self::SurfaceDestroyed`] event.
    SurfaceCreated,
    /// The platform revoked its native drawable. Consumers must stop acquiring
    /// frames and release any RHI presentation target before returning from the
    /// platform callback that reports this event.
    SurfaceDestroyed,
    /// The client area changed to this size without a restore transition.
    Resized {
        /// Client-area width in Win32 client pixels; zero is valid.
        width: u32,
        /// Client-area height in Win32 client pixels; zero is valid.
        height: u32,
    },
    /// The native window entered its minimized state.
    Minimized,
    /// The platform suspended foreground rendering without destroying the
    /// drawable. This is a lifecycle fact; a renderer chooses whether to idle
    /// or retain its device.
    Suspended,
    /// The platform resumed foreground rendering after [`Self::Suspended`].
    Resumed,
    /// The platform requested that a frame be produced for the current
    /// drawable, without implying a resize.
    RedrawRequested,
    /// The native window was restored with this client-area size.
    Restored {
        /// Client-area width in Win32 client pixels; zero is valid.
        width: u32,
        /// Client-area height in Win32 client pixels; zero is valid.
        height: u32,
    },
    /// `WM_CLOSE` requested application shutdown without destroying the HWND.
    CloseRequested,
}

/// Initial configuration for one native window.
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

    /// Initial client-area width requested during creation.
    pub fn client_width(&self) -> NonZeroU32 {
        self.client_width
    }

    /// Initial client-area height requested during creation.
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
            Self::UnsupportedPlatform => f.write_str(
                "the requested minimal Fluxel window operation is not supported on this platform",
            ),
            Self::Platform(message) => write!(f, "native window operation failed: {message}"),
        }
    }
}

impl std::error::Error for WindowError {}

#[cfg(windows)]
mod win32;
#[cfg(windows)]
pub use win32::Window;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
pub use android::Window;

#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "ios")]
pub use ios::Window;

#[cfg(not(any(windows, target_os = "android", target_os = "ios")))]
mod unsupported;
#[cfg(not(any(windows, target_os = "android", target_os = "ios")))]
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
