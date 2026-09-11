//! Minimal native-host primitives for Fluxel's first visible Windows slice.
//!
//! This crate owns native window creation and its message pump.  Renderers only
//! consume the standard [`raw_window_handle`] traits; surfaces, swapchains,
//! GPU synchronization, input, clocks, and application loops remain outside
//! this deliberately small boundary.

mod window;

pub use window::{Window, WindowConfig, WindowError};
