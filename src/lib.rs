//! Minimal native-host primitives for Fluxel's first visible Windows slice.
//!
//! This crate owns native window creation and its message pump.  Renderers only
//! consume the standard [`raw_window_handle`] traits; surfaces, swapchains,
//! GPU synchronization, input, clocks, and application loops remain outside
//! this deliberately small boundary.

mod window;

pub use window::{Window, WindowConfig, WindowError, WindowEvent};

#[cfg(feature = "winit-runtime")]
mod runtime;
#[cfg(all(feature = "winit-runtime", target_os = "android"))]
pub use runtime::AndroidApp;
#[cfg(feature = "winit-runtime")]
pub use runtime::{HostApplication, HostContext, HostRuntime, HostRuntimeError, HostWindow};
