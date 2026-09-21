//! Event-driven native bootstrap for presentation smoke tests and examples.
//!
//! This is intentionally separate from [`crate::Window`].  The latter is a
//! small owned/adopted raw-handle primitive; Android and iOS require an
//! application event loop whose lifetime cannot be represented safely by a
//! portable `poll_events` call.  The runtime owns that loop but exposes no
//! `winit` type in its public contract and never depends on a renderer.

mod winit;

pub use winit::{HostApplication, HostContext, HostRuntime, HostRuntimeError, HostWindow};

/// Android's activity object, re-exported from the runtime dependency so an
/// APK has one and only one `android-activity` version in its final link.
#[cfg(target_os = "android")]
pub use ::winit::platform::android::activity::AndroidApp;
