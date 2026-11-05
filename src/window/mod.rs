//! A minimal native window and ordered lifecycle-event contract.
//!
//! `Window` exposes creation, non-blocking event pumping, ordered close/size
//! observation, explicit destruction, and standard raw handles. It does not
//! establish a general host runtime, DPI policy, clock, or renderer/swapchain
//! ownership. The optional event-driven runtime also forwards input; the
//! minimal platform window may expose only lifecycle events.

use core::{fmt, num::NonZeroU32};

/// Mouse button reported by the event-driven runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MouseButton {
    /// Primary button.
    Left,
    /// Secondary button.
    Right,
    /// Middle button.
    Middle,
    /// Additional platform button.
    Other(u16),
}

/// Logical key reported independently of text entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Key {
    /// One logical character; ASCII letters are normalized to lowercase.
    Character(char),
    Escape,
    Enter,
    Tab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Space,
    Shift,
    Control,
    Alt,
    Super,
    /// A key without a representation in this contract.
    Other,
}

/// One native-window event observed by [`Window::poll_events`].
///
/// Width and height are Win32 client-pixel dimensions. A resize may contain a
/// zero dimension: it is an observable platform transition, not permission to
/// create a zero-sized rendering surface. `Minimized` is separate because
/// Win32 identifies it explicitly through `WM_SIZE`.
#[derive(Clone, Debug, PartialEq)]
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
    /// Cursor position in physical client pixels.
    CursorMoved { x: f64, y: f64 },
    /// The cursor left the client area.
    CursorLeft,
    /// A mouse button changed state.
    MouseInput { button: MouseButton, pressed: bool },
    /// Wheel displacement in lines; pixel deltas are converted at 40 pixels per line.
    MouseWheel { x: f32, y: f32 },
    /// A logical key changed state. Repeat is set for repeated presses.
    KeyboardInput {
        key: Key,
        pressed: bool,
        repeat: bool,
    },
    /// Committed text, including IME input.
    TextInput(String),
    /// Keyboard focus changed. Consumers should clear held input on focus loss.
    Focused(bool),
    /// `WM_CLOSE` requested application shutdown without destroying the HWND.
    CloseRequested,
}

/// Initial configuration for one native window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowConfig {
    title: String,
    client_width: NonZeroU32,
    client_height: NonZeroU32,
    visibility: WindowVisibility,
}

/// Presentation requested when a native window is created.
///
/// A [`Background`] window is shown without taking foreground activation where
/// the native backend can provide that guarantee. [`Hidden`] lets a renderer
/// create its surface and produce an initial frame before it explicitly calls
/// `show_background` on the platform window.
///
/// [`Background`]: Self::Background
/// [`Hidden`]: Self::Hidden
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WindowVisibility {
    /// Keep the newly-created window hidden until explicitly presented.
    Hidden,
    /// Show the newly-created window behind existing work without activation.
    #[default]
    Background,
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
            visibility: WindowVisibility::Background,
        })
    }

    /// Selects the initial presentation policy.
    ///
    /// The default is [`WindowVisibility::Background`]. Use
    /// [`WindowVisibility::Hidden`] when a consumer must finish setup or
    /// render an initial frame before requesting background presentation.
    #[must_use]
    pub fn with_visibility(mut self, visibility: WindowVisibility) -> Self {
        self.visibility = visibility;
        self
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

    /// Requested presentation policy for the newly-created window.
    pub fn visibility(&self) -> WindowVisibility {
        self.visibility
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
#[path = "../tests/window/mod.rs"]
mod tests;
